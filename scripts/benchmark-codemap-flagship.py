#!/usr/bin/env python3
"""Freeze, run, and evaluate the outcome-based codemap flagship A/B."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

from codemap_identity import benchmark_binary_identity, command_artifacts
from flagship_acceptance import evaluate
from flagship_manifest import command_version, freeze_corpus, load_frozen
from flagship_trajectory import analyze_trajectories


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


def _verify_frozen_tools(manifest: dict[str, Any]) -> None:
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


def run(args: argparse.Namespace) -> int:
    manifest_path = Path(args.manifest).resolve()
    manifest, _ = load_frozen(manifest_path)
    _verify_frozen_tools(manifest)
    command = _benchmark_command(args, manifest)
    return subprocess.run(command, cwd=ROOT, check=False).returncode


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    commands = root.add_subparsers(dest="command", required=True)
    freeze = commands.add_parser("freeze", help="freeze tasks, verifiers, repos, agent, and binary")
    freeze.add_argument("draft")
    freeze.add_argument("--out-dir", required=True)
    freeze.add_argument("--codex-bin", default="codex")
    freeze.add_argument("--codemap-bin")
    run_parser = commands.add_parser("run", help="run all 144 frozen agent trials")
    run_parser.add_argument("manifest")
    run_parser.add_argument("--out-dir", required=True)
    run_parser.add_argument("--work-dir")
    run_parser.add_argument("--resume", action="store_true")
    score = commands.add_parser("evaluate", help="evaluate outcomes and explain paired trajectories")
    score.add_argument("manifest")
    score.add_argument("--run-dir", required=True)
    score.add_argument("--out-dir", required=True)
    score.add_argument("--resume", action="store_true")
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
        manifest_path = Path(args.manifest).resolve()
        manifest, tasks = load_frozen(manifest_path)
        _verify_frozen_tools(manifest)
        out_dir = Path(args.out_dir).resolve()
        trajectory = None
        try:
            trajectory = analyze_trajectories(
                manifest_path,
                tasks,
                Path(args.run_dir).resolve(),
                out_dir / "trajectory-analysis",
                args.resume,
            )
        except Exception as exc:  # Interpretive analysis must never decide acceptance.
            print(f"trajectory analysis unavailable: {exc}", file=sys.stderr)
        output = evaluate(manifest_path, Path(args.run_dir), out_dir, trajectory)
        report = json.loads(output.read_text(encoding="utf-8"))
        print(output)
        return 0 if report["acceptance"]["accepted"] else 1
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as exc:
        print(f"codemap flagship benchmark: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
