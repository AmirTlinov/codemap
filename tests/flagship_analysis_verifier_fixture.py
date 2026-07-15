#!/usr/bin/env python3
"""Black-box citation parsing proof for arm-neutral analysis reports."""

from __future__ import annotations

import importlib.util
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
SPEC = importlib.util.spec_from_file_location(
    "flagship_external_verifier", ROOT / "benchmarks/flagship/verify.py"
)
assert SPEC and SPEC.loader
verifier = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(verifier)
from flagship_judging import blind_candidate_text  # noqa: E402


with tempfile.TemporaryDirectory(prefix="codemap-analysis-verifier-") as temporary:
    worktree = Path(temporary)
    (worktree / "src").mkdir()
    (worktree / "docs").mkdir()
    (worktree / "src/owner.rs").write_text("one\ntwo\n", encoding="utf-8")
    (worktree / "docs/contract.md").write_text("contract\n", encoding="utf-8")
    message = worktree / "report.md"
    message.write_text(
        "[src/owner.rs:2](/private/tmp/trial-codemap/src/owner.rs:2) and "
        "`docs/contract.md:1`; invalid missing/file.rs:3\n",
        encoding="utf-8",
    )
    receipt = verifier.citation_receipt(message, worktree)
    assert receipt["valid_citations"] == 2, receipt
    assert receipt["unique_valid_paths"] == 2, receipt
    assert receipt["top_level_surfaces"] == ["docs", "src"], receipt
    assert receipt["invalid_citations"] == [("missing/file.rs", 3)], receipt
    blinded = blind_candidate_text(message)
    assert "trial-codemap" not in blinded
    assert "src/owner.rs:2" in blinded
