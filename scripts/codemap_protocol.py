"""Parse codemap A/B invocation logs into a machine-checkable daily protocol."""

from __future__ import annotations

import os
import shlex
from typing import Any


FOCUSED_NAMES = {
    "ls",
    "cone",
    "where",
    "graph",
    "runtime",
    "contract",
    "flow",
    "boundary-map",
    "siblings",
    "place",
    "delete",
    "diff-map",
    "impact",
    "proof-map",
}
COMMAND_NAMES = FOCUSED_NAMES | {"changed", "proof"}
VALUE_OPTIONS = {
    "--depth",
    "--files",
    "--format",
    "--kind",
    "--lens",
    "--limit",
    "--path",
    "--root",
    "--section",
    "--since",
}
REQUIRED_FOCUS_ARGUMENT = {"cone", "where", "contract", "flow", "delete"}


def _words(invocation: str) -> list[str]:
    try:
        return shlex.split(invocation)
    except ValueError:
        return invocation.split()


def _command_index(args: list[str]) -> int | None:
    index = 0
    while index < len(args):
        value = args[index]
        if value == "--root":
            if index + 1 >= len(args):
                return None
            index += 2
            continue
        if value == "--brief" or value.startswith("--root="):
            index += 1
            continue
        return index if value in COMMAND_NAMES else None
    return None


def _positional_argument(args: list[str]) -> str | None:
    skip_value = False
    after_separator = False
    for value in args:
        if skip_value:
            skip_value = False
            continue
        if after_separator:
            return value
        if value == "--":
            after_separator = True
        elif value in VALUE_OPTIONS:
            skip_value = True
        elif value.startswith("-"):
            continue
        else:
            return value
    return None


def _effective_root(
    args: list[str], worktree: os.PathLike[str] | str | None
) -> str | None:
    selected = None
    for index, value in enumerate(args):
        if value == "--root" and index + 1 < len(args):
            selected = args[index + 1]
        elif value.startswith("--root="):
            selected = value.split("=", 1)[1]
    if selected is None:
        return os.path.realpath(worktree) if worktree is not None else None
    if os.path.isabs(selected):
        return os.path.realpath(selected)
    base = os.fspath(worktree) if worktree is not None else os.getcwd()
    return os.path.realpath(os.path.join(base, selected))


def _is_root_scope(argument: str | None, effective_root: str | None) -> bool:
    if argument is None or os.path.normpath(argument) == ".":
        return True
    if effective_root is None:
        return False
    candidate = argument if os.path.isabs(argument) else os.path.join(effective_root, argument)
    return os.path.realpath(candidate) == effective_root


def _call(
    index: int,
    args: list[str],
    worktree: os.PathLike[str] | str | None,
) -> tuple[int, str, str | None, str | None] | None:
    command_index = _command_index(args)
    if command_index is None:
        return None
    command = args[command_index]
    argument = _positional_argument(args[command_index + 1 :])
    entry = None
    if command == "ls":
        entry = "root" if _is_root_scope(argument, _effective_root(args, worktree)) else "exact"
    elif command == "cone" and argument is not None:
        entry = "root" if _is_root_scope(argument, _effective_root(args, worktree)) else "exact"
    elif command == "where" and argument is not None:
        entry = "exact"
    return index, command, argument, entry


def _record(raw: str | dict[str, Any]) -> dict[str, Any]:
    if isinstance(raw, str):
        argv, status = _words(raw), 0
    else:
        argv, status = raw.get("argv"), raw.get("status")
        if not isinstance(argv, list) or not all(isinstance(arg, str) for arg in argv):
            raise ValueError("codemap invocation argv must be a string array")
        if not isinstance(status, int):
            raise ValueError("codemap invocation status must be an integer")
    return {"argv": argv, "status": status, "display": shlex.join(argv)}


def codemap_protocol(
    mode: str,
    arm: str,
    invocations: list[str | dict[str, Any]],
    worktree: os.PathLike[str] | str | None = None,
) -> dict[str, Any]:
    records = [_record(raw) for raw in invocations]
    calls = [
        call
        for index, record in enumerate(records)
        if record["status"] == 0
        and (call := _call(index, record["argv"], worktree))
    ]
    entries = [(index, entry) for index, _, _, entry in calls if entry is not None]
    first = entries[0] if entries else None
    entry_is_first_invocation = bool(first and first[0] == 0)
    root_entry = any(entry == "root" for _, entry in entries)
    exact_entry = any(entry == "exact" for _, entry in entries)
    focused_calls = [
        index
        for index, command, argument, entry in calls
        if command in FOCUSED_NAMES
        and entry != "root"
        and not (command in REQUIRED_FOCUS_ARGUMENT and argument is None)
    ]
    focused_after_root = bool(
        first
        and first[1] == "root"
        and any(index > first[0] for index in focused_calls)
    )
    changed_calls = [index for index, command, _, _ in calls if command == "changed"]
    proof_changed_calls = [
        index
        for index, command, argument, _ in calls
        if command == "proof" and argument == "changed"
    ]
    ordered_daily = bool(
        first
        and any(
            first[0] < changed_call < proof_call
            for changed_call in changed_calls
            for proof_call in proof_changed_calls
        )
    )
    if arm == "control":
        compliant = not invocations
    elif mode == "analysis":
        compliant = bool(
            entry_is_first_invocation and (first[1] == "exact" or focused_after_root)
        )
    else:
        compliant = bool(entry_is_first_invocation and ordered_daily)
    return {
        "invocation_count": len(records),
        "successful_invocation_count": sum(record["status"] == 0 for record in records),
        "failed_invocation_count": sum(record["status"] != 0 for record in records),
        "invocations": [record["display"] for record in records],
        "invocation_results": records,
        "first_entry": records[first[0]]["display"] if first else None,
        "entry_is_first_invocation": entry_is_first_invocation,
        "entry_kind": first[1] if first else "none",
        "root_entry": root_entry,
        "exact_entry": exact_entry,
        "mixed": root_entry and exact_entry,
        "root_ls": root_entry,
        "changed": bool(changed_calls),
        "proof_changed": bool(proof_changed_calls),
        "ordered_daily": ordered_daily,
        "focused": bool(focused_calls),
        "focused_after_root": focused_after_root,
        "compliant": compliant,
    }
