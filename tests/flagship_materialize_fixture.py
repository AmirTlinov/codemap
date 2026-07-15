#!/usr/bin/env python3
"""Black-box proof that materialization makes provenance executable."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "benchmarks/flagship"))

from materialize import materialize  # noqa: E402


def git(repo: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo), *args], capture_output=True, text=True, check=True
    )
    return result.stdout.strip()


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="codemap-materialize-") as raw:
        root = Path(raw)
        source = root / "source"
        source.mkdir()
        git(source, "init", "-q")
        git(source, "config", "user.email", "fixture@example.com")
        git(source, "config", "user.name", "fixture")
        (source / "README.md").write_text("original phrase\n", encoding="utf-8")
        git(source, "add", ".")
        git(source, "commit", "-qm", "fixture")
        base = git(source, "rev-parse", "HEAD")
        criteria = {
            name: {"kind": "files", "exists": ["README.md"]}
            for name in ("required", "behavior", "contract", "downstream", "regression")
        }
        blueprint = {
            "kind": "codemap_flagship_blueprint",
            "version": 1,
            "repositories": [
                {
                    "id": "fixture",
                    "remote": str(source),
                    "source": str(source),
                    "base": base,
                    "negative_mutations": [
                        {"path": "README.md", "before": "original", "after": "seeded"}
                    ],
                }
            ],
            "verifier_artifacts": [],
            "experiment": {"model": "fixture"},
            "tasks": [
                {
                    "id": "fixture-task",
                    "repo_id": "fixture",
                    "ecosystem": "fixture",
                    "task_class": "implementation",
                    "split": "calibration",
                    "prompt": "Restore the exact phrase.",
                    "criteria": criteria,
                    "exception_criteria": ["behavior"],
                }
            ],
        }
        blueprint_path = root / "blueprint.json"
        blueprint_path.write_text(json.dumps(blueprint), encoding="utf-8")
        draft = materialize(blueprint_path, root / "corpus", False)
        task = json.loads((draft.parent / "tasks.jsonl").read_text(encoding="utf-8"))
        names = [criterion["name"] for criterion in task["verify"]]
        assert names == ["required", "behavior", "contract", "downstream", "regression", "provenance"]
        receipt = json.loads((draft.parent / "materialization-receipt.json").read_text())
        variants = {row["variant"]: row for row in receipt["repositories"]}
        benchmark_commit = variants["clean"]["benchmark_commit"]
        assert benchmark_commit == base
        assert variants["negative"]["benchmark_commit"] != base
        spec = json.loads((draft.parent / "verification-spec.json").read_text())
        assert spec["tasks"]["fixture-task"]["provenance"]["commit"] == benchmark_commit
        assert git(Path(task["repo"]), "rev-parse", "HEAD") == benchmark_commit
        assert (Path(task["repo"]) / "README.md").read_text() == "original phrase\n"
        negative = draft.parent / "repositories/fixture-negative/README.md"
        assert negative.read_text() == "seeded phrase\n"
        second = materialize(blueprint_path, root / "second-corpus", False)
        second_receipt = json.loads(
            (second.parent / "materialization-receipt.json").read_text()
        )
        second_variants = {
            row["variant"]: row["benchmark_commit"]
            for row in second_receipt["repositories"]
        }
        assert second_variants == {
            variant: row["benchmark_commit"] for variant, row in variants.items()
        }
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
