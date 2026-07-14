#!/usr/bin/env python3
"""Check the repository's mechanical 400-line code-file ratchet."""

from __future__ import annotations

import os
import sys
from pathlib import Path
from typing import Iterable


MAX_CODE_LINES = 400
CODE_EXTENSIONS = {
    ".bash",
    ".c",
    ".cc",
    ".cpp",
    ".cs",
    ".go",
    ".h",
    ".hpp",
    ".java",
    ".js",
    ".jsx",
    ".kt",
    ".kts",
    ".lua",
    ".php",
    ".py",
    ".rb",
    ".rs",
    ".scala",
    ".sh",
    ".sql",
    ".swift",
    ".ts",
    ".tsx",
    ".zsh",
}
EXCLUDED_DIRS = {
    ".git",
    ".venv",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "target",
    "vendor",
    "venv",
}
GENERATED_MARKERS = ("@generated", "code generated", "do not edit")


def repository_root() -> Path:
    return Path(__file__).resolve().parents[2]


def is_code_file(path: Path) -> bool:
    return (
        path.is_file()
        and not path.is_symlink()
        and path.suffix.lower() in CODE_EXTENSIONS
        and not any(part in EXCLUDED_DIRS for part in path.parts)
    )


def is_generated(text: str) -> bool:
    header = "\n".join(text.splitlines()[:10]).lower()
    return any(marker in header for marker in GENERATED_MARKERS)


def iter_code_files(root: Path) -> Iterable[Path]:
    for directory, dirs, files in os.walk(root):
        dirs[:] = sorted(name for name in dirs if name not in EXCLUDED_DIRS)
        base = Path(directory)
        for name in sorted(files):
            path = base / name
            if is_code_file(path):
                yield path


def relative(root: Path, path: Path) -> str:
    return path.relative_to(root).as_posix()


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def legacy_baseline(root: Path) -> dict[str, int]:
    path = root / ".codex" / "legacy-oversize.tsv"
    result = {}
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        name, count = line.split("\t", 1)
        result[name] = int(count)
    return result


def current_line_counts(root: Path) -> dict[str, int]:
    current: dict[str, int] = {}
    for path in iter_code_files(root):
        text = read_text(path)
        if is_generated(text):
            continue
        current[relative(root, path)] = len(text.splitlines())
    return current


def validate_counts(current: dict[str, int], baseline: dict[str, int]) -> list[str]:
    violations = []
    for name, lines in sorted(current.items()):
        recorded = baseline.get(name)
        if lines > MAX_CODE_LINES and recorded is None:
            violations.append(
                f"{name}: {lines} lines; new code files must stay at or below "
                f"{MAX_CODE_LINES}"
            )
        elif lines > MAX_CODE_LINES and lines > recorded:
            violations.append(
                f"{name}: {lines} lines exceeds its legacy debt ceiling of {recorded}"
            )
    for name, recorded in sorted(baseline.items()):
        lines = current.get(name)
        if lines is None:
            violations.append(f"{name}: remove stale legacy baseline {recorded}")
        elif lines <= MAX_CODE_LINES:
            violations.append(f"{name}: now {lines} lines; remove its legacy exemption")
    return violations


def validate_ratchet(root: Path) -> list[str]:
    return validate_counts(current_line_counts(root), legacy_baseline(root))


def violation_message(violations: list[str]) -> str:
    shown = violations[:25]
    extra = len(violations) - len(shown)
    body = "\n".join(f"- {item}" for item in shown)
    if extra:
        body += f"\n- ... and {extra} more"
    return (
        "Repository 400-line code budget failed:\n"
        f"{body}\n"
        "Split a new oversize file before finishing. Legacy files may shrink "
        "gradually, but may not exceed their recorded debt ceiling."
    )


def self_test() -> int:
    assert not validate_counts({"small.rs": 400}, {})
    assert "new code files" in "\n".join(validate_counts({"large.rs": 401}, {}))
    assert not validate_counts({"legacy.rs": 500}, {"legacy.rs": 600})
    assert "debt ceiling" in "\n".join(
        validate_counts({"legacy.rs": 601}, {"legacy.rs": 600})
    )
    assert "remove its legacy exemption" in "\n".join(
        validate_counts({"legacy.rs": 400}, {"legacy.rs": 600})
    )
    assert "remove stale legacy baseline" in "\n".join(
        validate_counts({}, {"deleted.rs": 600})
    )
    assert is_generated("// @generated\n" + "x\n" * 500)
    print("code policy self-test: ok")
    return 0


def main() -> int:
    action = sys.argv[1] if len(sys.argv) > 1 else ""
    if action == "check-all":
        violations = validate_ratchet(repository_root())
        if violations:
            print(violation_message(violations), file=sys.stderr)
            return 1
        print("code policy ratchet: ok")
        return 0
    if action == "self-test":
        return self_test()
    print("usage: code_policy.py check-all|self-test", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
