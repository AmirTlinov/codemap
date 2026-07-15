#!/usr/bin/env python3
"""Independently verify an S15 acceptance receipt without importing its evaluator."""

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
    acceptance = report.get("acceptance", {})
    checks = acceptance.get("checks", {})
    accepted = acceptance.get("accepted")
    if not isinstance(accepted, bool) or accepted != all(value is True for value in checks.values()):
        errors.append("accepted state does not equal all normative checks")
    bootstrap = acceptance.get("bootstrap", {})
    primary = checks.get("primary_bootstrap_lower_bound_above_zero")
    if primary is not (isinstance(bootstrap.get("lower_bound"), (int, float)) and bootstrap["lower_bound"] > 0):
        errors.append("primary endpoint state mismatch")
    holdout = report.get("holdout", {})
    complete = (
        not holdout.get("invalid")
        and holdout.get("valid_pairs") == holdout.get("expected_pairs")
        and holdout.get("observed_trials") == holdout.get("expected_trials")
    )
    if checks.get("complete_valid_denominator") is not complete:
        errors.append("holdout denominator state mismatch")
    agreement = report.get("agreement", {})
    agreement_valid = bool(agreement) and all(row.get("valid") is True for row in agreement.values())
    if checks.get("ordinal_agreement") is not agreement_valid:
        errors.append("ordinal agreement state mismatch")
    calibration = report.get("calibration", {})
    if calibration.get("split") != "calibration" or holdout.get("split") != "holdout":
        errors.append("calibration and holdout are not separated")
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
