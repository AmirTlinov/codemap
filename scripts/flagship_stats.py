"""Small deterministic aggregations for the outcome-based flagship gate."""

from __future__ import annotations

import statistics
from typing import Any


def median(values: list[float]) -> float | None:
    return statistics.median(values) if values else None


def relative_delta(treatment: float, control: float) -> float | None:
    if control <= 0:
        return 0.0 if treatment <= 0 else None
    return (treatment - control) / control


def criterion_score(criteria: list[dict[str, Any]]) -> float:
    total = sum(float(row["weight"]) for row in criteria)
    if total <= 0:
        raise ValueError("criterion weights must be positive")
    return sum(float(row["weight"]) * float(row["value"]) for row in criteria) / total


def task_aggregate(pair_rows: list[dict[str, Any]]) -> dict[str, Any]:
    if len(pair_rows) != 2:
        raise ValueError("every flagship task requires exactly two valid pairs")
    control = [float(row["control_score"]) for row in pair_rows]
    treatment = [float(row["codemap_score"]) for row in pair_rows]
    return {
        "task_id": pair_rows[0]["task_id"],
        "repo_id": pair_rows[0]["repo_id"],
        "task_class": pair_rows[0]["task_class"],
        "control_score": statistics.mean(control),
        "codemap_score": statistics.mean(treatment),
        "delta": statistics.mean(treatment) - statistics.mean(control),
        "control_outcomes": [row["control_outcome"] for row in pair_rows],
        "codemap_outcomes": [row["codemap_outcome"] for row in pair_rows],
        "time_overhead": median(
            [
                value
                for row in pair_rows
                if (value := relative_delta(row["codemap_elapsed"], row["control_elapsed"]))
                is not None
            ]
        ),
        "input_overhead": median(
            [
                value
                for row in pair_rows
                if (value := relative_delta(row["codemap_input"], row["control_input"]))
                is not None
            ]
        ),
    }
