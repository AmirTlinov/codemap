"""Evaluate frozen calibration/holdout receipts against the S15 flagship contract."""

from __future__ import annotations

import hashlib
import json
import statistics
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from flagship_artifacts import artifact_inventory
from flagship_judging import load_judgments, read_jsonl
from flagship_manifest import file_sha256, load_frozen
from flagship_receipts import trial_receipt_errors
from flagship_stats import criterion_score, median, paired_repo_bootstrap, repo_macro, task_aggregate

ARMS = ("control", "codemap")
NONINFERIOR = ("downstream", "contract", "regression")

def _task_meta(task: dict[str, Any]) -> dict[str, Any]:
    return task["benchmark"]

def _split_tasks(tasks: list[dict[str, Any]], split: str) -> list[dict[str, Any]]:
    return [task for task in tasks if _task_meta(task)["split"] == split]


def _input_usage(row: dict[str, Any]) -> int:
    usage = row["codex"]["usage"]
    return int(usage.get("input_tokens", 0))


def _result_identity(row: dict[str, Any]) -> tuple[str, int, str]:
    return row.get("task_id"), row.get("repetition"), row.get("arm")


def _provenance_errors(
    row: dict[str, Any],
    task: dict[str, Any],
    manifest: dict[str, Any],
    expected_order: int,
) -> list[str]:
    meta = _task_meta(task)
    expected_commit = manifest["repositories"][f"{meta['repo_id']}:{meta.get('repo_variant', 'default')}"]["commit"]
    expected_binary = manifest["codemap_identity"]["build_identity"]["binary_sha256"]
    checks = {
        "mode": row.get("mode") == task.get("mode", "implementation"),
        "prompt": row.get("task_prompt_sha256")
        == hashlib.sha256(task["prompt"].strip().encode()).hexdigest(),
        "repo": Path(str(row.get("repo", ""))).resolve() == Path(task["repo"]).resolve(),
        "commit": row.get("base_commit") == expected_commit,
        "model": row.get("model") == manifest["model"],
        "reasoning": row.get("reasoning_effort") == manifest["reasoning_effort"],
        "codex": row.get("codex_version") == manifest["codex_version"],
        "codex_binary": row.get("codex_artifacts") == manifest["codex_artifacts"],
        "binary": row.get("report_prelude", {})
        .get("codemap", {})
        .get("build_identity", {})
        .get("binary_sha256")
        == expected_binary,
        "order": row.get("order") == expected_order,
    }
    return [name for name, passed in checks.items() if not passed]


def _protocol_errors(row: dict[str, Any], task: dict[str, Any]) -> list[str]:
    protocol = row.get("codemap_protocol", {})
    if row["arm"] == "control":
        return [] if protocol.get("invocation_count") == 0 else ["control_codemap_access"]
    errors = [] if protocol.get("compliant") is True else ["treatment_protocol"]
    meta = _task_meta(task)
    if meta["task_class"] == "negative_control":
        if protocol.get("entry_kind") != "exact" or protocol.get("root_entry") is not False:
            errors.append("negative_not_exact")
        if protocol.get("first_entry") not in meta["allowed_exact_entries"]:
            errors.append("negative_unregistered_entry")
    return errors


def _trial_criteria(
    row: dict[str, Any],
    task: dict[str, Any],
    ordinal: dict[tuple[str, int, str, str], float],
) -> list[dict[str, Any]]:
    declared = {verifier["name"]: verifier for verifier in task["verify"]}
    observed = {verifier["name"]: verifier for verifier in row.get("verifiers", [])}
    if set(declared) != set(observed):
        raise ValueError(f"{task['id']}: verifier identities changed")
    criteria = []
    for name, verifier in declared.items():
        result = observed[name]
        for field in ("category", "weight", "required"):
            if result.get(field) != verifier.get(field):
                raise ValueError(f"{task['id']}: verifier {name} changed {field}")
        criteria.append(
            {
                "id": name,
                "category": verifier["category"],
                "weight": verifier["weight"],
                "required": verifier.get("required", True),
                "deterministic": True,
                "value": 1.0 if result.get("passed") else 0.0,
            }
        )
    for criterion in _task_meta(task).get("ordinal_criteria", []):
        key = (task["id"], row["repetition"], row["arm"], criterion["id"])
        if key not in ordinal:
            raise ValueError(f"missing blind judgment: {key}")
        criteria.append(
            {
                **criterion,
                "required": False,
                "deterministic": False,
                "value": ordinal[key],
            }
        )
    return criteria


def _pair_row(
    task: dict[str, Any],
    repetition: int,
    pair: dict[str, dict[str, Any]],
    ordinal: dict[tuple[str, int, str, str], float],
) -> dict[str, Any]:
    scores: dict[str, float] = {}
    categories: dict[str, dict[str, float]] = {}
    required: dict[str, bool] = {}
    criteria_by_arm: dict[str, list[dict[str, Any]]] = {}
    for arm in ARMS:
        criteria = _trial_criteria(pair[arm], task, ordinal)
        score, category = criterion_score(criteria)
        scores[arm], categories[arm], criteria_by_arm[arm] = score, category, criteria
        required[arm] = all(
            row["value"] == 1.0 for row in criteria if row["deterministic"] and row["required"]
        ) and pair[arm].get("outcome_passed") is True
    category_names = set(categories["control"]) | set(categories["codemap"])
    control_criteria = {row["id"]: row["value"] for row in criteria_by_arm["control"]}
    codemap_criteria = {row["id"]: row["value"] for row in criteria_by_arm["codemap"]}
    regressions = [
        row["id"]
        for row in criteria_by_arm["control"]
        if row["deterministic"]
        and row["required"]
        and row["value"] == 1.0
        and next(item for item in criteria_by_arm["codemap"] if item["id"] == row["id"])["value"]
        < 1.0
    ]
    return {
        "task_id": task["id"],
        "repo_id": _task_meta(task)["repo_id"],
        "task_class": _task_meta(task)["task_class"],
        "repetition": repetition,
        "control_score": scores["control"],
        "codemap_score": scores["codemap"],
        "category_deltas": {
            name: categories["codemap"].get(name, 0.0) - categories["control"].get(name, 0.0)
            for name in category_names
        },
        "criterion_deltas": {
            name: codemap_criteria[name] - control_criteria[name] for name in control_criteria
        },
        "control_outcome": required["control"],
        "codemap_outcome": required["codemap"],
        "required_regressions": regressions,
        "control_elapsed": pair["control"]["codex"]["elapsed_ms"],
        "codemap_elapsed": pair["codemap"]["codex"]["elapsed_ms"],
        "control_input": _input_usage(pair["control"]),
        "codemap_input": _input_usage(pair["codemap"]),
    }


def evaluate_split(
    split: str,
    run_dir: Path,
    tasks: list[dict[str, Any]],
    manifest: dict[str, Any],
    ordinal: dict[tuple[str, int, str, str], float],
) -> dict[str, Any]:
    selected = _split_tasks(tasks, split)
    expected_hash = manifest[f"{split}_tasks_sha256"]
    input_path = run_dir / "input-tasks.jsonl"
    if not input_path.is_file() or file_sha256(input_path) != expected_hash:
        raise ValueError(f"{split}: run task manifest differs from frozen bytes")
    summary = json.loads((run_dir / "summary.json").read_text())
    preflight = {row["task_id"]: row for row in summary.get("preflight", [])}
    rows = read_jsonl(run_dir / "results.jsonl")
    indexed: dict[tuple[str, int, str], dict[str, Any]] = {}
    duplicates = []
    for row in rows:
        key = _result_identity(row)
        if key in indexed:
            duplicates.append(key)
        indexed[key] = row
    schedule = {
        (row["task_id"], row["repetition"]): row["arms"]
        for row in manifest["pair_schedule"][split]
    }
    invalid: list[dict[str, Any]] = []
    pair_rows = []
    for task in selected:
        task_id = task["id"]
        if preflight.get(task_id, {}).get("baseline_passed") is not False:
            invalid.append({"task_id": task_id, "reason": "preflight_no_gap"})
        for repetition in range(1, manifest["repetitions"] + 1):
            pair: dict[str, dict[str, Any]] = {}
            for arm in ARMS:
                key = (task_id, repetition, arm)
                row = indexed.get(key)
                if row is None:
                    invalid.append({"key": key, "reason": "missing_arm"})
                    continue
                expected_order = schedule[(task_id, repetition)].index(arm) + 1
                errors = _provenance_errors(row, task, manifest, expected_order)
                errors += _protocol_errors(row, task)
                errors += trial_receipt_errors(row)
                if row.get("run_valid") is not True:
                    errors.append("invalid_run")
                    if row.get("invalidation_reason") not in manifest["allowed_exclusions"]:
                        errors.append("unregistered_invalidation_reason")
                if task.get("mode") == "analysis" and row.get("analysis_no_repo_changes") is not True:
                    errors.append("analysis_repo_write")
                if errors:
                    invalid.append({"key": key, "reason": "provenance_mismatch", "details": errors})
                else:
                    pair[arm] = row
            if set(pair) == set(ARMS):
                pair_rows.append(_pair_row(task, repetition, pair, ordinal))
    expected = len(selected) * manifest["repetitions"] * 2
    actual_keys = set(indexed)
    expected_keys = {
        (task["id"], repetition, arm)
        for task in selected
        for repetition in range(1, manifest["repetitions"] + 1)
        for arm in ARMS
    }
    invalid.extend(
        {"key": key, "reason": "provenance_mismatch", "details": ["unexpected_trial"]}
        for key in sorted(actual_keys - expected_keys)
    )
    invalid.extend(
        {"key": key, "reason": "provenance_mismatch", "details": ["duplicate_trial"]}
        for key in duplicates
    )
    by_task: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in pair_rows:
        by_task[row["task_id"]].append(row)
    task_rows = [task_aggregate(by_task[task["id"]]) for task in selected if by_task[task["id"]]]
    return {
        "split": split,
        "expected_trials": expected,
        "observed_trials": len(rows),
        "valid_pairs": len(pair_rows),
        "expected_pairs": expected // 2,
        "invalid": invalid,
        "pairs": pair_rows,
        "tasks": task_rows,
    }


def _category_macro(tasks: list[dict[str, Any]], category: str) -> float:
    rows = [{**row, "delta": row["category_deltas"].get(category, 0.0)} for row in tasks]
    return repo_macro(rows)


def acceptance_checks(holdout: dict[str, Any], manifest: dict[str, Any]) -> dict[str, Any]:
    thresholds = manifest["acceptance"]
    task_rows = holdout["tasks"]
    complex_tasks = [row for row in task_rows if row["task_class"] != "negative_control"]
    negatives = [row for row in task_rows if row["task_class"] == "negative_control"]
    bootstrap = paired_repo_bootstrap(
        complex_tasks,
        manifest["bootstrap_iterations"],
        manifest["bootstrap_seed"],
        thresholds["primary_alpha"],
    )
    primary = bootstrap["lower_bound"] > 0
    deltas = [row["delta"] for row in complex_tasks]
    wins = sum(delta > 0 for delta in deltas)
    required_regressions = [
        {"task_id": row["task_id"], "repetition": row["repetition"], "criteria": row["required_regressions"]}
        for row in holdout["pairs"]
        if row["required_regressions"]
    ]
    noninferior = {category: _category_macro(complex_tasks, category) for category in NONINFERIOR}
    complex_time = median([row["time_overhead"] for row in complex_tasks if row["time_overhead"] is not None])
    complex_input = median([row["input_overhead"] for row in complex_tasks if row["input_overhead"] is not None])
    negative_same = all(row["control_outcomes"] == row["codemap_outcomes"] for row in negatives)
    negative_time = median([row["time_overhead"] for row in negatives if row["time_overhead"] is not None])
    negative_input = median([row["input_overhead"] for row in negatives if row["input_overhead"] is not None])
    negative_budget = (
        negative_time is not None
        and negative_input is not None
        and negative_time <= thresholds["max_negative_overhead"]
        and negative_input <= thresholds["max_negative_overhead"]
    )
    over_budget = [
        row
        for row in complex_tasks
        if (row["time_overhead"] or 0) > thresholds["max_complex_time_overhead"]
        or (row["input_overhead"] or 0) > thresholds["max_complex_input_overhead"]
    ]
    task_meta = {row["id"]: _task_meta(row) for row in manifest["_tasks"]}
    exception_wins = []
    for row in over_budget:
        criteria = task_meta[row["task_id"]].get("exception_criteria", [])
        exception_wins.append(bool(criteria) and any(row["criterion_deltas"].get(name, 0) > 0 for name in criteria))
    exception = (
        primary
        and negative_same
        and negative_budget
        and over_budget
        and sum(exception_wins) / len(over_budget) >= 0.60
    )
    resource = (
        complex_time is not None
        and complex_input is not None
        and (complex_time <= thresholds["max_complex_time_overhead"] or exception)
        and (complex_input <= thresholds["max_complex_input_overhead"] or exception)
    )
    checks = {
        "complete_valid_denominator": not holdout["invalid"]
        and holdout["valid_pairs"] == holdout["expected_pairs"],
        "primary_bootstrap_lower_bound_above_zero": primary,
        "no_required_regression": not required_regressions,
        "positive_median_task_delta": bool(deltas) and statistics.median(deltas) > 0,
        "task_win_rate": bool(deltas) and wins / len(deltas) >= thresholds["min_task_win_rate"],
        "noninferior_categories": all(value >= 0 for value in noninferior.values()),
        "complex_resource_boundary_or_exception": resource,
        "negative_same_outcome": negative_same,
        "negative_resource_boundary": negative_budget,
        "analysis_zero_writes": not any(
            "analysis_repo_write" in row.get("details", []) for row in holdout["invalid"]
        ),
    }
    return {
        "accepted": all(checks.values()),
        "checks": checks,
        "bootstrap": bootstrap,
        "median_task_delta": median(deltas),
        "task_wins": wins,
        "task_win_rate": wins / len(deltas) if deltas else None,
        "category_repo_macro_deltas": noninferior,
        "required_regressions": required_regressions,
        "resources": {
            "complex_median_time_overhead": complex_time,
            "complex_median_input_overhead": complex_input,
            "negative_median_time_overhead": negative_time,
            "negative_median_input_overhead": negative_input,
            "over_budget_tasks": [row["task_id"] for row in over_budget],
            "completeness_exception": bool(exception),
        },
    }


def evaluate(
    manifest_path: Path,
    calibration_dir: Path,
    holdout_dir: Path,
    assignments: Path,
    key: Path,
    ratings: Path,
    out_dir: Path,
) -> Path:
    manifest, tasks = load_frozen(manifest_path)
    ordinal, agreement = load_judgments(
        tasks, assignments, key, ratings, manifest["acceptance"]["min_agreement_alpha"]
    )
    manifest["_tasks"] = tasks
    calibration = evaluate_split("calibration", calibration_dir, tasks, manifest, ordinal)
    holdout = evaluate_split("holdout", holdout_dir, tasks, manifest, ordinal)
    acceptance = acceptance_checks(holdout, manifest)
    agreement_valid = all(row["valid"] for row in agreement.values())
    acceptance["checks"]["ordinal_agreement"] = agreement_valid
    acceptance["accepted"] = acceptance["accepted"] and agreement_valid
    out_dir.mkdir(parents=True, exist_ok=False)
    evidence_inputs = [
        manifest_path,
        assignments,
        key,
        ratings,
    ]
    report = {
        "kind": "codemap_flagship_acceptance",
        "version": 1,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "manifest": str(manifest_path.resolve()),
        "manifest_sha256": file_sha256(manifest_path),
        "evidence": artifact_inventory(
            [calibration_dir, holdout_dir, assignments.parent], evidence_inputs
        ),
        "agreement": agreement,
        "calibration": calibration,
        "holdout": holdout,
        "acceptance": acceptance,
    }
    output = out_dir / "acceptance.json"
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    return output
