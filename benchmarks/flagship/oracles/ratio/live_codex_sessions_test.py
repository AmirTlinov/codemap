#!/usr/bin/env python3
"""Black-box Codex-session episode, carrier, and world-return contract."""

from __future__ import annotations

import hashlib
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path.cwd()
BINARY = ROOT / "target/debug/ratiotissue"
CALL_ID = "call-fixture-1"


def run(argv: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=1200)


def records(*, output: bool) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = [
        {
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": "user",
                "content": [
                    {
                        "type": "input_text",
                        "text": "Open /Users/alice/private and email alice@example.com with sk-secret",
                    }
                ],
            },
        },
        {
            "type": "response_item",
            "payload": {
                "type": "function_call",
                "name": "exec_command",
                "call_id": CALL_ID,
                "arguments": json.dumps({"cmd": "python3 verify.py"}),
            },
        },
    ]
    if output:
        rows.append(
            {
                "type": "response_item",
                "payload": {
                    "type": "function_call_output",
                    "call_id": CALL_ID,
                    "output": "Process exited with code 0\nverifier passed",
                },
            }
        )
    rows.append(
        {
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": "assistant",
                "content": [
                    {
                        "type": "output_text",
                        "text": "Changed /Users/alice/private/result.txt and ran the verifier",
                    }
                ],
            },
        }
    )
    return rows


def run_case(
    root: Path,
    name: str,
    rows: list[dict[str, object]],
    *,
    carriers: bool = True,
) -> tuple[subprocess.CompletedProcess[str], Path]:
    root.mkdir(parents=True, exist_ok=True)
    session = root / f"{name}.jsonl"
    session.write_text("".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8")
    database = root / f"{name}-db"
    initialized = run([str(BINARY), "init", str(database)])
    assert initialized.returncode == 0, initialized.stdout + initialized.stderr
    carrier_dir = root / f"{name}-carriers"
    command = [
        str(BINARY),
        "live-codex-sessions",
        str(session),
        "--db",
        str(database),
        "--max-sessions",
        "1",
        "--max-episodes",
        "1",
        "--max-fragment-bytes",
        "512",
    ]
    if carriers:
        command.extend(["--carrier-dir", str(carrier_dir), "--carriers", str(carrier_dir)])
    return run(command), carrier_dir


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def episode_identity(stdout: str) -> str:
    for line in stdout.splitlines():
        if "live_codex_episode" not in line:
            continue
        match = re.search(r"(?:^|\s)(?:episode_id|content_id|id)=([^\s]+)", line)
        if match:
            return match.group(1)
    raise AssertionError("live_codex_episode receipt has no stable identity")


def observed_contact(stdout: str) -> bool:
    for line in stdout.splitlines():
        fields = dict(
            token.split("=", 1)
            for token in line.split()
            if "=" in token
        )
        contact = fields.get("contact", "").lower()
        actionwave = fields.get("actionwave", "").lower()
        world_return = fields.get("world_return", "").lower()
        rejected = {"", "false", "none", "no_action", "no_contact", "synthetic"}
        if (
            contact in {"true", "observed"}
            and actionwave not in rejected
            and world_return not in rejected
        ):
            return True
    return (
        "observed=true" in stdout
        or "world_return_observed=true" in stdout
        or re.search(r"world_returns_observed=[1-9][0-9]*", stdout) is not None
        or all(
            marker in stdout
            for marker in ("world_return=observed", "interaction_id=", "return_source_id=")
        )
        or all(
            marker in stdout
            for marker in (
                "live_codex_world_return_receipt",
                "world_return_keys=",
                "conductance_changed=",
            )
        )
        or all(
            marker in stdout
            for marker in ("world_return=contact", "interaction_id=", "conductance_changed=")
        )
        or all(marker in stdout for marker in ("contact=observed", "actionwave=", "world_return="))
        or all(
            marker in stdout
            for marker in ("contact=true", "world_return_keys=", "conductance_changed=")
        )
    )


def explicit_no_contact(result: subprocess.CompletedProcess[str]) -> bool:
    output = result.stdout.lower()
    return result.returncode != 0 and "no_action" in output and "no_contact" in output


def carrier_state(directory: Path) -> dict[str, str]:
    return {path.name: digest(path) for path in sorted(directory.glob("*"))}


def check_redaction_and_bounds(root: Path) -> dict[str, str]:
    result, carriers = run_case(root, "bounded", records(output=True))
    assert result.returncode == 0 or explicit_no_contact(result), result.stdout + result.stderr
    files = sorted(carriers.glob("*"))
    assert 1 <= len(files) <= 2, files
    bodies = "\n".join(path.read_text(encoding="utf-8") for path in files)
    assert bodies.strip()
    assert all(
        secret not in bodies for secret in ("/Users/alice", "alice@example.com", "sk-secret")
    )
    assert all(0 < path.stat().st_size <= 513 for path in files)
    return carrier_state(carriers)


def check_stable_carriers(root: Path) -> dict[str, str]:
    first, carriers = run_case(root, "stable", records(output=True))
    assert first.returncode == 0 or explicit_no_contact(first), first.stdout + first.stderr
    before = carrier_state(carriers)
    assert before
    first_episode = episode_identity(first.stdout)
    session = root / "stable.jsonl"
    database = root / "stable-db"
    second = run(
        [
            str(BINARY),
            "live-codex-sessions",
            str(session),
            "--db",
            str(database),
            "--carrier-dir",
            str(carriers),
            "--max-sessions",
            "1",
            "--max-episodes",
            "1",
            "--max-fragment-bytes",
            "512",
        ]
    )
    assert second.returncode == 0 or explicit_no_contact(second), second.stdout + second.stderr
    assert carrier_state(carriers) == before
    assert episode_identity(second.stdout) == first_episode
    return before


def check_world_return(root: Path) -> dict[str, str]:
    paired, carriers = run_case(root, "paired", records(output=True))
    assert paired.returncode == 0, paired.stdout + paired.stderr
    assert "actionwave" in paired.stdout.lower()
    assert observed_contact(paired.stdout), paired.stdout

    unpaired, _ = run_case(root, "unpaired", records(output=False), carriers=False)
    assert explicit_no_contact(unpaired), unpaired.stdout + unpaired.stderr
    return carrier_state(carriers)


def check_fail_closed(root: Path) -> dict[str, str]:
    metadata = [{"type": "session_meta", "payload": {"id": "fixture"}}]
    empty, _ = run_case(root, "metadata", metadata, carriers=False)
    assert empty.returncode != 0

    dialogue = [row for row in records(output=True) if row["payload"]["type"] == "message"]
    no_action, _ = run_case(root, "dialogue", dialogue, carriers=False)
    assert explicit_no_contact(no_action), no_action.stdout + no_action.stderr
    return {}


def main() -> int:
    criterion = sys.argv[1] if len(sys.argv) > 1 else "all"
    build = run(["cargo", "build", "--quiet", "-p", "ratiotissue-cli"])
    assert build.returncode == 0, build.stderr
    checks = {
        "fail-closed-metadata": check_fail_closed,
        "redaction-and-bounds": check_redaction_and_bounds,
        "stable-carriers": check_stable_carriers,
        "world-return-contact": check_world_return,
    }
    selected = checks if criterion == "all" else {criterion: checks[criterion]}
    carriers: dict[str, str] = {}
    with tempfile.TemporaryDirectory(prefix="ratio-live-codex-") as raw:
        for name, check in selected.items():
            carriers.update(check(Path(raw) / name))
    print(json.dumps({"passed": True, "criterion": criterion, "carriers": carriers}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
