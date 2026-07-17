#!/usr/bin/env python3
"""Independently verify a deterministic flagship acceptance receipt."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def verify(path: Path) -> list[str]:
    report = json.loads(path.read_text(encoding="utf-8"))
    errors = []
    if report.get("kind") != "codemap_flagship_acceptance" or report.get("version") != 1:
        errors.append("unsupported acceptance receipt")
    manifest = Path(report.get("manifest", ""))
    if not manifest.is_file() or sha256(manifest) != report.get("manifest_sha256"):
        errors.append("manifest hash mismatch")
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
        and run.get("observed_trials") == 72
        and run.get("valid_pairs") == 36
        and len(run.get("tasks", [])) == 18
    )
    if validity.get("complete_72_run_denominator") is not complete:
        errors.append("72-run denominator state mismatch")
    zero_write = not run.get("zero_write_violations")
    if validity.get("zero_repo_writes_for_read_only_tasks") is not zero_write:
        errors.append("zero-write state mismatch")
    trajectory = report.get("trajectory_analysis", {})
    trajectory_complete = trajectory.get("pairs") == 36 and not trajectory.get("errors")
    if validity.get("paired_trajectory_analysis") is not trajectory_complete:
        errors.append("trajectory evidence state mismatch")
    complex_result = acceptance.get("complex", {})
    effectiveness = complex_result.get("wins", 0) >= 8 and not complex_result.get("losing_tasks")
    if criteria.get("complex_effectiveness") is not effectiveness:
        errors.append("complex effectiveness state mismatch")
    regression = not acceptance.get("required_criterion_losses") and not acceptance.get(
        "exact_regressions"
    )
    if criteria.get("regression_safety") is not regression:
        errors.append("regression safety state mismatch")
    resources = acceptance.get("resources", {})
    bounds = (
        resources.get("complex_median_time_overhead") is not None
        and resources["complex_median_time_overhead"] <= 0.20
        and resources.get("complex_median_input_overhead") is not None
        and resources["complex_median_input_overhead"] <= 0.15
        and resources.get("exact_median_time_overhead") is not None
        and resources["exact_median_time_overhead"] <= 0.10
        and resources.get("exact_median_input_overhead") is not None
        and resources["exact_median_input_overhead"] <= 0.10
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
