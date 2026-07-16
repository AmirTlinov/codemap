#!/usr/bin/env python3
"""Freeze, run, and evaluate the outcome-based codemap flagship A/B."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any

from codemap_identity import benchmark_binary_identity, command_artifacts
from flagship_acceptance import INFRASTRUCTURE_FAILURES, evaluate
from flagship_manifest import command_version, freeze_corpus, load_frozen, read_jsonl


ROOT = Path(__file__).resolve().parents[1]


def _benchmark_command(args: argparse.Namespace, manifest: dict[str, Any]) -> list[str]:
    limits = manifest["limits"]
    command = [
        sys.executable,
        str(ROOT / "scripts/benchmark-codemap-ab.py"),
        str(Path(args.manifest).resolve().parent / manifest["tasks_file"]),
        "--model",
        manifest["model"],
        "--reasoning-effort",
        manifest["reasoning_effort"],
        "--repetitions",
        str(limits["repetitions"]),
        "--timeout-seconds",
        str(limits["timeout_seconds"]),
        "--verifier-timeout-seconds",
        str(limits["verifier_timeout_seconds"]),
        "--codex-argv-json",
        json.dumps(manifest["codex"]["command_argv"]),
        "--codemap-argv-json",
        json.dumps(manifest["codemap_identity"]["command_argv"]),
        "--out-dir",
        str(Path(args.out_dir).resolve()),
        "--parallel-pairs",
        str(limits["parallel_pairs"]),
    ]
    if args.work_dir:
        command.extend(["--work-dir", str(Path(args.work_dir).resolve())])
    if args.resume:
        command.append("--resume")
    return command


def _infrastructure_failures(run_dir: Path) -> dict[tuple[str, int, str], dict[str, Any]]:
    results = run_dir / "results.jsonl"
    if not results.is_file():
        return {}
    failures = {}
    for row in read_jsonl(results):
        timed_out_verifier = any(item.get("timed_out") is True for item in row.get("verifiers", []))
        if row.get("invalidation_reason") in INFRASTRUCTURE_FAILURES or timed_out_verifier:
            failures[(row["task_id"], row["repetition"], row["arm"])] = row
    return failures


def _retry_infrastructure(
    command: list[str], run_dir: Path, failures: dict[tuple[str, int, str], dict[str, Any]]
) -> int:
    attempts = run_dir / "infrastructure-attempts"
    attempts.mkdir(exist_ok=True)
    receipts = {}
    for key, row in failures.items():
        artifact = Path(row["codex"]["last_message_artifact"]).parent
        target = attempts / f"{artifact.name}-attempt-1"
        if target.exists():
            raise ValueError(f"infrastructure attempt already exists: {target}")
        shutil.move(str(artifact), target)
        receipts[key] = {
            "attempt": 1,
            "reason": row.get("invalidation_reason") or "verifier_infrastructure_failure",
            "artifact_dir": str(target),
        }
    retry = [*command]
    if "--resume" not in retry:
        retry.append("--resume")
    status = subprocess.run(retry, cwd=ROOT, check=False).returncode
    results_path = run_dir / "results.jsonl"
    if not results_path.is_file():
        return status
    rows = read_jsonl(results_path)
    for row in rows:
        key = (row["task_id"], row["repetition"], row["arm"])
        if key not in receipts:
            continue
        row["infrastructure_attempts"] = [receipts[key]]
        if any(item.get("timed_out") is True for item in row.get("verifiers", [])):
            row["run_valid"] = False
            row["invalidation_reason"] = "verifier_infrastructure_failure"
        result_path = Path(row["codex"]["last_message_artifact"]).parent / "result.json"
        result_path.write_text(json.dumps(row, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    results_path.write_text(
        "\n".join(json.dumps(row, sort_keys=True) for row in rows) + "\n", encoding="utf-8"
    )
    return status


def run(args: argparse.Namespace) -> int:
    manifest_path = Path(args.manifest).resolve()
    manifest, _ = load_frozen(manifest_path)
    codex = manifest["codex"]["command_argv"]
    if command_version(codex) != manifest["codex"]["version"]:
        raise ValueError("Codex version differs from the frozen manifest")
    if command_artifacts(codex) != manifest["codex"]["artifacts"]:
        raise ValueError("Codex executable bytes differ from the frozen manifest")
    codemap = manifest["codemap_identity"]["command_argv"]
    identity = benchmark_binary_identity(
        codemap, manifest["codemap_identity"]["resolution"], ROOT
    )
    if identity != manifest["codemap_identity"]:
        raise ValueError("codemap binary identity differs from the frozen manifest")
    command = _benchmark_command(args, manifest)
    status = subprocess.run(command, cwd=ROOT, check=False).returncode
    run_dir = Path(args.out_dir).resolve()
    failures = _infrastructure_failures(run_dir)
    if failures:
        status = _retry_infrastructure(command, run_dir, failures)
    return status


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    commands = root.add_subparsers(dest="command", required=True)
    freeze = commands.add_parser("freeze", help="freeze tasks, verifiers, repos, agent, and binary")
    freeze.add_argument("draft")
    freeze.add_argument("--out-dir", required=True)
    freeze.add_argument("--codex-bin", default="codex")
    freeze.add_argument("--codemap-bin")
    run_parser = commands.add_parser("run", help="run all 72 frozen agent trials")
    run_parser.add_argument("manifest")
    run_parser.add_argument("--out-dir", required=True)
    run_parser.add_argument("--work-dir")
    run_parser.add_argument("--resume", action="store_true")
    score = commands.add_parser("evaluate", help="evaluate deterministic evidence")
    score.add_argument("manifest")
    score.add_argument("--run-dir", required=True)
    score.add_argument("--out-dir", required=True)
    return root


def main(argv: list[str]) -> int:
    args = parser().parse_args(argv)
    try:
        if args.command == "freeze":
            output = freeze_corpus(
                Path(args.draft), Path(args.out_dir), args.codex_bin, args.codemap_bin
            )
            print(output)
            return 0
        if args.command == "run":
            return run(args)
        output = evaluate(Path(args.manifest), Path(args.run_dir), Path(args.out_dir))
        report = json.loads(output.read_text(encoding="utf-8"))
        print(output)
        return 0 if report["acceptance"]["accepted"] else 1
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as exc:
        print(f"codemap flagship benchmark: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
