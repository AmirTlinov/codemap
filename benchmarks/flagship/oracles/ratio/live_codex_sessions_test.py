#!/usr/bin/env python3
"""Black-box Codex-session episode, carrier, and world-return contract."""

from __future__ import annotations

import hashlib
import json
import re
import subprocess
import tempfile
from pathlib import Path


ROOT = Path.cwd()
BINARY = ROOT / "target/debug/ratiotissue"


def run(argv: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=1200)


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    build = run(["cargo", "build", "--quiet", "-p", "ratiotissue-cli"])
    assert build.returncode == 0, build.stderr

    with tempfile.TemporaryDirectory(prefix="ratio-live-codex-") as raw:
        root = Path(raw)
        session = root / "session.jsonl"
        records = [
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
                    "type": "message",
                    "role": "assistant",
                    "content": [
                        {
                            "type": "output_text",
                            "text": "Changed /Users/alice/private/result.txt and ran the verifier",
                        }
                    ],
                },
            },
        ]
        session.write_text("".join(json.dumps(row) + "\n" for row in records), encoding="utf-8")
        database = root / "db"
        carriers = root / "carriers"
        initialized = run([str(BINARY), "init", str(database)])
        assert initialized.returncode == 0, initialized.stdout + initialized.stderr
        command = [
            str(BINARY),
            "live-codex-sessions",
            str(session),
            "--db",
            str(database),
            "--carrier-dir",
            str(carriers),
            "--carriers",
            str(carriers),
            "--max-sessions",
            "1",
            "--max-episodes",
            "1",
            "--limit",
            "1",
            "--max-fragment-bytes",
            "512",
        ]
        first = run(command)
        assert first.returncode == 0, first.stdout + first.stderr
        assert "actionwave" in first.stdout.lower()
        assert "live_codex_" in first.stdout and "episode_id=" in first.stdout
        observed = (
            "observed=true" in first.stdout
            or "world_return_observed=true" in first.stdout
            or re.search(r"world_returns_observed=[1-9][0-9]*", first.stdout) is not None
            or all(
                marker in first.stdout
                for marker in (
                    "live_codex_world_return_receipt",
                    "world_return_keys=",
                    "conductance_changed=",
                )
            )
            or all(
                marker in first.stdout
                for marker in ("contact=true", "world_return_keys=", "conductance_changed=")
            )
        )
        if observed:
            assert "conductance_changed=" in first.stdout
        else:
            no_emit = "reason=no_emit" in first.stdout and (
                "observed=false" in first.stdout
                or "world_return_observed=false" in first.stdout
                or "contact=false" in first.stdout
            )
            no_action = all(
                marker in first.stdout
                for marker in (
                    "world_return=none",
                    "reason=no_action",
                    "contact_claimed=false",
                )
            )
            assert no_emit or no_action

        files = sorted(carriers.glob("*"))
        assert 1 <= len(files) <= 2, files
        before = {path.name: digest(path) for path in files}
        bodies = "\n".join(path.read_text(encoding="utf-8") for path in files)
        assert bodies.strip()
        assert all(
            secret not in bodies
            for secret in ("/Users/alice", "alice@example.com", "sk-secret")
        )
        assert all(0 < path.stat().st_size <= 513 for path in files)

        second = run(command)
        assert second.returncode == 0, second.stdout + second.stderr
        after = {path.name: digest(path) for path in sorted(carriers.glob("*"))}
        assert after == before
        first_episode = next(part for part in first.stdout.split() if part.startswith("episode_id="))
        second_episode = next(part for part in second.stdout.split() if part.startswith("episode_id="))
        assert first_episode == second_episode

        metadata = root / "metadata.jsonl"
        metadata.write_text(
            json.dumps({"type": "session_meta", "payload": {"id": "fixture"}}) + "\n",
            encoding="utf-8",
        )
        empty = run(
            [
                str(BINARY),
                "live-codex-sessions",
                str(metadata),
                "--db",
                str(database),
            ]
        )
        assert empty.returncode != 0

        print(json.dumps({"passed": True, "carriers": before}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
