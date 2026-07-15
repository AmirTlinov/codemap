"""Normative frozen settings for the S15 behavioral gate."""

from __future__ import annotations

from typing import Any


REASONING_LEVELS = {"minimal", "low", "medium", "high", "xhigh"}
PAIR_ORDER = "split_task_index_plus_repetition_v1"
GATE_FILES = (
    "benchmark-codemap-flagship.py",
    "benchmark_parallel.py",
    "codemap_protocol_shim.py",
    "flagship_acceptance.py",
    "flagship_artifacts.py",
    "flagship_contract.py",
    "flagship_judge_runner.py",
    "flagship_judging.py",
    "flagship_manifest.py",
    "flagship_receipts.py",
    "flagship_stats.py",
    "verify-flagship-acceptance.py",
)
REQUIRED_ACCEPTANCE = {
    "primary_alpha": 0.05,
    "min_task_win_rate": 0.60,
    "max_complex_time_overhead": 0.20,
    "max_complex_input_overhead": 0.15,
    "max_negative_overhead": 0.10,
    "min_agreement_alpha": 0.67,
}


def validate_draft(draft: dict[str, Any]) -> None:
    if draft.get("model") != "gpt-5.6-sol":
        raise ValueError("flagship model must be gpt-5.6-sol")
    if draft.get("reasoning_effort") not in REASONING_LEVELS:
        raise ValueError("invalid reasoning_effort")
    if not isinstance(draft.get("repetitions"), int) or draft["repetitions"] < 3:
        raise ValueError("flagship requires at least 3 repetitions")
    if not isinstance(draft.get("parallel_pairs"), int) or not 1 <= draft["parallel_pairs"] <= 8:
        raise ValueError("parallel_pairs must be between 1 and 8")
    for field in ("timeout_seconds", "verifier_timeout_seconds", "bootstrap_iterations"):
        if not isinstance(draft.get(field), int) or draft[field] <= 0:
            raise ValueError(f"{field} must be a positive integer")
    if draft["bootstrap_iterations"] < 10_000:
        raise ValueError("bootstrap_iterations must be at least 10000")
    if not isinstance(draft.get("bootstrap_seed"), int):
        raise ValueError("bootstrap_seed must be an integer")
    if draft.get("pair_order") != PAIR_ORDER:
        raise ValueError(f"pair_order must be {PAIR_ORDER}")
    acceptance = draft.get("acceptance")
    if not isinstance(acceptance, dict):
        raise ValueError("acceptance thresholds are required")
    for field, expected in REQUIRED_ACCEPTANCE.items():
        if acceptance.get(field) != expected:
            raise ValueError(f"acceptance.{field} must equal {expected}")
    if acceptance.get("allow_completeness_exception") is not True:
        raise ValueError("pre-registered completeness exception must be explicit")
    judging = draft.get("judging")
    if not isinstance(judging, dict):
        raise ValueError("blind judging contract is required")
    for field in ("assignment_seed", "manual_audit_seed"):
        if not isinstance(judging.get(field), int):
            raise ValueError(f"judging.{field} must be an integer")
    sample = judging.get("manual_audit_sample_size")
    if not isinstance(sample, int) or sample < 6:
        raise ValueError("manual audit sample must contain at least 6 analysis pairs")
    if judging.get("judges_per_candidate") != 2 or judging.get("blind_adjudication") is not True:
        raise ValueError("judging requires two blind judges and blind adjudication")
    if judging.get("model") != draft["model"]:
        raise ValueError("blind judges must use the frozen experiment model")
    if judging.get("reasoning_effort") not in REASONING_LEVELS:
        raise ValueError("invalid judging.reasoning_effort")
    timeout = judging.get("timeout_seconds")
    if not isinstance(timeout, int) or timeout <= 0:
        raise ValueError("judging.timeout_seconds must be positive")
    workers = judging.get("parallel_jobs")
    if not isinstance(workers, int) or not 1 <= workers <= 8:
        raise ValueError("judging.parallel_jobs must be between 1 and 8")
