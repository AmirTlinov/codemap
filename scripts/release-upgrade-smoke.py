#!/usr/bin/env python3
"""Exercise one shared external cache across upgrade and downgrade order."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path


def run(argv: list[str], cwd: Path, env: dict[str, str]) -> str:
    result = subprocess.run(argv, cwd=cwd, env=env, capture_output=True, text=True, check=False)
    if result.returncode not in {0, 10}:
        raise ValueError(f"command failed ({result.returncode}): {' '.join(argv)}\n{result.stderr}")
    return result.stdout


def tree_digest(root: Path) -> str:
    digest = hashlib.sha256()
    for path in sorted(item for item in root.rglob("*") if item.is_file()):
        relative = path.relative_to(root).as_posix()
        digest.update(relative.encode())
        digest.update(path.read_bytes())
    return digest.hexdigest()


def schema_manifest(binary: Path, repo: Path, env: dict[str, str]) -> dict:
    return json.loads(run([str(binary), "schema", "manifest"], repo, env))


def assert_schema_forward(previous: dict, current: dict) -> None:
    if current["version"] < previous["version"]:
        raise ValueError("schema manifest version regressed")
    old = {row["kind"]: row for row in previous.get("schemas", [])}
    new = {row["kind"]: row for row in current.get("schemas", [])}
    missing = set(old) - set(new)
    if missing:
        raise ValueError(f"current release removed schema kinds: {sorted(missing)}")
    for kind in old:
        if "schema_version" not in old[kind]:
            if "schema_version" in new[kind]:
                continue
            if old[kind].get("file") != new[kind].get("file"):
                raise ValueError(f"{kind} unversioned schema owner changed")
            continue
        before = int(old[kind]["schema_version"])
        after = int(new[kind]["schema_version"])
        if after < before:
            raise ValueError(f"{kind} schema version regressed: {before} -> {after}")


def exercise(previous: Path, current: Path) -> dict:
    with tempfile.TemporaryDirectory(prefix="codemap-upgrade-smoke-") as temporary:
        root = Path(temporary)
        repo, cache = root / "repo", root / "cache"
        repo.mkdir()
        (repo / "src").mkdir()
        (repo / "src/lib.rs").write_text("pub fn release_smoke() {}\n", encoding="utf-8")
        subprocess.run(["git", "init", "-q", repo], check=True)
        subprocess.run(["git", "-C", repo, "add", "."], check=True)
        subprocess.run(
            [
                "git", "-C", repo, "-c", "user.name=release smoke",
                "-c", "user.email=release@codemap.invalid", "commit", "-qm", "fixture",
            ],
            check=True,
        )
        before = tree_digest(repo)
        env = {**os.environ, "CODEMAP_CACHE_DIR": str(cache)}
        sequence = [previous, current, previous, current]
        receipts = []
        for binary in sequence:
            version = run([str(binary), "--version"], repo, env).strip()
            run([str(binary), "doctor", "--format", "json"], repo, env)
            run([str(binary), "ls", ".", "--format", "json"], repo, env)
            receipts.append({"binary": str(binary), "version": version})
        after = tree_digest(repo)
        if before != after or subprocess.run(
            ["git", "-C", repo, "status", "--porcelain"], capture_output=True, text=True, check=True
        ).stdout:
            raise ValueError("upgrade/downgrade smoke changed the target repository")
        assert_schema_forward(
            schema_manifest(previous, repo, env), schema_manifest(current, repo, env)
        )
        return {
            "kind": "codemap_release_upgrade_smoke",
            "sequence": receipts,
            "shared_cache": str(cache),
            "target_repo_unchanged": True,
            "schema_forward_compatible": True,
        }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--previous", required=True)
    parser.add_argument("--current", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        receipt = exercise(Path(args.previous).resolve(), Path(args.current).resolve())
        Path(args.output).write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n")
        return 0
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as exc:
        print(f"codemap upgrade smoke: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
