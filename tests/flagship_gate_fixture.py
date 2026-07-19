#!/usr/bin/env python3
"""Black-box proof for the deterministic 18-task, 72-run flagship gate."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from flagship_manifest import load_frozen  # noqa: E402
from flagship_gate_support import (  # noqa: E402
    fixture_repos,
    freeze,
    make_run,
    make_tools,
    report,
    task_rows,
    write,
)


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="codemap-flagship-test-") as temporary:
        root = Path(temporary)
        repos = fixture_repos(root)
        codex, codemap, verifier_path = make_tools(root)
        tasks = task_rows(repos, verifier_path)
        manifest_path, manifest = freeze(root, tasks, codex, codemap)

        accepted_run = make_run(root, "accepted", tasks, manifest)
        accepted = report(root, "accepted", manifest_path, accepted_run)
        assert accepted["acceptance"]["accepted"] is True
        verify = ROOT / "scripts/verify-flagship-acceptance.py"
        receipt = root / "acceptance-accepted/acceptance.json"
        assert subprocess.run([sys.executable, str(verify), str(receipt)]).returncode == 0

        for state in ("missing", "incomplete", "corrupt"):
            diagnostic = report(
                root,
                f"accepted-{state}-trajectory",
                manifest_path,
                accepted_run,
                trajectory_state=state,
            )
            assert diagnostic["acceptance"]["accepted"] is True, state
            assert diagnostic["trajectory_analysis"]["errors"], state

        cases = {
            "seven-wins": {"wins": 7},
            "unequal-arms": {"missing_arm": True},
            "required-loss": {"required_loss": True},
            "task-loss": {"loss": True},
            "exact-regression": {"exact_regression": True},
            "complex-cost": {"complex_over": True},
            "exact-cost": {"exact_over": True},
            "repeated-infrastructure": {"infrastructure_failure": True},
            "repo-write": {"repo_write": True},
        }
        for name, options in cases.items():
            rejected = report(root, name, manifest_path, make_run(root, name, tasks, manifest, **options))
            assert rejected["acceptance"]["accepted"] is False, name

        rows = accepted_run.joinpath("results.jsonl").read_text(encoding="utf-8").splitlines()
        write(accepted_run / "results.jsonl", "\n".join(rows[:-1]) + "\n")
        assert subprocess.run([sys.executable, str(verify), str(receipt)]).returncode == 1

        tampered = root / "frozen/tampered-manifest.json"
        changed = json.loads(manifest_path.read_text(encoding="utf-8"))
        changed["pair_schedule"][0]["arms"].reverse()
        write(tampered, json.dumps(changed))
        try:
            load_frozen(tampered)
        except ValueError as error:
            assert "schedule" in str(error)
        else:
            raise AssertionError("tampered manifest was accepted")

        write(verifier_path, "raise SystemExit(1)\n")
        try:
            load_frozen(manifest_path)
        except ValueError as error:
            assert "verifier artifact changed" in str(error)
        else:
            raise AssertionError("tampered verifier bytes were accepted")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
