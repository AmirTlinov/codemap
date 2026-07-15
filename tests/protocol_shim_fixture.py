#!/usr/bin/env python3
"""Prove the A/B shim separates agent navigation from project consumers."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from codemap_protocol import codemap_protocol  # noqa: E402
from codemap_protocol_shim import (  # noqa: E402
    is_agent_direct,
    shell_profile_environment,
    write_shim,
)


def executable(path: Path, body: str) -> Path:
    path.write_text(body, encoding="utf-8")
    path.chmod(0o755)
    return path


def rows(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text().splitlines()]


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="codemap-protocol-shim-") as raw:
        root = Path(raw)
        log = root / "invocations.jsonl"
        project = executable(root / "project-codemap", "#!/bin/sh\necho project:$*\n")
        benchmark = executable(root / "benchmark-codemap", "#!/bin/sh\necho benchmark:$*\n")
        env = {
            **os.environ,
            "CODEMAP_AB_INVOCATION_LOG": str(log),
            "CARGO_BIN_EXE_codemap": str(project),
        }

        control = write_shim(root / "control", "control", [str(benchmark)])
        shell_env = {**env, **shell_profile_environment(control.parent)}
        shell_env["PATH"] = str(control.parent) + os.pathsep + shell_env["PATH"]
        for shell in ("zsh", "bash"):
            resolved = subprocess.run(
                [shell, "-lc", "command -v codemap"],
                env=shell_env,
                capture_output=True,
                text=True,
                check=True,
            )
            assert Path(resolved.stdout.strip()).resolve() == control.resolve()
        assert is_agent_direct(["/bin/zsh", "/opt/codex-code-mode-host"])
        assert is_agent_direct(["/opt/codex"])
        assert not is_agent_direct(["/tmp/debug/deps/project-tests", "/bin/zsh", "/opt/codex"])
        direct_rows = [{"argv": ["ls", "README.md"], "status": 127, "agent_direct": True}]
        assert codemap_protocol("analysis", "control", direct_rows)["compliant"] is False
        bypass = codemap_protocol(
            "analysis",
            "control",
            [],
            agent_commands=["/bin/zsh -lc '/opt/benchmark/codemap ls README.md'"],
        )
        assert bypass["compliant"] is False
        assert bypass["agent_command_trace_matches"] is False

        internal = subprocess.run(
            [str(control), "--version"], env=env, capture_output=True, text=True
        )
        assert internal.returncode == 0 and internal.stdout.strip() == "project:--version"
        internal_rows = rows(log)
        report = codemap_protocol("analysis", "control", internal_rows)
        assert internal_rows[0]["agent_direct"] is False
        assert report["compliant"] is True
        assert report["invocation_count"] == 0
        assert report["ignored_internal_invocation_count"] == 1

        treatment_rows = [
            {"argv": ["cone", "README.md"], "status": 0, "agent_direct": True}
        ]
        treatment_report = codemap_protocol(
            "analysis",
            "codemap",
            treatment_rows,
            agent_commands=["/bin/zsh -lc 'codemap cone README.md'"],
        )
        assert treatment_report["compliant"] is True
        assert treatment_report["agent_command_trace_matches"] is True
        assert not codemap_protocol(
            "analysis",
            "control",
            [],
            agent_commands=["/bin/zsh -lc 'rg codemap README.md'"],
        )["agent_command_invocations"]
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
