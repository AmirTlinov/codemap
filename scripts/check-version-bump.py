#!/usr/bin/env python3
"""Require a higher Cargo package version whenever repository bytes changed."""

from __future__ import annotations

import os
import re
import subprocess
import sys
from pathlib import Path


def git(*args: str, check: bool = False) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args], capture_output=True, text=True, check=check
    )


def commit(value: str) -> str | None:
    result = git("rev-parse", "--verify", f"{value}^{{commit}}")
    return result.stdout.strip() if result.returncode == 0 else None


def release_tag_base() -> str | None:
    result = git("tag", "--merged", "HEAD", "--list", "v*")
    if result.returncode != 0:
        return None
    candidates: list[tuple[tuple[int, int, int], str]] = []
    for tag in result.stdout.splitlines():
        try:
            candidates.append((semver(tag.removeprefix("v")), tag))
        except ValueError:
            continue
    if not candidates:
        return None
    return commit(max(candidates)[1])


def resolve_base(requested: str | None) -> str | None:
    if requested:
        resolved = commit(requested)
        if not resolved:
            raise ValueError(f"base ref {requested!r} is not a commit")
        return resolved
    return release_tag_base()


def version_from_text(text: str) -> str | None:
    match = re.search(r'^version\s*=\s*"([^"]+)"', text, re.MULTILINE)
    return match.group(1) if match else None


def semver(value: str) -> tuple[int, int, int]:
    match = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)(?:[-+].*)?", value)
    if not match:
        raise ValueError(f"version {value!r} is not semver x.y.z")
    return tuple(map(int, match.groups()))


def changed_files(base: str) -> list[str]:
    commands = [
        ("diff", "--name-only", f"{base}...HEAD"),
        ("diff", "--cached", "--name-only"),
        ("diff", "--name-only"),
        ("ls-files", "--others", "--exclude-standard"),
    ]
    rows = set()
    for command in commands:
        result = git(*command)
        if result.returncode == 0:
            rows.update(result.stdout.splitlines())
    return sorted(row for row in rows if row)


def main(argv: list[str]) -> int:
    try:
        cargo = Path("Cargo.toml")
        if not cargo.is_file():
            raise ValueError("Cargo.toml not found; run from the repository root")
        base = resolve_base(argv[0] if argv else os.environ.get("CODEMAP_VERSION_BASE"))
        if not base:
            print("codemap version guard: no release tag found; skipping", file=sys.stderr)
            return 0
        current = version_from_text(cargo.read_text(encoding="utf-8"))
        base_file = git("show", f"{base}:Cargo.toml")
        previous = version_from_text(base_file.stdout) if base_file.returncode == 0 else None
        if not current:
            raise ValueError("Cargo.toml package version is missing")
        if not previous:
            raise ValueError(f"base Cargo.toml package version is missing at {base}")
        changed = changed_files(base)
        if not changed:
            print(f"codemap version guard: no changed files since {base}", file=sys.stderr)
            return 0
        if semver(current) <= semver(previous):
            print("\n".join(changed), file=sys.stderr)
            raise ValueError(
                f"changed files require Cargo.toml package version bump: {previous} -> {current}"
            )
        print(f"codemap version guard: version bump ok: {previous} -> {current}", file=sys.stderr)
        return 0
    except (OSError, ValueError) as error:
        print(f"codemap version guard: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
