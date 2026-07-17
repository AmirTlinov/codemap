#!/usr/bin/env python3
"""Public active-plan projection contract for core-daemon and WorkbenchCore."""

from __future__ import annotations

import json
import os
import subprocess
import tempfile
from pathlib import Path
from typing import Any


ROOT = Path.cwd()
DAEMON = ROOT / "target/debug/core-daemon"


def run(argv: list[str], *, cwd: Path = ROOT, env: dict[str, str] | None = None, input: str | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        argv,
        cwd=cwd,
        env={**os.environ, **(env or {})},
        input=input,
        capture_output=True,
        text=True,
        timeout=1200,
    )


def field(value: dict[str, Any], camel: str, snake: str) -> Any:
    return value[camel] if camel in value else value[snake]


def selected_slice(value: dict[str, Any], expected_id: str | None) -> Any:
    for candidate in value.values():
        if isinstance(candidate, dict) and candidate.get("id") == expected_id:
            return candidate
    raise KeyError("first non-done slice")


def roadmap_rows() -> list[tuple[str, str]]:
    rows = []
    header = False
    for raw in (ROOT / "ROADMAP.md").read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line.startswith("|") or not line.endswith("|"):
            continue
        cells = [cell.strip().strip("`") for cell in line[1:-1].split("|")]
        if len(cells) != 4:
            continue
        if cells[0].lower() == "slice" and cells[3].lower() == "status":
            header = True
            continue
        if not header or all(set(cell) <= {"-", ":", " "} for cell in cells):
            continue
        rows.append((cells[0], cells[3].lower()))
    return rows


def validate_projection(payload: dict[str, Any]) -> None:
    assert payload["kind"] in {"active_plan", "active_plan_projection"}, payload
    rows = roadmap_rows()
    slices = payload["slices"]
    assert [(row["id"], row["status"]) for row in slices] == rows
    counts = payload["counts"]
    for status in ("done", "active", "planned", "blocked"):
        assert counts[status] == sum(row_status == status for _, row_status in rows)
    expected = next((row_id for row_id, status in rows if status != "done"), None)
    active = selected_slice(payload, expected)
    assert (active and active["id"]) == expected
    assert field(payload, "roadmapPath", "roadmap_path") == "ROADMAP.md"

    proof = active.get("proof", active.get("proofs"))
    assert proof, active
    for artifact in proof.values():
        available = artifact.get(
            "available", artifact.get("exists", artifact["state"].lower() != "missing")
        )
        artifact_path = artifact.get("path")
        present = bool(artifact_path and (ROOT / artifact_path).is_file())
        assert available == present
        assert artifact["state"]


def main() -> int:
    build = run(["cargo", "build", "--quiet", "-p", "core-daemon"])
    assert build.returncode == 0, build.stderr

    cli = run([str(DAEMON), "--active-plan"])
    assert cli.returncode == 0, cli.stderr
    projection = json.loads(cli.stdout)
    validate_projection(projection)

    stdio = run(
        [str(DAEMON), "--serve-stdio"],
        input=json.dumps({"operation": "active-plan"}) + "\n",
    )
    assert stdio.returncode == 0, stdio.stderr
    validate_projection(json.loads(stdio.stdout.splitlines()[0]))

    with tempfile.TemporaryDirectory(prefix="openslop-active-plan-") as raw:
        empty = Path(raw)
        missing = run(
            [str(DAEMON), "--active-plan"],
            cwd=empty,
            env={"OPEN_SLOP_REPO_ROOT": str(empty)},
        )
        assert missing.returncode != 0
        (empty / "ROADMAP.md").write_text("# ROADMAP\n", encoding="utf-8")
        no_rows = run(
            [str(DAEMON), "--active-plan"],
            cwd=empty,
            env={"OPEN_SLOP_REPO_ROOT": str(empty)},
        )
        assert no_rows.returncode != 0

    probe = run(["make", "probe-active-plan"])
    assert probe.returncode == 0, probe.stdout + probe.stderr
    print(json.dumps({"passed": True, "slices": len(projection["slices"])}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
