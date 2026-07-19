"""Describe codemap activity in an A/B trajectory without judging the agent."""

from __future__ import annotations

import os
import shlex
from typing import Any


MAP_COMMANDS = {
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
COMMAND_NAMES = MAP_COMMANDS | {"changed", "proof", "doctor", "status"}
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


def _words(invocation: str) -> list[str]:
    try:
        return shlex.split(invocation)
    except ValueError:
        return invocation.split()


def _shell_words(script: str) -> list[str]:
    try:
        lexer = shlex.shlex(script, posix=True, punctuation_chars=";&|()\n")
        # A newline is a shell command boundary, not disposable whitespace. Keeping
        # it as punctuation lets one completed Codex event account for every
        # command in a multi-line `sh -lc` payload.
        lexer.whitespace = " \t\r"
        lexer.whitespace_split = True
        return list(lexer)
    except ValueError:
        return script.split()


def agent_codemap_commands(commands: list[str]) -> list[str]:
    observed = []
    separators = {";", "&&", "||", "|", "&", "(", ")", "\n"}
    prefixes = {"command", "exec", "time", "env"}
    for command in commands:
        outer = _words(command)
        script = command
        if outer and os.path.basename(outer[0]) in {"bash", "dash", "fish", "sh", "zsh"}:
            for flag in ("-lc", "-c"):
                if flag in outer and outer.index(flag) + 1 < len(outer):
                    script = outer[outer.index(flag) + 1]
                    break
        words = _shell_words(script)
        command_start = True
        for word in words:
            if word in separators:
                command_start = True
                continue
            if not command_start:
                continue
            if word in prefixes or ("=" in word and not word.startswith("/")):
                continue
            if os.path.basename(word) == "codemap":
                observed.append(word)
            command_start = False
    return observed


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
) -> dict[str, Any] | None:
    command_index = _command_index(args)
    if command_index is None:
        return None
    command = args[command_index]
    argument = _positional_argument(args[command_index + 1 :])
    scope_kind = None
    if command == "ls":
        scope_kind = (
            "current_level" if _is_root_scope(argument, _effective_root(args, worktree)) else "scoped"
        )
    elif command == "cone" and argument is not None:
        scope_kind = (
            "current_level" if _is_root_scope(argument, _effective_root(args, worktree)) else "scoped"
        )
    elif command == "where" and argument is not None:
        scope_kind = "symbol"
    return {
        "invocation_index": index,
        "command": command,
        "argument": argument,
        "scope_kind": scope_kind,
    }


def _record(raw: str | dict[str, Any]) -> dict[str, Any]:
    if isinstance(raw, str):
        argv, status, direct = _words(raw), 0, True
    else:
        argv, status = raw.get("argv"), raw.get("status")
        direct = raw.get("agent_direct", True)
        if not isinstance(argv, list) or not all(isinstance(arg, str) for arg in argv):
            raise ValueError("codemap invocation argv must be a string array")
        if not isinstance(status, int):
            raise ValueError("codemap invocation status must be an integer")
        if not isinstance(direct, bool):
            raise ValueError("codemap invocation agent_direct must be boolean")
    return {
        "argv": argv,
        "status": status,
        "agent_direct": direct,
        "display": shlex.join(argv),
    }


def codemap_activity(
    invocations: list[str | dict[str, Any]],
    worktree: os.PathLike[str] | str | None = None,
    agent_commands: list[str] | None = None,
) -> dict[str, Any]:
    all_records = [_record(raw) for raw in invocations]
    records = [record for record in all_records if record["agent_direct"]]
    internal = [record for record in all_records if not record["agent_direct"]]
    calls = []
    for index, record in enumerate(records):
        call = _call(index, record["argv"], worktree)
        if call is None:
            continue
        calls.append(
            {
                **call,
                "status": record["status"],
                "succeeded": record["status"] == 0,
                "argv": record["argv"],
                "display": record["display"],
            }
        )
    observed_commands = agent_codemap_commands(agent_commands or [])
    trace_matches = agent_commands is None or len(observed_commands) == len(records)
    return {
        "invocation_count": len(records),
        "successful_invocation_count": sum(record["status"] == 0 for record in records),
        "failed_invocation_count": sum(record["status"] != 0 for record in records),
        "invocations": [record["display"] for record in records],
        "invocation_results": records,
        "ignored_internal_invocation_count": len(internal),
        "internal_invocation_results": internal,
        "agent_command_invocations": observed_commands,
        "agent_command_trace_matches": trace_matches,
        "calls": calls,
    }
