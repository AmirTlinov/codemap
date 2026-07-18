"""Immutable file inventory for behavioral evidence receipts."""

from __future__ import annotations

from pathlib import Path
from typing import Iterable

from flagship_manifest import file_sha256


def artifact_inventory(roots: Iterable[Path], files: Iterable[Path] = ()) -> list[dict[str, str]]:
    candidates: set[Path] = set()
    for root in roots:
        if not root.is_dir():
            raise ValueError(f"evidence directory is missing: {root}")
        candidates.update(
            path.resolve()
            for path in root.rglob("*")
            if path.is_file() and "codemap-cache" not in path.parts
        )
    for path in files:
        if not path.is_file():
            raise ValueError(f"evidence file is missing: {path}")
        candidates.add(path.resolve())
    return [
        {"path": str(path), "sha256": file_sha256(path)}
        for path in sorted(candidates, key=lambda item: str(item))
    ]
