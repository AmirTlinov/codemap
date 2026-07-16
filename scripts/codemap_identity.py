#!/usr/bin/env python3
"""Resolve one codemap command and capture attributable binary provenance."""

from __future__ import annotations

import hashlib
import json
import os
import re
import shlex
import shutil
import subprocess
from pathlib import Path
from typing import Any


class CodemapIdentityError(ValueError):
    pass


def canonical(path: Path) -> Path:
    return path.expanduser().resolve()


def _script_index(command: list[str]) -> int | None:
    executable = Path(command[0]).name
    python = re.fullmatch(
        r"(?:python|pypy)(?:[0-9]+(?:\.[0-9]+)*)?(?:\.exe)?",
        executable,
        re.IGNORECASE,
    )
    shell = re.fullmatch(r"(?:ba|da|k|z)?sh(?:\.exe)?", executable, re.IGNORECASE)
    if not (python or shell):
        return None
    index = 1
    while index < len(command):
        argument = command[index]
        if argument == "--":
            return index + 1 if index + 1 < len(command) else None
        if python and (argument in ("-c", "-m") or argument.startswith(("-c", "-m"))):
            return None
        if python and argument in ("-W", "-X", "--check-hash-based-pycs"):
            index += 2
            continue
        if shell and argument in ("-c", "--command"):
            return None
        if shell and argument in ("-O", "+O", "-o", "+o"):
            index += 2
            continue
        if argument.startswith("-"):
            index += 1
            continue
        return index
    return None


def _split_command(value: str, cwd: Path) -> list[str]:
    command = shlex.split(value)
    if not command:
        raise CodemapIdentityError("empty codemap command")
    executable = Path(command[0])
    if executable.is_absolute():
        command[0] = str(canonical(executable))
    elif "/" in command[0] or "\\" in command[0]:
        command[0] = str(canonical(cwd / executable))
    else:
        resolved = shutil.which(command[0])
        if not resolved:
            raise CodemapIdentityError(f"codemap executable not found: {command[0]}")
        command[0] = str(canonical(Path(resolved)))
    if not Path(command[0]).is_file():
        raise CodemapIdentityError(f"codemap executable not found: {command[0]}")
    script_index = _script_index(command)
    if script_index is not None:
        script = Path(command[script_index])
        script = canonical(script if script.is_absolute() else cwd / script)
        if not script.is_file():
            raise CodemapIdentityError(f"codemap interpreter wrapper not found: {script}")
        command[script_index] = str(script)
    return command


def resolve_codemap_command(
    explicit: str | list[str] | None,
    repo_root: Path,
    cwd: Path | None = None,
) -> tuple[list[str], str]:
    cwd = canonical(cwd or Path.cwd())
    if explicit:
        if isinstance(explicit, list):
            if not explicit or not all(isinstance(part, str) and part for part in explicit):
                raise CodemapIdentityError("codemap argv must be a non-empty string array")
            return _split_command(shlex.join(explicit), cwd), "explicit"
        return _split_command(explicit, cwd), "explicit"
    if os.environ.get("CODEMAP_BIN"):
        return _split_command(os.environ["CODEMAP_BIN"], cwd), "environment"
    for profile in ("debug", "release"):
        local = canonical(repo_root / "target" / profile / ("codemap.exe" if os.name == "nt" else "codemap"))
        if local.is_file():
            return [str(local)], f"local_target_{profile}"
    installed = shutil.which("codemap")
    if installed:
        return [str(canonical(Path(installed)))], "path"
    raise CodemapIdentityError("codemap binary not found; pass --codemap-bin")


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _executable_paths(command: list[str]) -> list[Path]:
    paths = [canonical(Path(command[0]))]
    script_index = _script_index(command)
    if script_index is not None:
        script = Path(command[script_index])
        if script.is_absolute():
            paths.append(canonical(script))
    return paths


def _primary_executable_path(command: list[str]) -> Path:
    paths = _executable_paths(command)
    return paths[-1]


def command_artifacts(command: list[str]) -> list[dict[str, str]]:
    artifacts: list[dict[str, str]] = []
    seen: set[Path] = set()
    for path in _executable_paths(command):
        if not path.is_file():
            continue
        if path in seen:
            continue
        seen.add(path)
        artifacts.append({"path": str(path), "sha256": _sha256(path)})
    return artifacts


def _run(command: list[str], cwd: Path, timeout: int) -> subprocess.CompletedProcess[str] | None:
    try:
        return subprocess.run(
            command,
            cwd=cwd,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None


def _version(command: list[str], cwd: Path) -> tuple[str, str | None]:
    result = _run([*command, "--version"], cwd, 15)
    if not result or result.returncode != 0:
        return "unknown", None
    output = (result.stdout or result.stderr).strip().splitlines()
    version_output = output[0] if output else "unknown"
    match = re.search(r"(?<![0-9])([0-9]+\.[0-9]+\.[0-9]+(?:[-+][A-Za-z0-9.-]+)?)", version_output)
    return version_output, match.group(1) if match else None


def _diagnostic_identity(command: list[str], cwd: Path) -> dict[str, Any] | None:
    result = _run([*command, "--root", str(cwd), "doctor", "--format", "json"], cwd, 120)
    if not result or result.returncode != 0:
        return None
    try:
        report = json.loads(result.stdout)
    except json.JSONDecodeError:
        return None
    identity = report.get("build_identity")
    return identity if isinstance(identity, dict) else None


def benchmark_binary_identity(
    command: list[str],
    resolution: str,
    diagnostic_root: Path,
) -> dict[str, Any]:
    diagnostic_root = canonical(diagnostic_root)
    artifacts = command_artifacts(command)
    version_output, semver = _version(command, diagnostic_root)
    diagnostic = _diagnostic_identity(command, diagnostic_root)
    primary_path = _primary_executable_path(command)
    primary_hash = _sha256(primary_path) if primary_path.is_file() else None
    if diagnostic is None:
        build_identity: dict[str, Any] = {
            "semver": semver or "unknown",
            "cache_format": "unknown",
            "schema_manifest_version": 0,
            "executable_path": str(primary_path),
            "binary_sha256": primary_hash,
            "binary_sha256_state": "computed" if primary_hash else "unavailable",
            "source_commit": None,
            "dirty_build": None,
        }
        diagnostic_state = "unavailable"
    else:
        build_identity = diagnostic
        diagnostic_state = "verified"
        if semver and build_identity.get("semver") != semver:
            raise CodemapIdentityError("codemap --version disagrees with doctor build identity")
        executable = canonical(Path(str(build_identity.get("executable_path", ""))))
        direct = len(command) == 1
        if direct and executable != canonical(Path(command[0])):
            raise CodemapIdentityError("doctor attributed a different codemap executable")
        exact_hash = _sha256(executable) if executable.is_file() else None
        if exact_hash != build_identity.get("binary_sha256"):
            raise CodemapIdentityError("doctor binary hash disagrees with executable bytes")
    return {
        "build_identity": build_identity,
        "diagnostic_state": diagnostic_state,
        "resolution": resolution,
        "command_argv": command,
        "command_artifacts": artifacts,
        "version_output": version_output,
    }
