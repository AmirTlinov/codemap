"""Evaluate one frozen 72-run A/B against the outcome-based flagship contract."""

from __future__ import annotations

import hashlib
import json
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from flagship_artifacts import artifact_inventory
from flagship_manifest import file_sha256, load_frozen, read_jsonl, task_meta
from flagship_receipts import trial_receipt_errors
from flagship_stats import criterion_score, median, task_aggregate


ARMS = ("control", "codemap")
INFRASTRUCTURE_FAILURES = {"codex_crash", "codex_timeout", "verifier_timeout"}


def _input_usage(row: dict[str, Any]) -> int:
    usage = row.get("codex", {}).get("usage", {})
    return int(usage.get("input_tokens", 0))


def _result_identity(row: dict[str, Any]) -> tuple[str, int, str]:
    return row.get("task_id"), row.get("repetition"), row.get("arm")


def _provenance_errors(
    row: dict[str, Any],
    task: dict[str, Any],
    manifest: dict[str, Any],
    expected_order: int,
) -> list[str]:
    meta = task_meta(task)
    repo_key = f"{meta['repo_id']}:{meta.get('repo_variant', 'default')}"
    expected_binary = manifest["codemap_identity"]["build_identity"]["binary_sha256"]
    checks = {
        "mode": row.get("mode") == task.get("mode", "implementation"),
        "prompt": row.get("task_prompt_sha256")
        == hashlib.sha256(task["prompt"].strip().encode()).hexdigest(),
        "repo": Path(str(row.get("repo", ""))).resolve() == Path(task["repo"]).resolve(),
        "commit": row.get("base_commit") == manifest["repositories"][repo_key]["commit"],
        "model": row.get("model") == manifest["model"],
        "reasoning": row.get("reasoning_effort") == manifest["reasoning_effort"],
        "codex_version": row.get("codex_version") == manifest["codex"]["version"],
        "codex_binary": row.get("codex_artifacts") == manifest["codex"]["artifacts"],
        "codemap_binary": row.get("report_prelude", {})
        .get("codemap", {})
        .get("build_identity", {})
        .get("binary_sha256")
        == expected_binary,
        "order": row.get("order") == expected_order,
    }
    return [name for name, passed in checks.items() if not passed]

def _assignment_errors(row: dict[str, Any]) -> list[str]:
    activity = row.get("codemap_activity", {})
    if row.get("arm") != "control":
        return []
    return [] if activity.get("invocation_count") == 0 else ["control_codemap_access"]


def _run_errors(row: dict[str, Any]) -> list[str]:
    attempt = row.get("infrastructure_attempt")
    prior = row.get("prior_attempts")
    expected_prior = [] if attempt == 1 else ["attempts/attempt-1/result.json"]
    if attempt not in (1, 2) or prior != expected_prior:
        return ["invalid_infrastructure_attempt_history"]
    if attempt == 2:
        first_path = Path(row["codex"]["last_message_artifact"]).parent / prior[0]
        if not first_path.is_file():
            return ["missing_first_infrastructure_attempt"]
        first = json.loads(first_path.read_text(encoding="utf-8"))
        valid_first = first.get("infrastructure_attempt") == 1 and first.get("invalidation_reason") in INFRASTRUCTURE_FAILURES
        if not valid_first or first.get("trial_fingerprint") != row.get("trial_fingerprint"):
            return ["invalid_first_infrastructure_attempt"]
    if row.get("run_valid") is True:
        return []
    reason = row.get("invalidation_reason")
    if reason in INFRASTRUCTURE_FAILURES and attempt == 2:
        return ["repeated_infrastructure_failure"]
    return ["invalid_run"]


def _trial_criteria(row: dict[str, Any], task: dict[str, Any]) -> list[dict[str, Any]]:
    declared = {verifier["name"]: verifier for verifier in task["verify"]}
    observed = {verifier["name"]: verifier for verifier in row.get("verifiers", [])}
    if set(declared) != set(observed):
        raise ValueError(f"{task['id']}: verifier identities changed")
    criteria = []
    for name, verifier in declared.items():
        result = observed[name]
        for field in ("category", "weight", "required"):
            if result.get(field) != verifier.get(field, True if field == "required" else None):
                raise ValueError(f"{task['id']}: verifier {name} changed {field}")
        criteria.append(
            {
                "id": name,
                "category": verifier["category"],
                "weight": verifier["weight"],
                "required": verifier.get("required", True),
                "value": 1.0 if result.get("passed") else 0.0,
            }
        )
    return criteria


def _pair_row(
    task: dict[str, Any], repetition: int, pair: dict[str, dict[str, Any]]
) -> dict[str, Any]:
    criteria = {arm: _trial_criteria(pair[arm], task) for arm in ARMS}
    by_arm = {
        arm: {criterion["id"]: criterion for criterion in criteria[arm]} for arm in ARMS
    }
    required_criteria = {
        name: {
            "control": control["value"],
            "codemap": by_arm["codemap"][name]["value"],
        }
        for name, control in by_arm["control"].items()
        if control["required"]
    }
    return {
        "task_id": task["id"],
        "repo_id": task_meta(task)["repo_id"],
        "task_class": task_meta(task)["task_class"],
        "repetition": repetition,
        "control_score": criterion_score(criteria["control"]),
        "codemap_score": criterion_score(criteria["codemap"]),
        "control_outcome": pair["control"].get("outcome_passed") is True,
        "codemap_outcome": pair["codemap"].get("outcome_passed") is True,
        "required_criteria": required_criteria,
        "control_elapsed": pair["control"]["codex"]["elapsed_ms"],
        "codemap_elapsed": pair["codemap"]["codex"]["elapsed_ms"],
        "control_input": _input_usage(pair["control"]),
        "codemap_input": _input_usage(pair["codemap"]),
    }


def evaluate_run(
    run_dir: Path, tasks: list[dict[str, Any]], manifest: dict[str, Any]
) -> dict[str, Any]:
    input_path = run_dir / "input-tasks.jsonl"
    if not input_path.is_file() or file_sha256(input_path) != manifest["tasks_sha256"]:
        raise ValueError("run task bytes differ from frozen tasks")
    summary = json.loads((run_dir / "summary.json").read_text(encoding="utf-8"))
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
        for row in manifest["pair_schedule"]
    }
    invalid: list[dict[str, Any]] = []
    pair_rows = []
    zero_write_violations = []
    for task in tasks:
        task_id = task["id"]
        if preflight.get(task_id, {}).get("baseline_passed") is not False:
            invalid.append({"task_id": task_id, "reason": "preflight_no_gap"})
        for repetition in (1, 2):
            pair: dict[str, dict[str, Any]] = {}
            for arm in ARMS:
                key = (task_id, repetition, arm)
                row = indexed.get(key)
                if row is None:
                    invalid.append({"key": key, "reason": "missing_arm"})
                    continue
                order = schedule[(task_id, repetition)].index(arm) + 1
                errors = _provenance_errors(row, task, manifest, order)
                errors += _assignment_errors(row)
                errors += _run_errors(row)
                errors += trial_receipt_errors(row)
                if task["mode"] == "analysis" and row.get("analysis_no_repo_changes") is not True:
                    errors.append("analysis_repo_write")
                    zero_write_violations.append(key)
                if errors:
                    invalid.append({"key": key, "reason": "invalid_trial", "details": errors})
                else:
                    pair[arm] = row
            if set(pair) == set(ARMS):
                pair_rows.append(_pair_row(task, repetition, pair))
    expected_keys = {
        (task["id"], repetition, arm)
        for task in tasks
        for repetition in (1, 2)
        for arm in ARMS
    }
    invalid.extend(
        {"key": key, "reason": "unexpected_trial"} for key in sorted(set(indexed) - expected_keys)
    )
    invalid.extend({"key": key, "reason": "duplicate_trial"} for key in duplicates)
    by_task: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in pair_rows:
        by_task[row["task_id"]].append(row)
    task_rows = []
    for task in tasks:
        if len(by_task[task["id"]]) == 2:
            task_rows.append(task_aggregate(by_task[task["id"]]))
    return {
        "expected_trials": 72,
        "observed_trials": len(rows),
        "expected_pairs": 36,
        "valid_pairs": len(pair_rows),
        "invalid": invalid,
        "zero_write_violations": zero_write_violations,
        "pairs": pair_rows,
        "tasks": task_rows,
    }


def _trajectory_evidence(
    path: Path | None, tasks: list[dict[str, Any]], manifest_path: Path
) -> tuple[dict[str, Any] | None, list[str]]:
    if path is None or not path.is_file():
        return None, ["missing_trajectory_analysis"]
    summary = json.loads(path.read_text(encoding="utf-8"))
    errors = []
    if summary.get("kind") != "codemap_flagship_trajectory_analysis" or summary.get("version") != 1:
        errors.append("unsupported_trajectory_analysis")
    if summary.get("manifest_sha256") != file_sha256(manifest_path):
        errors.append("trajectory_manifest_mismatch")
    expected = {(task["id"], repetition) for task in tasks for repetition in (1, 2)}
    observed = set()
    for row in summary.get("pairs", []):
        key = (row.get("task_id"), row.get("repetition"))
        if key in observed:
            errors.append(f"duplicate_trajectory:{key}")
        observed.add(key)
        report = Path(str(row.get("report", "")))
        context = report.parent / "pair-context.md"
        if row.get("complete") is not True or row.get("status") != 0 or row.get("timed_out") is not False:
            errors.append(f"incomplete_trajectory:{key}")
        if not report.is_file() or not report.read_text(encoding="utf-8", errors="replace").strip():
            errors.append(f"missing_trajectory_report:{key}")
        elif file_sha256(report) != row.get("report_sha256"):
            errors.append(f"trajectory_report_hash:{key}")
        if not context.is_file() or file_sha256(context) != row.get("context_sha256"):
            errors.append(f"trajectory_context_hash:{key}")
        if set(row.get("labels", {}).values()) != set(ARMS):
            errors.append(f"trajectory_arm_labels:{key}")
    if observed != expected:
        errors.append("trajectory_pair_denominator")
    if summary.get("complete") is not (not errors):
        errors.append("trajectory_summary_state")
    return summary, errors


def acceptance_checks(
    run: dict[str, Any], manifest: dict[str, Any], trajectory_errors: list[str]
) -> dict[str, Any]:
    thresholds = manifest["acceptance"]
    complex_tasks = [row for row in run["tasks"] if row["task_class"] != "exact_control"]
    exact_tasks = [row for row in run["tasks"] if row["task_class"] == "exact_control"]
    wins = [row["task_id"] for row in complex_tasks if row["delta"] > 0]
    losses = [row["task_id"] for row in complex_tasks if row["delta"] < 0]
    required_losses = [
        {
            "task_id": row["task_id"],
            "criteria": row["required_criterion_losses"],
        }
        for row in run["tasks"]
        if row["required_criterion_losses"]
    ]
    exact_regressions = [
        row["task_id"]
        for row in exact_tasks
        if sum(row["control_outcomes"]) != sum(row["codemap_outcomes"])
    ]
    complex_time = median([row["time_overhead"] for row in complex_tasks if row["time_overhead"] is not None])
    complex_input = median([row["input_overhead"] for row in complex_tasks if row["input_overhead"] is not None])
    exact_time = median([row["time_overhead"] for row in exact_tasks if row["time_overhead"] is not None])
    exact_input = median([row["input_overhead"] for row in exact_tasks if row["input_overhead"] is not None])
    validity = {
        "complete_72_run_denominator": not run["invalid"]
        and run["observed_trials"] == 72
        and run["valid_pairs"] == 36
        and len(run["tasks"]) == 18,
        "zero_repo_writes_for_read_only_tasks": not run["zero_write_violations"],
        "paired_trajectory_analysis": not trajectory_errors,
    }
    criteria = {
        "complex_effectiveness": len(complex_tasks) == 12
        and len(wins) >= thresholds["min_complex_wins"]
        and not losses,
        "regression_safety": not required_losses
        and len(exact_tasks) == 6
        and not exact_regressions,
        "bounded_cost": complex_time is not None
        and complex_input is not None
        and exact_time is not None
        and exact_input is not None
        and complex_time <= thresholds["max_complex_time_overhead"]
        and complex_input <= thresholds["max_complex_input_overhead"]
        and exact_time <= thresholds["max_exact_overhead"]
        and exact_input <= thresholds["max_exact_overhead"],
    }
    return {
        "accepted": all(validity.values()) and all(criteria.values()),
        "validity": validity,
        "criteria": criteria,
        "complex": {"wins": len(wins), "winning_tasks": wins, "losing_tasks": losses},
        "required_criterion_losses": required_losses,
        "exact_regressions": exact_regressions,
        "resources": {
            "complex_median_time_overhead": complex_time,
            "complex_median_input_overhead": complex_input,
            "exact_median_time_overhead": exact_time,
            "exact_median_input_overhead": exact_input,
        },
    }


def _markdown(report: dict[str, Any]) -> str:
    acceptance = report["acceptance"]
    resources = acceptance["resources"]
    state = "PASSED" if acceptance["accepted"] else "FAILED"
    return "\n".join(
        [
            "# codemap flagship A/B",
            "",
            f"**{state}** — deterministic external verification over 6 repositories, "
            "4+ ecosystems, 18 tasks, 2 counterbalanced repetitions, and 72 agent runs.",
            "",
            f"- Complex wins: **{acceptance['complex']['wins']}/12**; losses: "
            f"**{len(acceptance['complex']['losing_tasks'])}**.",
            f"- Required criterion losses: **{len(acceptance['required_criterion_losses'])}**; "
            f"exact regressions: **{len(acceptance['exact_regressions'])}**.",
            f"- Causal trajectory reports: **{report['trajectory_analysis']['pairs']}/36**.",
            f"- Complex overhead: time **{resources['complex_median_time_overhead']:.1%}**, "
            f"input **{resources['complex_median_input_overhead']:.1%}**.",
            f"- Exact overhead: time **{resources['exact_median_time_overhead']:.1%}**, "
            f"input **{resources['exact_median_input_overhead']:.1%}**.",
            "",
            "This result is scoped to the frozen corpus. It does not claim that every project or task improves.",
            "",
        ]
    )


def evaluate(
    manifest_path: Path,
    run_dir: Path,
    out_dir: Path,
    trajectory_summary_path: Path | None = None,
) -> Path:
    manifest, tasks = load_frozen(manifest_path)
    run = evaluate_run(run_dir.resolve(), tasks, manifest)
    trajectory, trajectory_errors = _trajectory_evidence(
        trajectory_summary_path, tasks, manifest_path.resolve()
    )
    acceptance = acceptance_checks(run, manifest, trajectory_errors)
    out_dir.mkdir(parents=True, exist_ok=True)
    if (out_dir / "acceptance.json").exists():
        raise ValueError(f"acceptance output already exists: {out_dir}")
    evidence_roots = [run_dir.resolve()]
    if trajectory_summary_path is not None:
        evidence_roots.append(trajectory_summary_path.resolve().parent)
    report = {
        "kind": "codemap_flagship_acceptance",
        "version": 1,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "manifest": str(manifest_path.resolve()),
        "manifest_sha256": file_sha256(manifest_path),
        "evidence": artifact_inventory(
            evidence_roots,
            [manifest_path.resolve(), manifest_path.resolve().parent / manifest["tasks_file"]],
        ),
        "run": run,
        "trajectory_analysis": {
            "summary": str(trajectory_summary_path.resolve()) if trajectory_summary_path else None,
            "summary_sha256": (
                file_sha256(trajectory_summary_path) if trajectory_summary_path else None
            ),
            "pairs": len(trajectory.get("pairs", [])) if trajectory else 0,
            "errors": trajectory_errors,
        },
        "acceptance": acceptance,
    }
    output = out_dir / "acceptance.json"
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    (out_dir / "acceptance.md").write_text(_markdown(report), encoding="utf-8")
    return output
