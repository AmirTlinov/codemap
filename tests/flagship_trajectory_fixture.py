#!/usr/bin/env python3
"""Black-box fixture for paired causal trajectory materialization and analysis."""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from flagship_trajectory import analyze_trajectories  # noqa: E402


def write(path: Path, body: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body, encoding="utf-8")


def trial(root: Path, repetition: int, arm: str, order: int) -> dict:
    directory = root / "trials" / f"task-r{repetition}-{arm}"
    events = directory / "events.jsonl"
    final = directory / "last-message.md"
    patch = directory / "patch.diff"
    stdout = directory / "verify.stdout.log"
    stderr = directory / "verify.stderr.log"
    write(
        events,
        json.dumps(
            {
                "type": "item.completed",
                "item": {
                    "id": f"{arm}-read",
                    "type": "command_execution",
                    "command": "sed -n '1,20p' src/owner.rs",
                    "aggregated_output": f"{arm} owner evidence",
                    "status": "completed",
                },
            }
        )
        + "\n",
    )
    write(final, f"{arm} report\n")
    write(patch, f"diff --git a/owner b/owner\n+{arm}\n")
    write(stdout, f"{arm} verifier output\n")
    write(stderr, "")
    return {
        "task_id": "task",
        "repetition": repetition,
        "arm": arm,
        "order": order,
        "base_commit": "a" * 40,
        "outcome_passed": True,
        "completeness": {"score": 1.0},
        "changed_paths": ["src/owner.rs"],
        "codemap_activity": {"invocation_count": 1 if arm == "codemap" else 0},
        "codex": {
            "events_artifact": str(events),
            "last_message_artifact": str(final),
            "elapsed_ms": 100,
            "usage": {"input_tokens": 200},
        },
        "verifiers": [
            {
                "name": "contract",
                "category": "contract",
                "required": True,
                "status": 0,
                "timed_out": False,
                "passed": True,
                "stdout_artifact": str(stdout),
                "stderr_artifact": str(stderr),
            }
        ],
    }


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="codemap-trajectory-test-") as temporary:
        root = Path(temporary)
        fake = root / "fake-codex.py"
        write(
            fake,
            """import json, sys
from pathlib import Path
if '--version' in sys.argv:
    print('codex-cli fixture')
else:
    output = Path(sys.argv[sys.argv.index('-o') + 1])
    output.write_text('Причинный разбор [A:control-read] [B:codemap-read].\\n', encoding='utf-8')
    print(json.dumps({'type':'turn.completed','usage':{'input_tokens':10,'output_tokens':4}}))
""",
        )
        manifest = {
            "model": "gpt-fixture",
            "reasoning_effort": "high",
            "codex": {"command_argv": [sys.executable, str(fake)], "version": "fixture"},
            "limits": {"timeout_seconds": 30, "parallel_pairs": 1},
        }
        manifest_path = root / "manifest.json"
        write(manifest_path, json.dumps(manifest))
        task = {"id": "task", "repo": str(root / "repo"), "prompt": "Trace the owner."}
        run = root / "run"
        rows = []
        for repetition in (1, 2):
            order = ("control", "codemap") if repetition == 1 else ("codemap", "control")
            rows.extend(trial(run, repetition, arm, order.index(arm) + 1) for arm in order)
        write(run / "results.jsonl", "".join(json.dumps(row) + "\n" for row in rows))
        summary_path = analyze_trajectories(
            manifest_path, [task], run, root / "analysis", resume=False
        )
        summary = json.loads(summary_path.read_text(encoding="utf-8"))
        assert len(summary["pairs"]) == 2
        assert all(row["complete"] for row in summary["pairs"])
        assert summary["complete"] is False
        context = root / "analysis/task-r1/pair-context.md"
        body = context.read_text(encoding="utf-8")
        for expected in (
            "[A:control-read]",
            "[B:codemap-read]",
            "[A:diff]",
            "[B:verify:contract:stdout]",
            '"input_tokens": 200',
        ):
            assert expected in body
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
