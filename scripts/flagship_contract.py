"""Normative settings for the outcome-based flagship A/B gate."""

from __future__ import annotations

from typing import Any


MODEL = "gpt-5.6-sol"
REASONING_EFFORT = "high"
PAIR_ORDER = "task_index_plus_repetition_v1"
TASK_CLASSES = {"investigation": 6, "implementation": 6, "exact_control": 6}
REQUIRED_ACCEPTANCE = {
    "min_complex_wins": 8,
    "max_complex_time_overhead": 0.20,
    "max_complex_input_overhead": 0.15,
    "max_exact_overhead": 0.10,
}


def validate_draft(draft: dict[str, Any]) -> None:
    if draft.get("model") != MODEL:
        raise ValueError(f"flagship model must be {MODEL}")
    if draft.get("reasoning_effort") != REASONING_EFFORT:
        raise ValueError(f"flagship reasoning_effort must be {REASONING_EFFORT}")
    if draft.get("pair_order") != PAIR_ORDER:
        raise ValueError(f"pair_order must be {PAIR_ORDER}")
    limits = draft.get("limits")
    if not isinstance(limits, dict):
        raise ValueError("flagship limits are required")
    if limits.get("repetitions") != 2:
        raise ValueError("flagship requires exactly two counterbalanced repetitions")
    if limits.get("infrastructure_retries") != 1:
        raise ValueError("flagship infrastructure failures must retry exactly once")
    for field in ("timeout_seconds", "verifier_timeout_seconds"):
        if not isinstance(limits.get(field), int) or limits[field] <= 0:
            raise ValueError(f"limits.{field} must be a positive integer")
    parallel = limits.get("parallel_pairs")
    if not isinstance(parallel, int) or not 1 <= parallel <= 8:
        raise ValueError("limits.parallel_pairs must be between 1 and 8")
    if draft.get("acceptance") != REQUIRED_ACCEPTANCE:
        raise ValueError("flagship acceptance thresholds are fixed by the product contract")
