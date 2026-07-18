#!/usr/bin/env python3
"""Run a paired behavioral A/B benchmark with Codex, with and without codemap.

Each task is executed from the same git commit in two disposable worktrees. The
model, reasoning effort, task text, sandbox, and deterministic verifier are held
constant. Only the navigation arm changes: the treatment must use codemap's
daily workflow, while the control blocks agent-attributed codemap calls.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import shlex
import shutil
import statistics
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from benchmark_parallel import ProcessResult, run_ordered, run_process
from benchmark_attempts import (
    current_attempt,
    existing_trial,
    retry_infrastructure_failure,
)
import benchmark_codex_runtime as codex_runtime
from benchmark_worktrees import add_worktree, remove_worktree
from codemap_identity import CodemapIdentityError, benchmark_binary_identity, command_artifacts, resolve_codemap_command
from codemap_protocol import codemap_protocol
from codemap_protocol_shim import shell_profile_environment, write_shim


ARM_CONTROL = "control"
ARM_TREATMENT = "codemap"
ARMS = (ARM_CONTROL, ARM_TREATMENT)
MODE_IMPLEMENTATION = "implementation"
MODE_ANALYSIS = "analysis"
TASK_MODES = (MODE_IMPLEMENTATION, MODE_ANALYSIS)
PROMPT_PROTOCOL_VERSION = 15

COMMON_PROMPT = """You are completing one benchmark coding task in a disposable git worktree.
Make the smallest complete implementation that satisfies the task. Work autonomously; do not ask
questions. Do not weaken or rewrite tests to manufacture a pass. Finish with a concise summary.
"""

ANALYSIS_COMMON_PROMPT = """You are completing one benchmark repository-analysis task in a
disposable git worktree. Investigate autonomously and report only claims supported by repository
evidence. Cite concrete paths and line numbers or symbols. Distinguish confirmed problems from
hypotheses. Do not modify any repository file. In the final report, cite relative `path:line`
evidence and do not mention navigation commands or tools. Finish with a concise, self-contained report.
"""

ARM_PROMPTS = {
    ARM_CONTROL: """CONTROL ARM: codemap is unavailable. Do not attempt to use it. Navigate with
ordinary repository tools only.
""",
    ARM_TREATMENT: """CODEMAP TREATMENT ARM: before ordinary inspection, use one proportionate entry:
`codemap cone <file-or-file#symbol>` for a task-named file, `codemap where <symbol>` when only a
symbol is known, or `codemap ls <directory>` for a named directory. Use `codemap ls .` only when
scope is unknown. If a task-named path does not exist yet, map its nearest existing parent; otherwise
never replace an exact file with its parent directory. Read the relevant linked source.
Use a printed exact Expand only when its hidden or unknown evidence matters to the task.
After editing, run `codemap changed && codemap proof changed` once, then the task-specific check.
Do not run broad repository gates.""",
}

ANALYSIS_ARM_PROMPTS = {
    ARM_CONTROL: """CONTROL ARM: codemap is unavailable. Do not attempt to use it. Navigate with
ordinary read-only repository tools only.
""",
    ARM_TREATMENT: """CODEMAP TREATMENT ARM: begin with one proportionate structural map: `codemap cone
<file-or-file#symbol>` for a task-named file, `codemap where <symbol>` when only a symbol is known,
or `codemap ls <directory>` for a named directory. Use `codemap ls .` only when scope is unknown;
never replace an exact file with its parent directory. Read the relevant linked source for line
evidence. Use a printed exact Expand only when its hidden or unknown evidence matters to the task.
Do not edit the repository.
""",
}

EXACT_COMMON_PROMPT = """EXACT TASK CONTACT: the task already fixes the file and exact bytes, so there
is no repository-navigation uncertainty. Apply the replacement directly; verify only resulting bytes."""
EXACT_ARM_PROMPTS = {
    ARM_CONTROL: "CONTROL ARM: codemap is unavailable. Do not attempt to use it.",
    ARM_TREATMENT: "CODEMAP TREATMENT ARM: codemap is available; the exact task requires no map call.",
}

@dataclass(frozen=True)
class Verifier:
    name: str
    command: list[str]
    timeout_seconds: int
    category: str
    weight: float
    required: bool


@dataclass(frozen=True)
class Task:
    task_id: str
    mode: str
    repo: Path
    base_ref: str
    base_commit: str
    prompt: str
    verifiers: list[Verifier]
    task_class: str | None = None


def canonical(path: Path) -> Path:
    return path.expanduser().resolve()


def safe_label(value: str) -> str:
    return re.sub(r"[^A-Za-z0-9_.-]+", "_", value).strip("_") or "task"


def stable_hash(value: Any) -> str:
    body = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(body.encode("utf-8")).hexdigest()


def git(repo: Path, args: list[str], timeout_seconds: int = 60) -> ProcessResult:
    return run_process(["git", "-C", str(repo), *args], repo, timeout_seconds)


def require_git(repo: Path, args: list[str], description: str) -> str:
    result = git(repo, args)
    if result.status != 0:
        raise ValueError(f"{description}: {result.stderr.strip() or result.stdout.strip()}")
    return result.stdout.strip()


def resolve_task_path(raw: str, tasks_path: Path) -> Path:
    path = Path(raw).expanduser()
    if not path.is_absolute():
        path = tasks_path.parent / path
    return canonical(path)


def validate_relative_path(value: str, task_id: str) -> str:
    path = Path(value)
    if path.is_absolute() or ".." in path.parts or value in {"", "."}:
        raise ValueError(f"task {task_id}: protected path must be repo-relative: {value!r}")
    return path.as_posix().rstrip("/")


def load_tasks(path: Path, default_verifier_timeout: int) -> list[Task]:
    tasks: list[Task] = []
    seen: set[str] = set()
    seen_labels: dict[str, str] = {}
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        try:
            raw = json.loads(line)
        except json.JSONDecodeError as exc:
            raise ValueError(f"{path}:{line_number}: invalid JSON: {exc}") from exc
        if not isinstance(raw, dict):
            raise ValueError(f"{path}:{line_number}: task must be a JSON object")
        task_id = raw.get("id")
        mode = raw.get("mode", MODE_IMPLEMENTATION)
        prompt = raw.get("prompt")
        repo_raw = raw.get("repo")
        if not isinstance(task_id, str) or not task_id.strip():
            raise ValueError(f"{path}:{line_number}: task id must be a non-empty string")
        if mode not in TASK_MODES:
            raise ValueError(
                f"task {task_id or line_number}: mode must be one of {', '.join(TASK_MODES)}"
            )
        if task_id in seen:
            raise ValueError(f"{path}:{line_number}: duplicate task id {task_id!r}")
        label = safe_label(task_id)
        if label in seen_labels:
            raise ValueError(
                f"{path}:{line_number}: task ids {seen_labels[label]!r} and {task_id!r} "
                f"collide as artifact label {label!r}"
            )
        if not isinstance(prompt, str) or not prompt.strip():
            raise ValueError(f"task {task_id}: prompt must be a non-empty string")
        if not isinstance(repo_raw, str) or not repo_raw.strip():
            raise ValueError(f"task {task_id}: repo must be a path string")
        repo = resolve_task_path(repo_raw, path)
        if not repo.is_dir():
            raise ValueError(f"task {task_id}: repository does not exist: {repo}")
        base_ref = raw.get("base_ref", "HEAD")
        if not isinstance(base_ref, str) or not base_ref.strip():
            raise ValueError(f"task {task_id}: base_ref must be a non-empty string")
        require_git(repo, ["rev-parse", "--is-inside-work-tree"], f"task {task_id} is not git")
        base_commit = require_git(
            repo,
            ["rev-parse", "--verify", f"{base_ref}^{{commit}}"],
            f"task {task_id} base_ref",
        )

        raw_verifiers = raw.get("verify")
        if not isinstance(raw_verifiers, list) or not raw_verifiers:
            raise ValueError(f"task {task_id}: verify must contain at least one verifier")
        verifiers: list[Verifier] = []
        for index, verifier in enumerate(raw_verifiers, 1):
            if not isinstance(verifier, dict):
                raise ValueError(f"task {task_id}: verifier {index} must be an object")
            command = verifier.get("command")
            if (
                not isinstance(command, list)
                or not command
                or not all(isinstance(part, str) and part for part in command)
            ):
                raise ValueError(f"task {task_id}: verifier {index} command must be argv strings")
            name = verifier.get("name", f"verify-{index}")
            timeout = verifier.get("timeout_seconds", default_verifier_timeout)
            category = verifier.get("category", "behavior")
            weight = verifier.get("weight", 1.0)
            required = verifier.get("required", True)
            if not isinstance(name, str) or not name.strip():
                raise ValueError(f"task {task_id}: verifier {index} name must be a string")
            if not isinstance(timeout, int) or timeout <= 0:
                raise ValueError(f"task {task_id}: verifier {index} timeout must be positive")
            if not isinstance(category, str) or not category.strip():
                raise ValueError(f"task {task_id}: verifier {index} category must be a string")
            if (
                isinstance(weight, bool)
                or not isinstance(weight, (int, float))
                or not math.isfinite(weight)
                or weight <= 0
            ):
                raise ValueError(f"task {task_id}: verifier {index} weight must be positive")
            if not isinstance(required, bool):
                raise ValueError(f"task {task_id}: verifier {index} required must be boolean")
            verifiers.append(
                Verifier(
                    name=name,
                    command=command,
                    timeout_seconds=timeout,
                    category=category.strip(),
                    weight=float(weight),
                    required=required,
                )
            )

        benchmark = raw.get("benchmark")
        task_class = benchmark.get("task_class") if isinstance(benchmark, dict) else None
        tasks.append(
            Task(
                task_id=task_id,
                mode=mode,
                repo=repo,
                base_ref=base_ref,
                base_commit=base_commit,
                prompt=prompt.strip(),
                verifiers=verifiers,
                task_class=task_class,
            )
        )
        seen.add(task_id)
        seen_labels[label] = task_id
    if not tasks:
        raise ValueError(f"no tasks found in {path}")
    return tasks


def make_codemap_shim(shim_dir: Path, arm: str, codemap_cmd: list[str]) -> Path:
    return write_shim(shim_dir, arm, codemap_cmd)


def task_prompt(task: Task, arm: str) -> str:
    if task.mode == MODE_ANALYSIS:
        common = ANALYSIS_COMMON_PROMPT
        arm_prompt = ANALYSIS_ARM_PROMPTS[arm]
    elif task.task_class == "exact_control":
        common, arm_prompt = f"{COMMON_PROMPT}\n{EXACT_COMMON_PROMPT}", EXACT_ARM_PROMPTS[arm]
    else:
        common = COMMON_PROMPT
        arm_prompt = ARM_PROMPTS[arm]
    return f"{common}\n{arm_prompt}\nTASK (identical in both arms):\n{task.prompt}\n"


def arm_protocol_valid(task: Task, arm: str, protocol: dict[str, Any]) -> bool:
    if arm == ARM_CONTROL:
        return protocol["invocation_count"] == 0
    if task.task_class == "exact_control":
        return protocol["invocation_count"] == 0
    return protocol["compliant"] is True


def parse_codex_events(text: str) -> dict[str, Any]:
    usage = {
        "input_tokens": 0,
        "cached_input_tokens": 0,
        "output_tokens": 0,
        "reasoning_output_tokens": 0,
    }
    thread_ids: list[str] = []
    agent_messages: list[str] = []
    command_items = 0
    completed_commands: list[str] = []
    invalid_lines = 0
    for line in text.splitlines():
        if not line.strip():
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            invalid_lines += 1
            continue
        if event.get("type") == "thread.started" and isinstance(event.get("thread_id"), str):
            thread_ids.append(event["thread_id"])
        if event.get("type") == "turn.completed" and isinstance(event.get("usage"), dict):
            for key in usage:
                value = event["usage"].get(key, 0)
                if isinstance(value, int):
                    usage[key] += value
        item = event.get("item")
        if isinstance(item, dict):
            if item.get("type") == "command_execution":
                command_items += 1
                if item.get("status") in {"completed", "failed"} and isinstance(
                    item.get("command"), str
                ):
                    completed_commands.append(item["command"])
            if item.get("type") == "agent_message" and isinstance(item.get("text"), str):
                agent_messages.append(item["text"])
    return {
        "usage": usage,
        "thread_ids": thread_ids,
        "agent_messages": agent_messages,
        "command_execution_items": command_items,
        "completed_commands": completed_commands,
        "invalid_jsonl_lines": invalid_lines,
    }


def read_invocations(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    rows = [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]
    if not all(isinstance(row, dict) for row in rows):
        raise ValueError(f"invalid codemap invocation log: {path}")
    return rows


def expand_command(
    command: list[str], worktree: Path, repo: Path, artifact_dir: Path
) -> list[str]:
    values = {
        "worktree": str(worktree),
        "repo": str(repo),
        "last_message": str(artifact_dir / "last-message.md"),
        "events": str(artifact_dir / "events.jsonl"),
        "patch": str(artifact_dir / "patch.diff"),
    }
    try:
        return [part.format_map(values) for part in command]
    except KeyError as exc:
        raise ValueError(f"unknown verifier placeholder: {exc.args[0]}") from exc


def run_verifiers(task: Task, worktree: Path, artifact_dir: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for index, verifier in enumerate(task.verifiers, 1):
        command = expand_command(verifier.command, worktree, task.repo, artifact_dir)
        result = run_process(command, worktree, verifier.timeout_seconds)
        stdout_path = artifact_dir / f"verify-{index}-{safe_label(verifier.name)}.stdout.log"
        stderr_path = artifact_dir / f"verify-{index}-{safe_label(verifier.name)}.stderr.log"
        stdout_path.write_text(result.stdout, encoding="utf-8")
        stderr_path.write_text(result.stderr, encoding="utf-8")
        rows.append(
            {
                "name": verifier.name,
                "category": verifier.category,
                "weight": verifier.weight,
                "required": verifier.required,
                "command": command,
                "status": result.status,
                "elapsed_ms": result.elapsed_ms,
                "timed_out": result.timed_out,
                "passed": result.status == 0 and not result.timed_out,
                "stdout_artifact": str(stdout_path),
                "stderr_artifact": str(stderr_path),
            }
        )
    return rows


def completeness_summary(verifiers: list[dict[str, Any]]) -> dict[str, Any]:
    total_weight = sum(verifier["weight"] for verifier in verifiers)
    passed_weight = sum(verifier["weight"] for verifier in verifiers if verifier["passed"])
    categories: dict[str, dict[str, Any]] = {}
    for verifier in verifiers:
        category = categories.setdefault(
            verifier["category"],
            {
                "criteria": 0,
                "passed_criteria": 0,
                "total_weight": 0.0,
                "passed_weight": 0.0,
            },
        )
        category["criteria"] += 1
        category["total_weight"] += verifier["weight"]
        if verifier["passed"]:
            category["passed_criteria"] += 1
            category["passed_weight"] += verifier["weight"]
    for category in categories.values():
        category["score"] = round(category["passed_weight"] / category["total_weight"], 6)
        category["total_weight"] = round(category["total_weight"], 6)
        category["passed_weight"] = round(category["passed_weight"], 6)
    return {
        "score": round(passed_weight / total_weight, 6),
        "criteria": len(verifiers),
        "passed_criteria": sum(verifier["passed"] for verifier in verifiers),
        "total_weight": round(total_weight, 6),
        "passed_weight": round(passed_weight, 6),
        "required_criteria_passed": all(
            verifier["passed"] for verifier in verifiers if verifier["required"]
        ),
        "categories": categories,
    }


def capture_patch(worktree: Path, base_commit: str, artifact_dir: Path) -> list[str]:
    git(worktree, ["add", "-N", "--", "."])
    changed = git(worktree, ["diff", "--name-only", base_commit, "--"])
    patch = git(worktree, ["diff", "--binary", base_commit, "--"], timeout_seconds=120)
    (artifact_dir / "patch.diff").write_text(patch.stdout, encoding="utf-8")
    (artifact_dir / "git-status.txt").write_text(
        git(worktree, ["status", "--short"]).stdout, encoding="utf-8"
    )
    return [line for line in changed.stdout.splitlines() if line]


def command_version(command: list[str]) -> str:
    result = run_process([*command, "--version"], Path.cwd(), 15)
    text = (result.stdout or result.stderr).strip()
    return text.splitlines()[0] if result.status == 0 and text else "unknown"


def trial_fingerprint(
    task: Task,
    base_commit: str,
    arm: str,
    order: int,
    args: argparse.Namespace,
    codex_version: str,
    codemap_version: str,
    codemap_hashes: list[dict[str, str]],
    codemap_identity: dict[str, Any],
) -> str:
    return stable_hash(
        {
            "task_id": task.task_id,
            "mode": task.mode,
            "task_class": task.task_class,
            "repo": str(task.repo),
            "base_commit": base_commit,
            "task_prompt": task.prompt,
            "verifiers": [verifier.__dict__ for verifier in task.verifiers],
            "arm": arm,
            "order": order,
            "prompt_protocol_version": PROMPT_PROTOCOL_VERSION,
            "composed_prompt_sha256": hashlib.sha256(
                task_prompt(task, arm).encode("utf-8")
            ).hexdigest(),
            "harness_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            "protocol_parser_sha256": hashlib.sha256(
                Path(__file__).with_name("codemap_protocol.py").read_bytes()
            ).hexdigest(),
            "process_runner_sha256": hashlib.sha256(
                Path(__file__).with_name("benchmark_parallel.py").read_bytes()
            ).hexdigest(),
            "codex_runtime_sha256": codex_runtime.codex_runtime_sha256(),
            "model": args.model,
            "reasoning_effort": args.reasoning_effort,
            "timeout_seconds": args.timeout_seconds,
            "codex_version": codex_version,
            "codex_artifacts": getattr(args, "codex_artifacts", []),
            "codemap_version": codemap_version,
            "codemap_hashes": codemap_hashes,
            "codemap_identity": codemap_identity,
        }
    )


def run_trial(
    task: Task,
    repetition: int,
    arm: str,
    order: int,
    args: argparse.Namespace,
    codex_cmd: list[str],
    codemap_cmd: list[str],
    codex_version: str,
    codemap_version: str,
    codemap_hashes: list[dict[str, str]],
    codemap_identity: dict[str, Any],
    out_dir: Path,
    work_root: Path,
) -> dict[str, Any]:
    base_commit = task.base_commit
    key = f"{safe_label(task.task_id)}-r{repetition}-{arm}"
    artifact_dir = out_dir / "trials" / key
    fingerprint = trial_fingerprint(
        task,
        base_commit,
        arm,
        order,
        args,
        codex_version,
        codemap_version,
        codemap_hashes,
        codemap_identity,
    )
    resumed = existing_trial(artifact_dir, fingerprint, args.resume)
    if resumed is not None:
        print(f"[ab] resume {key}", file=sys.stderr)
        return resumed
    infrastructure_attempt = current_attempt(artifact_dir)
    artifact_dir.mkdir(parents=True, exist_ok=True)
    worktree = work_root / (key if infrastructure_attempt == 1 else f"{key}-attempt-2")
    added = add_worktree(task.repo, worktree, base_commit)
    if added.status != 0:
        raise ValueError(f"cannot create worktree {worktree}: {added.stderr.strip()}")
    try:
        cache_dir = artifact_dir / "codemap-cache"
        cache_dir.mkdir(parents=True, exist_ok=True)
        invocation_log = cache_dir / "invocations.log"
        shim_dir = artifact_dir / "bin"
        make_codemap_shim(shim_dir, arm, codemap_cmd)
        prompt = task_prompt(task, arm)
        (artifact_dir / "prompt.txt").write_text(prompt, encoding="utf-8")
        events_path = artifact_dir / "events.jsonl"
        last_message_path = artifact_dir / "last-message.md"
        stderr_path = artifact_dir / "codex.stderr.log"
        env = os.environ.copy()
        env["PATH"] = str(shim_dir) + os.pathsep + env.get("PATH", "")
        env.update(shell_profile_environment(shim_dir))
        env["CODEMAP_CACHE_DIR"] = str(cache_dir)
        env["CODEMAP_AB_ARM"] = arm
        env["CODEMAP_AB_INVOCATION_LOG"] = str(invocation_log)
        env["CODEMAP_AB_WORKTREE"] = str(worktree)
        invocation = [
            *codex_cmd,
            "exec",
            "--json",
            "--ephemeral",
            "--ignore-user-config",
            "--ignore-rules",
            "--strict-config", *codex_runtime.codex_runtime_isolation_args(),
            "--color",
            "never",
            "--disable",
            "multi_agent",
            "--disable",
            "enable_fanout",
            "-m",
            args.model,
            "-c",
            f'model_reasoning_effort="{args.reasoning_effort}"',
            "-c",
            'approval_policy="never"',
            "-s",
            "workspace-write",
            "-C",
            str(worktree),
            "--add-dir",
            str(cache_dir),
            "-o",
            str(last_message_path),
            prompt,
        ]
        print(f"[ab] run {key} order={order} model={args.model}/{args.reasoning_effort}", file=sys.stderr)
        with codex_runtime.isolated_codex_runtime(env) as runtime:
            codex = run_process(invocation, worktree, args.timeout_seconds, env=runtime.env)
            runtime_evidence = runtime.evidence()
        events_path.write_text(codex.stdout, encoding="utf-8")
        stderr_path.write_text(codex.stderr, encoding="utf-8")
        event_summary = parse_codex_events(codex.stdout)
        protocol = codemap_protocol(
            task.mode,
            arm,
            read_invocations(invocation_log),
            worktree,
            event_summary["completed_commands"],
        )
        changed_paths = capture_patch(worktree, base_commit, artifact_dir)
        # Capture the candidate before trusted verifiers run. Verifiers may compile,
        # create caches, or otherwise touch the worktree; those effects are not the
        # model's patch and must not leak into changed-path evidence.
        verifiers = run_verifiers(task, worktree, artifact_dir)
        completeness = completeness_summary(verifiers)
        (artifact_dir / "post-verifier-git-status.txt").write_text(
            git(worktree, ["status", "--short"]).stdout, encoding="utf-8"
        )
        outcome_passed = (
            codex.status == 0
            and not codex.timed_out
            and completeness["required_criteria_passed"]
            and (task.mode != MODE_ANALYSIS or not changed_paths)
        )
        arm_valid = arm_protocol_valid(task, arm, protocol)
        verifier_timed_out = any(verifier["timed_out"] for verifier in verifiers)
        run_valid = codex.status == 0 and not codex.timed_out and arm_valid and not verifier_timed_out
        invalidation_reason = None
        if codex.timed_out:
            invalidation_reason = "codex_timeout"
        elif codex.status != 0:
            invalidation_reason = "codex_crash"
        elif verifier_timed_out:
            invalidation_reason = "verifier_timeout"
        elif not arm_valid:
            invalidation_reason = (
                "control_codemap_access"
                if arm == ARM_CONTROL
                else "treatment_protocol_noncompliant"
            )
        result = {
            "task_id": task.task_id,
            "mode": task.mode,
            "task_prompt_sha256": hashlib.sha256(task.prompt.encode("utf-8")).hexdigest(),
            "composed_prompt_sha256": hashlib.sha256(prompt.encode("utf-8")).hexdigest(),
            "prompt_protocol_version": PROMPT_PROTOCOL_VERSION,
            "repo": str(task.repo),
            "base_ref": task.base_ref,
            "base_commit": base_commit,
            "repetition": repetition,
            "arm": arm,
            "order": order,
            "model": args.model,
            "reasoning_effort": args.reasoning_effort,
            "codex_version": codex_version,
            "codex_artifacts": getattr(args, "codex_artifacts", []),
            "codemap_version": codemap_version,
            "codemap_binary_hashes": codemap_hashes,
            "report_prelude": {"codemap": codemap_identity},
            "trial_fingerprint": fingerprint,
            "infrastructure_attempt": infrastructure_attempt,
            "prior_attempts": ["attempts/attempt-1/result.json"] if infrastructure_attempt == 2 else [],
            "runtime": runtime_evidence,
            "codex": {
                "status": codex.status,
                "elapsed_ms": codex.elapsed_ms,
                "timed_out": codex.timed_out,
                "events_artifact": str(events_path),
                "stderr_artifact": str(stderr_path),
                "last_message_artifact": str(last_message_path),
                **event_summary,
            },
            "codemap_protocol": protocol,
            "patch_artifact": str(artifact_dir / "patch.diff"),
            "verifiers": verifiers,
            "completeness": completeness,
            "changed_paths": changed_paths,
            "analysis_no_repo_changes": task.mode != MODE_ANALYSIS or not changed_paths,
            "verifier_passed": all(verifier["passed"] for verifier in verifiers),
            "outcome_passed": outcome_passed,
            "run_valid": run_valid,
            "invalidation_reason": invalidation_reason,
            "worktree": str(worktree) if args.keep_worktrees else None,
        }
        (artifact_dir / "result.json").write_text(
            json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
    finally:
        if not args.keep_worktrees:
            removed = remove_worktree(task.repo, worktree)
            if removed.status != 0:
                print(f"[ab] warning: could not remove {worktree}: {removed.stderr}", file=sys.stderr)
    if retry_infrastructure_failure(artifact_dir, result):
        print(f"[ab] retry {key} after {result['invalidation_reason']}", file=sys.stderr)
        return run_trial(
            task, repetition, arm, order, args, codex_cmd, codemap_cmd, codex_version,
            codemap_version, codemap_hashes, codemap_identity, out_dir, work_root,
        )
    return result


def median(values: list[int]) -> int | None:
    return round(statistics.median(values)) if values else None


def aggregate_category_coverage(rows: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    categories: dict[str, dict[str, Any]] = {}
    for row in rows:
        for name, coverage in row["completeness"]["categories"].items():
            aggregate = categories.setdefault(
                name,
                {
                    "criteria": 0,
                    "passed_criteria": 0,
                    "total_weight": 0.0,
                    "passed_weight": 0.0,
                },
            )
            for key in ["criteria", "passed_criteria", "total_weight", "passed_weight"]:
                aggregate[key] += coverage[key]
    for coverage in categories.values():
        coverage["score"] = round(coverage["passed_weight"] / coverage["total_weight"], 6)
        coverage["total_weight"] = round(coverage["total_weight"], 6)
        coverage["passed_weight"] = round(coverage["passed_weight"], 6)
    return categories


def valid_pair_keys(results: list[dict[str, Any]]) -> set[tuple[str, int]]:
    by_pair: dict[tuple[str, int], dict[str, dict[str, Any]]] = {}
    for row in results:
        by_pair.setdefault((row["task_id"], row["repetition"]), {})[row["arm"]] = row
    return {
        key
        for key, pair in by_pair.items()
        if set(pair) == set(ARMS) and all(row["run_valid"] for row in pair.values())
    }


def arm_summary(results: list[dict[str, Any]], arm: str) -> dict[str, Any]:
    rows = [row for row in results if row["arm"] == arm]
    valid_keys = valid_pair_keys(results)
    valid = [row for row in rows if (row["task_id"], row["repetition"]) in valid_keys]
    passed = [row for row in valid if row["outcome_passed"]]
    usage_keys = ["input_tokens", "cached_input_tokens", "output_tokens", "reasoning_output_tokens"]
    return {
        "trials": len(rows),
        "valid_trials": len(valid),
        "passed_trials": len(passed),
        "pass_rate": round(len(passed) / len(valid), 4) if valid else None,
        "mean_completeness_score": (
            round(statistics.mean(row["completeness"]["score"] for row in valid), 6)
            if valid
            else None
        ),
        "median_completeness_score": (
            round(statistics.median(row["completeness"]["score"] for row in valid), 6)
            if valid
            else None
        ),
        "category_coverage": aggregate_category_coverage(valid),
        "median_elapsed_ms": median([row["codex"]["elapsed_ms"] for row in valid]),
        "median_usage": {
            key: median([row["codex"]["usage"][key] for row in valid]) for key in usage_keys
        },
    }


def paired_summary(results: list[dict[str, Any]]) -> dict[str, int]:
    by_pair: dict[tuple[str, int], dict[str, dict[str, Any]]] = {}
    for row in results:
        by_pair.setdefault((row["task_id"], row["repetition"]), {})[row["arm"]] = row
    summary = {
        "pairs": len(by_pair),
        "valid_pairs": 0,
        "invalid_pairs": 0,
        "codemap_wins": 0,
        "control_wins": 0,
        "ties": 0,
        "both_pass": 0,
        "both_fail": 0,
        "mixed_outcome": 0,
    }
    for pair in by_pair.values():
        if set(pair) != set(ARMS) or not all(row["run_valid"] for row in pair.values()):
            summary["invalid_pairs"] += 1
            continue
        summary["valid_pairs"] += 1
        control_passed = pair[ARM_CONTROL]["outcome_passed"]
        treatment_passed = pair[ARM_TREATMENT]["outcome_passed"]
        control_score = pair[ARM_CONTROL]["completeness"]["score"]
        treatment_score = pair[ARM_TREATMENT]["completeness"]["score"]
        if control_passed and treatment_passed:
            summary["both_pass"] += 1
        elif not control_passed and not treatment_passed:
            summary["both_fail"] += 1
        else:
            summary["mixed_outcome"] += 1

        # Required criteria and protected-path integrity remain non-negotiable.
        # Within the same task-outcome class, weighted completeness decides which
        # arm understood and covered more of the independently checked surface.
        if treatment_passed != control_passed:
            winner = ARM_TREATMENT if treatment_passed else ARM_CONTROL
        elif treatment_score > control_score + 1e-9:
            winner = ARM_TREATMENT
        elif control_score > treatment_score + 1e-9:
            winner = ARM_CONTROL
        else:
            winner = None
        if winner == ARM_TREATMENT:
            summary["codemap_wins"] += 1
        elif winner == ARM_CONTROL:
            summary["control_wins"] += 1
        else:
            summary["ties"] += 1
    return summary


def effect_summary(arms: dict[str, dict[str, Any]]) -> dict[str, Any]:
    control = arms[ARM_CONTROL]
    treatment = arms[ARM_TREATMENT]

    def metric_delta(key: str) -> int | None:
        treatment_value = treatment[key]
        control_value = control[key]
        if treatment_value is None or control_value is None:
            return None
        return treatment_value - control_value

    control_rate = control["pass_rate"]
    treatment_rate = treatment["pass_rate"]
    control_usage = control["median_usage"]
    treatment_usage = treatment["median_usage"]
    control_score = control["mean_completeness_score"]
    treatment_score = treatment["mean_completeness_score"]
    score_delta = (
        round((treatment_score - control_score) * 100, 2)
        if treatment_score is not None and control_score is not None
        else None
    )
    input_delta = (
        treatment_usage["input_tokens"] - control_usage["input_tokens"]
        if treatment_usage["input_tokens"] is not None
        and control_usage["input_tokens"] is not None
        else None
    )
    if score_delta is None:
        interpretation = "No valid paired completeness measurement is available."
    elif score_delta > 0 and (input_delta is None or input_delta > 0):
        interpretation = (
            "codemap produced more externally verified completeness at a higher or unknown "
            "observed input-token cost."
        )
    elif score_delta > 0:
        interpretation = (
            "codemap produced more externally verified completeness without higher observed "
            "input-token cost."
        )
    elif score_delta < 0:
        interpretation = "codemap produced less externally verified completeness on this task set."
    elif input_delta is not None and input_delta > 0:
        interpretation = (
            "codemap used more input tokens without a measured completeness gain on this task set."
        )
    else:
        interpretation = "No externally verified completeness difference was measured."
    return {
        "mean_completeness_delta_percentage_points": score_delta,
        "pass_rate_delta_percentage_points": (
            round((treatment_rate - control_rate) * 100, 2)
            if treatment_rate is not None and control_rate is not None
            else None
        ),
        "median_elapsed_delta_ms": metric_delta("median_elapsed_ms"),
        "median_input_token_delta": input_delta,
        "median_output_token_delta": (
            treatment_usage["output_tokens"] - control_usage["output_tokens"]
            if treatment_usage["output_tokens"] is not None
            and control_usage["output_tokens"] is not None
            else None
        ),
        "interpretation": interpretation,
    }


def run_preflight(task: Task, out_dir: Path, work_root: Path) -> dict[str, Any]:
    base_commit = task.base_commit
    key = f"preflight-{safe_label(task.task_id)}"
    artifact_dir = out_dir / "preflight" / safe_label(task.task_id)
    artifact_dir.mkdir(parents=True, exist_ok=True)
    worktree = work_root / key
    added = add_worktree(task.repo, worktree, base_commit)
    if added.status != 0:
        raise ValueError(f"cannot create preflight worktree {worktree}: {added.stderr.strip()}")
    try:
        print(f"[ab] preflight {task.task_id} base={base_commit[:12]}", file=sys.stderr)
        for name in ["last-message.md", "events.jsonl", "patch.diff"]:
            (artifact_dir / name).write_text("", encoding="utf-8")
        verifiers = run_verifiers(task, worktree, artifact_dir)
        completeness = completeness_summary(verifiers)
        result = {
            "task_id": task.task_id,
            "mode": task.mode,
            "base_commit": base_commit,
            "verifiers": verifiers,
            "completeness": completeness,
            "baseline_passed": all(verifier["passed"] for verifier in verifiers),
        }
        (artifact_dir / "result.json").write_text(
            json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        if result["baseline_passed"]:
            raise ValueError(
                f"task {task.task_id}: every verifier already passes at {base_commit[:12]}; "
                "the task has no measured behavioral gap"
            )
        return result
    finally:
        removed = remove_worktree(task.repo, worktree)
        if removed.status != 0:
            print(f"[ab] warning: could not remove {worktree}: {removed.stderr}", file=sys.stderr)


def write_summary(
    out_dir: Path,
    tasks_path: Path,
    results: list[dict[str, Any]],
    preflight: list[dict[str, Any]],
    args: argparse.Namespace,
    codex_version: str,
    codemap_version: str,
    codemap_hashes: list[dict[str, str]],
    codemap_identity: dict[str, Any],
) -> dict[str, Any]:
    arms = {arm: arm_summary(results, arm) for arm in ARMS}
    pairs = paired_summary(results)
    effect = effect_summary(arms)
    summary = {
        "kind": "codemap_behavioral_ab",
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "tasks_file": str(tasks_path),
        "model": args.model,
        "reasoning_effort": args.reasoning_effort,
        "codex_version": codex_version,
        "codex_artifacts": getattr(args, "codex_artifacts", []),
        "codemap_version": codemap_version,
        "codemap_binary_hashes": codemap_hashes,
        "report_prelude": {"codemap": codemap_identity},
        "repetitions": args.repetitions,
        "preflight": preflight,
        "scoring_contract": {
            "primary_metric": "weighted_external_completeness",
            "criterion_formula": "passed_weight / total_weight",
            "winner_rule": "required_outcome_first_then_completeness",
            "resource_metrics_role": "secondary_cost_only",
            "valid_pairs_only": True,
        },
        "claim_boundary": (
            "This benchmark measures externally verified completeness for the declared tasks. "
            "Token and time deltas are resource costs, not automatic evidence against deeper "
            "repository understanding. General lift still requires representative tasks and "
            "repeated runs."
        ),
        "arms": arms,
        "paired": pairs,
        "effect": effect,
    }
    (out_dir / "summary.json").write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (out_dir / "results.jsonl").write_text(
        "\n".join(json.dumps(row, sort_keys=True) for row in results) + "\n",
        encoding="utf-8",
    )
    lines = [
        "# codemap behavioral A/B",
        "",
        summary["claim_boundary"],
        "",
        f"- Model: `{args.model}`",
        f"- Reasoning: `{args.reasoning_effort}`",
        f"- Codex: `{codex_version}`",
        f"- codemap: `{codemap_version}`",
        f"- codemap executable: `{codemap_identity['build_identity']['executable_path']}`",
        f"- codemap SHA-256: `{codemap_identity['build_identity']['binary_sha256']}`",
        f"- Tasks: `{tasks_path}`",
        f"- Repetitions: `{args.repetitions}`",
        "",
        "## Externally verified result",
        "",
        "| Arm | Trials | Valid | Mean completeness | Median completeness | Required pass rate |",
        "| --- | ---: | ---: | ---: | ---: | ---: |",
    ]
    for arm in ARMS:
        row = arms[arm]
        rate = "-" if row["pass_rate"] is None else f'{row["pass_rate"] * 100:.1f}%'
        mean_score = (
            "-"
            if row["mean_completeness_score"] is None
            else f'{row["mean_completeness_score"] * 100:.1f}%'
        )
        median_score = (
            "-"
            if row["median_completeness_score"] is None
            else f'{row["median_completeness_score"] * 100:.1f}%'
        )
        lines.append(
            f'| {arm} | {row["trials"]} | {row["valid_trials"]} | {mean_score} | '
            f'{median_score} | {rate} |'
        )
    categories = sorted(
        set(arms[ARM_CONTROL]["category_coverage"])
        | set(arms[ARM_TREATMENT]["category_coverage"])
    )
    if categories:
        lines.extend(
            [
                "",
                "### Coverage by criterion category",
                "",
                "| Category | Control | codemap | Delta |",
                "| --- | ---: | ---: | ---: |",
            ]
        )
        for category in categories:
            control_coverage = arms[ARM_CONTROL]["category_coverage"].get(category)
            treatment_coverage = arms[ARM_TREATMENT]["category_coverage"].get(category)
            control_score = control_coverage["score"] if control_coverage else None
            treatment_score = treatment_coverage["score"] if treatment_coverage else None
            delta = (
                (treatment_score - control_score) * 100
                if treatment_score is not None and control_score is not None
                else None
            )
            lines.append(
                f'| {category} | '
                f'{"-" if control_score is None else f"{control_score * 100:.1f}%"} | '
                f'{"-" if treatment_score is None else f"{treatment_score * 100:.1f}%"} | '
                f'{"-" if delta is None else f"{delta:+.1f} pp"} |'
            )
    lines.extend(
        [
            "",
            "## Paired outcomes",
            "",
            "| Valid pairs | Invalid | codemap better | Control better | Ties | Both pass | Both fail | Mixed |",
            "| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
            f'| {pairs["valid_pairs"]} | {pairs["invalid_pairs"]} | {pairs["codemap_wins"]} | '
            f'{pairs["control_wins"]} | {pairs["ties"]} | {pairs["both_pass"]} | '
            f'{pairs["both_fail"]} | {pairs["mixed_outcome"]} |',
            "",
            "## Completeness effect",
            "",
            "| Mean completeness delta | Required pass-rate delta |",
            "| ---: | ---: |",
            f'| {effect["mean_completeness_delta_percentage_points"] if effect["mean_completeness_delta_percentage_points"] is not None else "-"} pp | '
            f'{effect["pass_rate_delta_percentage_points"] if effect["pass_rate_delta_percentage_points"] is not None else "-"} pp |',
            "",
            effect["interpretation"],
            "",
            "## Resource cost (secondary)",
            "",
            "| Arm | Median time | Median input | Median cached input | Median output |",
            "| --- | ---: | ---: | ---: | ---: |",
            f'| control | {arms[ARM_CONTROL]["median_elapsed_ms"] or "-"}ms | '
            f'{arms[ARM_CONTROL]["median_usage"]["input_tokens"] or "-"} | '
            f'{arms[ARM_CONTROL]["median_usage"]["cached_input_tokens"] or "-"} | '
            f'{arms[ARM_CONTROL]["median_usage"]["output_tokens"] or "-"} |',
            f'| codemap | {arms[ARM_TREATMENT]["median_elapsed_ms"] or "-"}ms | '
            f'{arms[ARM_TREATMENT]["median_usage"]["input_tokens"] or "-"} | '
            f'{arms[ARM_TREATMENT]["median_usage"]["cached_input_tokens"] or "-"} | '
            f'{arms[ARM_TREATMENT]["median_usage"]["output_tokens"] or "-"} |',
            "",
            "| Median time delta | Median input delta | Median output delta |",
            "| ---: | ---: | ---: |",
            f'| {effect["median_elapsed_delta_ms"] if effect["median_elapsed_delta_ms"] is not None else "-"}ms | '
            f'{effect["median_input_token_delta"] if effect["median_input_token_delta"] is not None else "-"} | '
            f'{effect["median_output_token_delta"] if effect["median_output_token_delta"] is not None else "-"} |',
            "",
            "A winner is chosen by required outcome first, then weighted completeness. Token use "
            "never decides the winner. A Codex failure, timeout, or arm protocol violation makes "
            "the pair invalid instead of silently counting it as a product loss.",
            "",
            "Artifacts: `results.jsonl` plus prompt, events, final message, patch, git status, and "
            "verifier logs under `trials/`.",
        ]
    )
    (out_dir / "summary.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    return summary


def default_out_dir() -> Path:
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    return Path(__file__).resolve().parents[1] / "target" / "codemap-ab" / stamp


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Paired Codex behavioral A/B: identical tasks with and without codemap."
    )
    parser.add_argument("tasks", help="JSONL task manifest; see docs/BENCHMARK_AB.md")
    parser.add_argument("--model", default="gpt-5.6-sol")
    parser.add_argument("--reasoning-effort", default="high")
    parser.add_argument("--repetitions", type=int, default=1)
    parser.add_argument("--timeout-seconds", type=int, default=1800)
    parser.add_argument("--verifier-timeout-seconds", type=int, default=600)
    parser.add_argument("--codex-bin", default=os.environ.get("CODEX_BIN") or shutil.which("codex"))
    parser.add_argument("--codemap-bin", help="Direct codemap executable path.")
    parser.add_argument("--codex-argv-json", help=argparse.SUPPRESS)
    parser.add_argument("--codemap-argv-json", help=argparse.SUPPRESS)
    parser.add_argument("--out-dir", default=str(default_out_dir()))
    parser.add_argument("--work-dir", help="Parent for disposable git worktrees (default: /tmp).")
    parser.add_argument("--keep-worktrees", action="store_true")
    parser.add_argument("--resume", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--preflight-only", action="store_true")
    parser.add_argument("--treatment-preflight", action="store_true")
    parser.add_argument("--parallel-pairs", type=int, default=1)
    args = parser.parse_args(argv)
    if args.preflight_only and args.treatment_preflight:
        parser.error("--preflight-only and --treatment-preflight are mutually exclusive")
    for name in ["repetitions", "timeout_seconds", "verifier_timeout_seconds", "parallel_pairs"]:
        if getattr(args, name) <= 0:
            parser.error(f"--{name.replace('_', '-')} must be positive")
    if args.reasoning_effort not in {"minimal", "low", "medium", "high", "xhigh"}:
        parser.error("--reasoning-effort must be minimal, low, medium, high, or xhigh")
    return args

def split_command(value: str | None, label: str, argv_json: str | None = None) -> list[str]:
    if not value and not argv_json:
        raise ValueError(f"{label} not found; pass --{label.replace('_', '-')}")
    command = json.loads(argv_json) if argv_json else [value]
    if not isinstance(command, list) or not all(isinstance(part, str) for part in command):
        raise ValueError(f"{label} argv must be a string array")
    if not command:
        raise ValueError(f"empty {label} command")
    executable = command[0]
    if not Path(executable).is_absolute():
        resolved = shutil.which(executable)
        if resolved:
            command[0] = resolved
        elif "/" in executable or "\\" in executable:
            command[0] = str(canonical(Path.cwd() / executable))
    if not Path(command[0]).is_file():
        raise ValueError(f"{label} executable not found: {command[0]}")
    return command

def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        tasks_path = canonical(Path(args.tasks))
        tasks = load_tasks(tasks_path, args.verifier_timeout_seconds)
        codex_cmd = split_command(args.codex_bin, "codex_bin", args.codex_argv_json)
        codemap_value = json.loads(args.codemap_argv_json) if args.codemap_argv_json else args.codemap_bin
        codemap_cmd, codemap_resolution = resolve_codemap_command(
            codemap_value, Path(__file__).resolve().parents[1]
        )
        codemap_identity = benchmark_binary_identity(
            codemap_cmd, codemap_resolution, tasks[0].repo
        )
        args.codex_artifacts = command_artifacts(codex_cmd)
    except (OSError, ValueError, CodemapIdentityError) as exc:
        print(f"codemap A/B: {exc}", file=sys.stderr)
        return 2
    pairs = []
    for task_index, task in enumerate(tasks):
        for repetition in range(1, args.repetitions + 1):
            order = list(ARMS) if (task_index + repetition) % 2 else list(reversed(ARMS))
            pairs.append((task, repetition, [(arm, index + 1) for index, arm in enumerate(order)]))
    matrix = [
        (task, repetition, arm, order)
        for task, repetition, arms in pairs
        for arm, order in arms
    ]
    if args.dry_run:
        print(
            json.dumps(
                {
                    "model": args.model,
                    "reasoning_effort": args.reasoning_effort,
                    "tasks": len(tasks),
                    "repetitions": args.repetitions,
                    "trials": [
                        {
                            "task_id": task.task_id,
                            "mode": task.mode,
                            "repetition": repetition,
                            "arm": arm,
                            "order": order,
                        }
                        for task, repetition, arm, order in matrix
                    ],
                },
                indent=2,
            )
        )
        return 0
    out_dir = canonical(Path(args.out_dir))
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "input-tasks.jsonl").write_text(
        tasks_path.read_text(encoding="utf-8"), encoding="utf-8"
    )
    if args.work_dir:
        work_root = canonical(Path(args.work_dir))
        work_root.mkdir(parents=True, exist_ok=True)
        remove_work_root = False
    else:
        work_root = Path(tempfile.mkdtemp(prefix="codemap-ab-worktrees-"))
        remove_work_root = not args.keep_worktrees
    codex_version = command_version(codex_cmd)
    codemap_version = codemap_identity["version_output"]
    codemap_hashes = codemap_identity["command_artifacts"]
    results: list[dict[str, Any]] = []
    preflight: list[dict[str, Any]] = []
    try:
        preflight = run_ordered(
            tasks, lambda task: run_preflight(task, out_dir, work_root), args.parallel_pairs
        )
        if args.preflight_only:
            print(f"A/B preflight: {out_dir / 'preflight'}")
            return 0
        if args.treatment_preflight:
            results = run_ordered(
                tasks,
                lambda task: run_trial(
                    task, 1, ARM_TREATMENT, 1, args, codex_cmd, codemap_cmd,
                    codex_version, codemap_version, codemap_hashes, codemap_identity,
                    out_dir, work_root,
                ),
                args.parallel_pairs,
            )
            baseline_leaks = [row["task_id"] for row in preflight if row["baseline_passed"]]
            failures = [
                row["task_id"] for row in results
                if not row["run_valid"] or not row["outcome_passed"]
            ]
            results_path = out_dir / "treatment-preflight-results.jsonl"
            results_path.write_text(
                "".join(json.dumps(row, sort_keys=True) + "\n" for row in results),
                encoding="utf-8",
            )
            summary = {
                "kind": "codemap_treatment_preflight",
                "tasks": len(tasks),
                "passed": len(tasks) - len(set(failures) | set(baseline_leaks)),
                "failed_tasks": sorted(set(failures)),
                "baseline_leaks": sorted(baseline_leaks),
            }
            (out_dir / "treatment-preflight-summary.json").write_text(
                json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
            )
            print(f"Treatment preflight: {results_path}")
            return 1 if failures or baseline_leaks else 0
        eligible = {
            row["task_id"] for row in preflight if row.get("baseline_passed") is False
        }
        eligible_pairs = [pair for pair in pairs if pair[0].task_id in eligible]

        def run_pair(pair: tuple[Task, int, list[tuple[str, int]]]) -> list[dict[str, Any]]:
            task, repetition, arms = pair
            return [
                run_trial(
                    task, repetition, arm, order, args, codex_cmd, codemap_cmd,
                    codex_version, codemap_version, codemap_hashes, codemap_identity,
                    out_dir, work_root,
                )
                for arm, order in arms
            ]

        for pair_results in run_ordered(eligible_pairs, run_pair, args.parallel_pairs):
            results.extend(pair_results)
        summary = write_summary(
            out_dir,
            tasks_path,
            results,
            preflight,
            args,
            codex_version,
            codemap_version,
            codemap_hashes,
            codemap_identity,
        )
    except (OSError, ValueError) as exc:
        print(f"codemap A/B: {exc}", file=sys.stderr)
        return 2
    finally:
        if remove_work_root:
            shutil.rmtree(work_root, ignore_errors=True)
    print(f"A/B summary: {out_dir / 'summary.md'}\nA/B results: {out_dir / 'results.jsonl'}")
    if summary["paired"]["invalid_pairs"]:
        print("codemap A/B: one or more pairs are invalid", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
