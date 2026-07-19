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


def stable_direction(
    treatment: list[float | bool], control: list[float | bool], minimum: int
) -> str:
    if (
        len(treatment) != len(control)
        or minimum <= len(treatment) // 2
        or minimum > len(treatment)
    ):
        raise ValueError("stable direction requires equal arms and a strict repetition majority")
    wins = sum(left > right for left, right in zip(treatment, control, strict=True))
    losses = sum(left < right for left, right in zip(treatment, control, strict=True))
    if wins >= minimum:
        return "win"
    if losses >= minimum:
        return "loss"
    return "neutral"


def task_aggregate(
    pair_rows: list[dict[str, Any]], repetitions: int, minimum: int
) -> dict[str, Any]:
    if len(pair_rows) != repetitions:
        raise ValueError(f"every flagship task requires exactly {repetitions} valid pairs")
    pair_rows = sorted(pair_rows, key=lambda row: int(row["repetition"]))
    control = [float(row["control_score"]) for row in pair_rows]
    treatment = [float(row["codemap_score"]) for row in pair_rows]
    required_ids = set(pair_rows[0]["required_criteria"])
    if any(set(row["required_criteria"]) != required_ids for row in pair_rows):
        raise ValueError("required criterion identities changed between repetitions")
    required_directions = {
        criterion: stable_direction(
            [row["required_criteria"][criterion]["codemap"] for row in pair_rows],
            [row["required_criteria"][criterion]["control"] for row in pair_rows],
            minimum,
        )
        for criterion in sorted(required_ids)
    }
    control_outcomes = [row["control_outcome"] for row in pair_rows]
    codemap_outcomes = [row["codemap_outcome"] for row in pair_rows]
    return {
        "task_id": pair_rows[0]["task_id"],
        "repo_id": pair_rows[0]["repo_id"],
        "task_class": pair_rows[0]["task_class"],
        "control_score": statistics.mean(control),
        "codemap_score": statistics.mean(treatment),
        "delta": statistics.mean(treatment) - statistics.mean(control),
        "direction": stable_direction(treatment, control, minimum),
        "control_outcomes": control_outcomes,
        "codemap_outcomes": codemap_outcomes,
        "outcome_direction": stable_direction(codemap_outcomes, control_outcomes, minimum),
        "required_criterion_directions": required_directions,
        "required_criterion_losses": [
            criterion for criterion, direction in required_directions.items() if direction == "loss"
        ],
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
