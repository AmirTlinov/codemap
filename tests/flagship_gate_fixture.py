#!/usr/bin/env python3
"""Black-box proof for the stable-effect 18-task, 144-run flagship gate."""

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
        complex_ids = [
            task["id"]
            for task in tasks
            if task["benchmark"]["task_class"] != "exact_control"
        ]
        assert manifest["limits"]["repetitions"] == 4
        assert manifest["acceptance"]["min_direction_repetitions"] == 3
        assert len(manifest["pair_schedule"]) == 72
        for task in tasks:
            schedule = [
                row["arms"] for row in manifest["pair_schedule"] if row["task_id"] == task["id"]
            ]
            assert sum(arms[0] == "control" for arms in schedule) == 2
            assert sum(arms[0] == "codemap" for arms in schedule) == 2

        accepted_run = make_run(root, "accepted", tasks, manifest)
        accepted = report(root, "accepted", manifest_path, accepted_run)
        assert accepted["acceptance"]["accepted"] is True
        assert accepted["run"]["expected_trials"] == 144
        assert accepted["run"]["observed_trials"] == 144
        assert accepted["run"]["expected_pairs"] == 72
        assert accepted["run"]["valid_pairs"] == 72
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

        rejected_cases = {
            "seven-wins": {"wins": 7},
            "unequal-arms": {"missing_arm": True},
            "required-loss": {"required_loss_repetitions": 3},
            "task-loss": {"task_directions": {complex_ids[-1]: [-1, -1, -1, 0]}},
            "exact-regression": {"exact_regression_repetitions": 3},
            "complex-cost": {"complex_over": True},
            "exact-cost": {"exact_over": True},
            "repeated-infrastructure": {"infrastructure_failure": True},
            "repo-write": {"repo_write": True},
        }
        for name, options in rejected_cases.items():
            rejected = report(
                root, name, manifest_path, make_run(root, name, tasks, manifest, **options)
            )
            assert rejected["acceptance"]["accepted"] is False, name
            if name == "task-loss":
                task = next(
                    row for row in rejected["run"]["tasks"] if row["task_id"] == complex_ids[-1]
                )
                assert task["direction"] == "loss"
            if name == "required-loss":
                assert rejected["acceptance"]["required_criterion_losses"]
            if name == "exact-regression":
                assert rejected["acceptance"]["exact_regressions"]

        stable_cases = {
            "single-task-miss": {"task_directions": {complex_ids[-1]: [-1, 0, 0, 0]}},
            "mixed-direction": {
                "wins": 9,
                "task_directions": {complex_ids[8]: [1, 1, -1, -1]},
            },
            "three-wins": {
                "wins": 7,
                "task_directions": {complex_ids[7]: [1, 1, 1, 0]},
            },
            "single-required-miss": {"required_loss_repetitions": 1},
            "single-exact-miss": {"exact_regression_repetitions": 1},
        }
        for name, options in stable_cases.items():
            result = report(
                root, name, manifest_path, make_run(root, name, tasks, manifest, **options)
            )
            assert result["acceptance"]["accepted"] is True, name
            assert not result["acceptance"]["complex"]["losing_tasks"], name
            if name == "single-task-miss":
                task = next(
                    row for row in result["run"]["tasks"] if row["task_id"] == complex_ids[-1]
                )
                assert task["direction"] == "neutral"
            if name == "mixed-direction":
                task = next(
                    row for row in result["run"]["tasks"] if row["task_id"] == complex_ids[8]
                )
                assert task["direction"] == "neutral"
            if name == "three-wins":
                task = next(
                    row for row in result["run"]["tasks"] if row["task_id"] == complex_ids[7]
                )
                assert task["direction"] == "win"
            if name == "single-required-miss":
                assert not result["acceptance"]["required_criterion_losses"]
            if name == "single-exact-miss":
                assert not result["acceptance"]["exact_regressions"]

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
