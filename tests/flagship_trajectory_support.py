"""Fixture builder for complete, non-scoring trajectory-analysis evidence."""

from __future__ import annotations

import json
from pathlib import Path

from flagship_manifest import file_sha256


def build_trajectory(root: Path, tasks: list[dict], manifest_path: Path) -> Path:
    root.mkdir()
    pairs = []
    for task in tasks:
        for repetition in (1, 2):
            pair = root / f"{task['id']}-r{repetition}"
            pair.mkdir()
            context = pair / "pair-context.md"
            report = pair / "analysis.md"
            context.write_text("[task] fixture\n[A:item_1] read\n[B:item_1] read\n", encoding="utf-8")
            report.write_text("Обе траектории подтверждены [A:item_1] [B:item_1].\n", encoding="utf-8")
            pairs.append(
                {
                    "task_id": task["id"],
                    "repetition": repetition,
                    "labels": {"A": "control", "B": "codemap"},
                    "context_sha256": file_sha256(context),
                    "status": 0,
                    "timed_out": False,
                    "complete": True,
                    "report": str(report),
                    "report_sha256": file_sha256(report),
                }
            )
    summary = {
        "kind": "codemap_flagship_trajectory_analysis",
        "version": 1,
        "manifest_sha256": file_sha256(manifest_path),
        "pairs": pairs,
        "complete": True,
    }
    output = root / "summary.json"
    output.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    return output
