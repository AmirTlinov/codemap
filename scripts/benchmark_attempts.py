"""Own the two-attempt infrastructure-failure lifecycle for A/B trials."""

from __future__ import annotations

import json
import os
import shutil
from pathlib import Path
from typing import Any


INFRASTRUCTURE_FAILURES = frozenset({"codex_timeout", "codex_crash", "verifier_timeout"})


def infrastructure_failure(result: dict[str, Any]) -> bool:
    return result.get("invalidation_reason") in INFRASTRUCTURE_FAILURES


def current_attempt(path: Path) -> int:
    return 2 if (path / "attempts" / "attempt-1" / "result.json").is_file() else 1


def _rewrite_paths(value: Any, source: str, target: str) -> Any:
    if isinstance(value, dict):
        return {key: _rewrite_paths(item, source, target) for key, item in value.items()}
    if isinstance(value, list):
        return [_rewrite_paths(item, source, target) for item in value]
    if isinstance(value, str) and (value == source or value.startswith(source + os.sep)):
        return target + value[len(source) :]
    return value


def archive_first_attempt(path: Path) -> Path:
    target = path / "attempts" / "attempt-1"
    if target.exists():
        raise ValueError(f"infrastructure retry already archived: {target}")
    target.mkdir(parents=True)
    for child in list(path.iterdir()):
        if child.name != "attempts":
            shutil.move(str(child), target / child.name)
    result_path = target / "result.json"
    if not result_path.is_file():
        raise ValueError(f"infrastructure attempt has no result: {result_path}")
    result = json.loads(result_path.read_text(encoding="utf-8"))
    result = _rewrite_paths(result, str(path), str(target))
    result_path.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return target


def retry_infrastructure_failure(path: Path, result: dict[str, Any]) -> bool:
    attempt = result.get("infrastructure_attempt", 1)
    if not infrastructure_failure(result) or attempt != 1:
        return False
    archive_first_attempt(path)
    return True


def _clear_incomplete_attempt(path: Path, preserve_first: bool) -> None:
    if not preserve_first:
        shutil.rmtree(path)
        return
    for child in path.iterdir():
        if child.name != "attempts":
            shutil.rmtree(child) if child.is_dir() else child.unlink()


def existing_trial(path: Path, fingerprint: str, resume: bool) -> dict[str, Any] | None:
    result_path = path / "result.json"
    if not result_path.exists():
        if path.exists():
            preserved_first = current_attempt(path) == 2
            current_artifacts = any(child.name != "attempts" for child in path.iterdir())
            if current_artifacts and not resume:
                raise ValueError(f"incomplete trial exists; use --resume or another --out-dir: {path}")
            if current_artifacts or not preserved_first:
                if not resume:
                    raise ValueError(
                        f"incomplete trial exists; use --resume or another --out-dir: {path}"
                    )
                _clear_incomplete_attempt(path, preserved_first)
        return None
    if not resume:
        raise ValueError(f"trial already exists; use --resume or another --out-dir: {path}")
    result = json.loads(result_path.read_text(encoding="utf-8"))
    if result.get("trial_fingerprint") != fingerprint:
        raise ValueError(f"cannot resume trial with a different configuration: {path}")
    return None if retry_infrastructure_failure(path, result) else result
