"""Cross-check aggregate rows against immutable per-trial benchmark receipts."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


def trial_receipt_errors(row: dict[str, Any]) -> list[str]:
    errors = []
    codex = row.get("codex", {})
    required = ("events_artifact", "stderr_artifact", "last_message_artifact")
    for field in required:
        artifact = Path(str(codex.get(field, "")))
        if not artifact.is_file():
            errors.append(f"missing_{field}")
    patch = Path(str(row.get("patch_artifact", "")))
    if not patch.is_file():
        errors.append("missing_patch_artifact")
    last_message = Path(str(codex.get("last_message_artifact", "")))
    result_path = last_message.parent / "result.json"
    if not result_path.is_file():
        errors.append("missing_trial_receipt")
    else:
        try:
            if json.loads(result_path.read_text(encoding="utf-8")) != row:
                errors.append("aggregate_trial_receipt_mismatch")
        except json.JSONDecodeError:
            errors.append("invalid_trial_receipt")
    for verifier in row.get("verifiers", []):
        expected_passed = verifier.get("status") == 0 and verifier.get("timed_out") is False
        if verifier.get("passed") is not expected_passed:
            errors.append(f"verifier_state_mismatch:{verifier.get('name')}")
        for field in ("stdout_artifact", "stderr_artifact"):
            if not Path(str(verifier.get(field, ""))).is_file():
                errors.append(f"missing_verifier_artifact:{verifier.get('name')}:{field}")
    return errors
