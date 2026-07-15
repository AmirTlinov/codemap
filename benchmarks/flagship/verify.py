#!/usr/bin/env python3
"""Execute one frozen external criterion without exposing its oracle to the agent."""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any


CITATION = re.compile(
    r"(?<![A-Za-z0-9_.-])((?:[A-Za-z0-9_.@()\[\] -]+/)+[A-Za-z0-9_.@()\[\] -]+):(\d+)"
)


def fail(message: str, receipt: dict[str, Any] | None = None) -> int:
    body = {"passed": False, "error": message, **(receipt or {})}
    print(json.dumps(body, ensure_ascii=False, indent=2))
    return 1


def citation_receipt(message: Path, worktree: Path) -> dict[str, Any]:
    text = message.read_text(encoding="utf-8") if message.is_file() else ""
    valid: list[tuple[str, int]] = []
    invalid: list[tuple[str, int]] = []
    for raw_path, raw_line in CITATION.findall(text):
        path = raw_path.strip()
        candidate = worktree / path
        line = int(raw_line)
        if candidate.is_file():
            count = sum(1 for _ in candidate.open(encoding="utf-8", errors="replace"))
            (valid if 1 <= line <= max(count, 1) else invalid).append((path, line))
        else:
            invalid.append((path, line))
    unique = sorted({path for path, _ in valid})
    top = sorted({path.split("/", 1)[0] for path in unique})
    return {
        "characters": len(text),
        "valid_citations": len(valid),
        "unique_valid_paths": len(unique),
        "top_level_surfaces": top,
        "invalid_citations": invalid,
    }


def verify_analysis(action: dict[str, Any], message: Path, worktree: Path) -> tuple[bool, dict]:
    receipt = citation_receipt(message, worktree)
    valid = receipt["valid_citations"]
    invalid = len(receipt["invalid_citations"])
    passed = (
        receipt["characters"] >= action.get("min_characters", 1800)
        and valid >= action.get("min_citations", 8)
        and receipt["unique_valid_paths"] >= action.get("min_paths", 6)
        and len(receipt["top_level_surfaces"]) >= action.get("min_surfaces", 2)
        and invalid <= action.get("max_invalid", max(1, valid // 5))
    )
    return passed, receipt


def verify_head(action: dict[str, Any], worktree: Path) -> tuple[bool, dict]:
    result = subprocess.run(
        ["git", "-C", str(worktree), "rev-parse", "HEAD"], capture_output=True, text=True
    )
    actual = result.stdout.strip()
    expected = action["commit"]
    return result.returncode == 0 and actual == expected, {"expected": expected, "actual": actual}


def verify_files(action: dict[str, Any], worktree: Path) -> tuple[bool, dict]:
    errors = []
    for relative in action.get("exists", []):
        if not (worktree / relative).is_file():
            errors.append(f"missing:{relative}")
    for relative, needles in action.get("contains", {}).items():
        path = worktree / relative
        body = path.read_text(encoding="utf-8", errors="replace") if path.is_file() else ""
        for needle in needles:
            if needle not in body:
                errors.append(f"missing-text:{relative}:{needle}")
    for relative, needles in action.get("not_contains", {}).items():
        path = worktree / relative
        body = path.read_text(encoding="utf-8", errors="replace") if path.is_file() else ""
        for needle in needles:
            if needle in body:
                errors.append(f"forbidden-text:{relative}:{needle}")
    return not errors, {"errors": errors}


def expand(value: str, worktree: Path) -> str:
    return value.replace("{worktree}", str(worktree))


def apply_overlays(action: dict[str, Any], worktree: Path) -> list[str]:
    copied = []
    for row in action.get("overlays", []):
        source = Path(row["source"])
        target = worktree / row["target"]
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)
        copied.append(row["target"])
    return copied


def run_commands(action: dict[str, Any], worktree: Path) -> tuple[bool, dict]:
    copied = apply_overlays(action, worktree)
    receipts = []
    for command in action.get("commands", []):
        argv = [expand(part, worktree) for part in command["argv"]]
        cwd = worktree / command.get("cwd", ".")
        env = os.environ.copy()
        env.update({key: expand(value, worktree) for key, value in command.get("env", {}).items()})
        try:
            result = subprocess.run(
                argv,
                cwd=cwd,
                env=env,
                capture_output=True,
                text=True,
                timeout=command.get("timeout_seconds", 600),
            )
        except subprocess.TimeoutExpired:
            return False, {"overlays": copied, "commands": receipts, "timeout": argv}
        expected = command.get("expected_status", 0)
        status_ok = result.returncode != 0 if expected == "nonzero" else result.returncode == expected
        stdout_ok = all(value in result.stdout for value in command.get("stdout_contains", []))
        stderr_ok = all(value in result.stderr for value in command.get("stderr_contains", []))
        receipt = {
            "argv": argv,
            "status": result.returncode,
            "stdout_tail": result.stdout[-2000:],
            "stderr_tail": result.stderr[-2000:],
            "passed": status_ok and stdout_ok and stderr_ok,
        }
        receipts.append(receipt)
        if not receipt["passed"]:
            return False, {"overlays": copied, "commands": receipts}
    return True, {"overlays": copied, "commands": receipts}


def verify(action: dict[str, Any], message: Path, worktree: Path) -> tuple[bool, dict]:
    kind = action["kind"]
    if kind == "analysis":
        return verify_analysis(action, message, worktree)
    if kind == "git_head":
        return verify_head(action, worktree)
    if kind == "files":
        return verify_files(action, worktree)
    if kind == "commands":
        return run_commands(action, worktree)
    raise ValueError(f"unknown verifier action: {kind}")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("spec")
    parser.add_argument("task_id")
    parser.add_argument("criterion")
    parser.add_argument("worktree")
    parser.add_argument("last_message")
    args = parser.parse_args(argv)
    spec = json.loads(Path(args.spec).read_text(encoding="utf-8"))
    try:
        action = spec["tasks"][args.task_id][args.criterion]
        passed, receipt = verify(action, Path(args.last_message), Path(args.worktree))
    except (KeyError, OSError, ValueError, json.JSONDecodeError) as exc:
        return fail(str(exc))
    print(json.dumps({"passed": passed, **receipt}, ensure_ascii=False, indent=2))
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
