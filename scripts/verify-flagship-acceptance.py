#!/usr/bin/env python3
"""Independently verify a stable-effect flagship acceptance receipt."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def stable_direction(treatment: list[Any], control: list[Any], minimum: int) -> str:
    wins = sum(left > right for left, right in zip(treatment, control, strict=True))
    losses = sum(left < right for left, right in zip(treatment, control, strict=True))
    return "win" if wins >= minimum else "loss" if losses >= minimum else "neutral"


def task_effects(run: dict[str, Any], minimum: int) -> tuple[list[dict[str, Any]], list[str]]:
    errors = []
    pairs: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in run.get("pairs", []):
        pairs[row.get("task_id")].append(row)
    for task in run.get("tasks", []):
        rows = sorted(pairs[task["task_id"]], key=lambda row: row["repetition"])
        if len(rows) != 4:
            errors.append(f"task pair denominator mismatch: {task['task_id']}")
            task.update(
                _verified_required_losses=[],
                _verified_direction="neutral",
                _verified_outcome_direction="neutral",
                _verified_delta=math.nan,
            )
            continue
        direction = stable_direction(
            [row["codemap_score"] for row in rows],
            [row["control_score"] for row in rows],
            minimum,
        )
        outcome_direction = stable_direction(
            [row["codemap_outcome"] for row in rows],
            [row["control_outcome"] for row in rows],
            minimum,
        )
        delta = sum(row["codemap_score"] for row in rows) / 4 - sum(
            row["control_score"] for row in rows
        ) / 4
        required_ids = set(rows[0]["required_criteria"])
        required = {
            criterion: stable_direction(
                [row["required_criteria"][criterion]["codemap"] for row in rows],
                [row["required_criteria"][criterion]["control"] for row in rows],
                minimum,
            )
            for criterion in sorted(required_ids)
        }
        if task.get("direction") != direction or task.get("outcome_direction") != outcome_direction:
            errors.append(f"task direction mismatch: {task['task_id']}")
        if not math.isclose(task.get("delta", math.inf), delta, abs_tol=1e-12):
            errors.append(f"task mean delta mismatch: {task['task_id']}")
        if task.get("required_criterion_directions") != required:
            errors.append(f"required direction mismatch: {task['task_id']}")
        task["_verified_required_losses"] = [
            criterion for criterion, value in required.items() if value == "loss"
        ]
        task["_verified_direction"] = direction
        task["_verified_outcome_direction"] = outcome_direction
        task["_verified_delta"] = delta
    return run.get("tasks", []), errors


def verify(path: Path) -> list[str]:
    report = json.loads(path.read_text(encoding="utf-8"))
    errors = []
    if report.get("kind") != "codemap_flagship_acceptance" or report.get("version") != 1:
        errors.append("unsupported acceptance receipt")
    manifest_path = Path(report.get("manifest", ""))
    if not manifest_path.is_file() or sha256(manifest_path) != report.get("manifest_sha256"):
        errors.append("manifest hash mismatch")
        return errors
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    repetitions = manifest.get("limits", {}).get("repetitions")
    minimum = manifest.get("acceptance", {}).get("min_direction_repetitions")
    if repetitions != 4 or minimum != 3:
        errors.append("unsupported repetition contract")
        return errors
    for artifact in report.get("evidence", []):
        candidate = Path(artifact.get("path", ""))
        if not candidate.is_file() or sha256(candidate) != artifact.get("sha256"):
            errors.append(f"evidence hash mismatch: {candidate}")
    run = report.get("run", {})
    acceptance = report.get("acceptance", {})
    validity = acceptance.get("validity", {})
    criteria = acceptance.get("criteria", {})
    complete = (
        not run.get("invalid")
        and run.get("expected_trials") == 144
        and run.get("observed_trials") == 144
        and run.get("expected_pairs") == 72
        and run.get("valid_pairs") == 72
        and len(run.get("tasks", [])) == 18
    )
    if validity.get("complete_144_run_denominator") is not complete:
        errors.append("144-run denominator state mismatch")
    zero_write = not run.get("zero_write_violations")
    if validity.get("zero_repo_writes_for_read_only_tasks") is not zero_write:
        errors.append("zero-write state mismatch")
    tasks, effect_errors = task_effects(run, minimum)
    errors.extend(effect_errors)
    complex_tasks = [row for row in tasks if row.get("task_class") != "exact_control"]
    exact_tasks = [row for row in tasks if row.get("task_class") == "exact_control"]
    wins = [row["task_id"] for row in complex_tasks if row.get("_verified_direction") == "win"]
    losses = [row["task_id"] for row in complex_tasks if row.get("_verified_direction") == "loss"]
    required_losses = [
        {"task_id": row["task_id"], "criteria": row["_verified_required_losses"]}
        for row in tasks
        if row.get("_verified_required_losses")
    ]
    exact_regressions = [
        row["task_id"]
        for row in exact_tasks
        if row.get("_verified_outcome_direction") == "loss"
    ]
    complex_result = acceptance.get("complex", {})
    if (
        complex_result.get("wins") != len(wins)
        or complex_result.get("winning_tasks") != wins
        or complex_result.get("losing_tasks") != losses
    ):
        errors.append("complex effect summary mismatch")
    mean_delta = sum(row["_verified_delta"] for row in complex_tasks) / len(complex_tasks)
    if not math.isclose(
        complex_result.get("mean_completeness_delta", math.inf), mean_delta, abs_tol=1e-12
    ):
        errors.append("mean completeness delta mismatch")
    if acceptance.get("required_criterion_losses") != required_losses:
        errors.append("required loss summary mismatch")
    if acceptance.get("exact_regressions") != exact_regressions:
        errors.append("exact regression summary mismatch")
    thresholds = manifest["acceptance"]
    effectiveness = (
        len(complex_tasks) == 12
        and len(wins) >= thresholds["min_complex_wins"]
        and not losses
    )
    if criteria.get("complex_effectiveness") is not effectiveness:
        errors.append("complex effectiveness state mismatch")
    regression = not required_losses and len(exact_tasks) == 6 and not exact_regressions
    if criteria.get("regression_safety") is not regression:
        errors.append("regression safety state mismatch")
    resources = acceptance.get("resources", {})
    bounds = (
        resources.get("complex_median_time_overhead") is not None
        and resources["complex_median_time_overhead"] <= thresholds["max_complex_time_overhead"]
        and resources.get("complex_median_input_overhead") is not None
        and resources["complex_median_input_overhead"] <= thresholds["max_complex_input_overhead"]
        and resources.get("exact_median_time_overhead") is not None
        and resources["exact_median_time_overhead"] <= thresholds["max_exact_overhead"]
        and resources.get("exact_median_input_overhead") is not None
        and resources["exact_median_input_overhead"] <= thresholds["max_exact_overhead"]
    )
    if criteria.get("bounded_cost") is not bounds:
        errors.append("cost boundary state mismatch")
    accepted = all(value is True for value in validity.values()) and all(
        value is True for value in criteria.values()
    )
    if acceptance.get("accepted") is not accepted:
        errors.append("accepted state mismatch")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("receipt")
    args = parser.parse_args()
    try:
        errors = verify(Path(args.receipt).resolve())
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as exc:
        print(f"flagship acceptance verifier: {exc}", file=sys.stderr)
        return 2
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print("flagship acceptance receipt verified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
