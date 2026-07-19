"""Isolated Codex home lifecycle for one benchmark trial."""

from __future__ import annotations

import hashlib
import os
import tempfile
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import Iterator, Mapping


DISABLED_FEATURES = (
    "apps",
    "browser_use",
    "browser_use_external",
    "browser_use_full_cdp_access",
    "computer_use",
    "in_app_browser",
    "plugins",
    "remote_plugin",
)
DESKTOP_BROWSER_BINARY_ENV = "MCP_BROWSER_BINARY"
UNAVAILABLE_BROWSER_BINARY = "desktop-browser-unavailable-in-workspace-write"


def codex_runtime_sha256() -> str:
    return hashlib.sha256(Path(__file__).read_bytes()).hexdigest()


def codex_runtime_isolation_args() -> list[str]:
    return [part for feature in DISABLED_FEATURES for part in ("--disable", feature)]


@dataclass(frozen=True)
class CodexTrialRuntime:
    env: dict[str, str]
    auth_linked: bool

    def evidence(self) -> dict[str, object]:
        return {
            "codex_home": "isolated",
            "auth": "linked" if self.auth_linked else "environment_or_unavailable",
            "extensions": "disabled",
            "desktop_browser": "unavailable",
        }


@contextmanager
def isolated_codex_runtime(base_env: Mapping[str, str]) -> Iterator[CodexTrialRuntime]:
    """Expose auth, but no user config, skills, plugins, MCP, or browser state."""

    source_home = Path(
        base_env.get("CODEX_HOME") or Path.home() / ".codex"
    ).expanduser()
    with tempfile.TemporaryDirectory(prefix="codemap-codex-home-") as raw_home:
        runtime_home = Path(raw_home)
        source_auth = source_home / "auth.json"
        auth_linked = source_auth.is_file()
        if auth_linked:
            (runtime_home / "auth.json").symlink_to(source_auth)
        env = dict(base_env)
        env["CODEX_HOME"] = os.fspath(runtime_home)
        env[DESKTOP_BROWSER_BINARY_ENV] = os.fspath(
            runtime_home / UNAVAILABLE_BROWSER_BINARY
        )
        yield CodexTrialRuntime(env=env, auth_linked=auth_linked)
