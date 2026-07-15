#!/usr/bin/env python3
"""Freeze, run, blind, and evaluate the S15 flagship behavioral corpus."""

from __future__ import annotations

import argparse
import json
import shlex
import subprocess
import sys
from pathlib import Path

from codemap_identity import benchmark_binary_identity, command_artifacts
from flagship_acceptance import evaluate
from flagship_judging import prepare_assignments
from flagship_manifest import (
    command_version,
    file_sha256,
    freeze_corpus,
    load_frozen,
    resolve_command,
)


ROOT = Path(__file__).resolve().parents[1]


def require_current_contract(manifest: dict, manifest_path: Path) -> None:
    expected = {
        "harness_sha256": ROOT / "scripts/benchmark-codemap-ab.py",
        "protocol_sha256": ROOT / "scripts/codemap_protocol.py",
        "manifest_owner_sha256": ROOT / "scripts/flagship_manifest.py",
    }
    for field, path in expected.items():
        if file_sha256(path) != manifest.get(field):
            raise ValueError(f"frozen contract changed: {path.name}; freeze a new corpus")
    for artifact in manifest.get("gate_artifacts", []):
        path = ROOT / artifact["path"]
        if not path.is_file() or file_sha256(path) != artifact["sha256"]:
            raise ValueError(f"frozen gate changed: {artifact['path']}; freeze a new corpus")
    split_paths = [manifest_path.parent / f"{split}.tasks.jsonl" for split in ("calibration", "holdout")]
    for split, path in zip(("calibration", "holdout"), split_paths):
        if not path.is_file() or file_sha256(path) != manifest[f"{split}_tasks_sha256"]:
            raise ValueError(f"frozen {split} task bytes changed")


def run_split(args: argparse.Namespace) -> int:
    manifest_path = Path(args.manifest).resolve()
    manifest, _ = load_frozen(manifest_path)
    require_current_contract(manifest, manifest_path)
    codex = manifest["codex_command"]
    if command_version(codex) != manifest["codex_version"]:
        raise ValueError("Codex version differs from the frozen manifest")
    if command_artifacts(codex) != manifest["codex_artifacts"]:
        raise ValueError("Codex executable bytes differ from the frozen manifest")
    codemap = manifest["codemap_identity"]["command_argv"]
    identity = benchmark_binary_identity(codemap, manifest["codemap_identity"]["resolution"], ROOT)
    if identity != manifest["codemap_identity"]:
        raise ValueError("codemap binary identity differs from the frozen manifest")
    tasks = manifest_path.parent / f"{args.split}.tasks.jsonl"
    command = [
        sys.executable,
        str(ROOT / "scripts/benchmark-codemap-ab.py"),
        str(tasks),
        "--model",
        manifest["model"],
        "--reasoning-effort",
        manifest["reasoning_effort"],
        "--repetitions",
        str(manifest["repetitions"]),
        "--timeout-seconds",
        str(manifest["timeout_seconds"]),
        "--verifier-timeout-seconds",
        str(manifest["verifier_timeout_seconds"]),
        "--codex-bin",
        shlex.join(codex),
        "--codemap-bin",
        shlex.join(codemap),
        "--out-dir",
        str(Path(args.out_dir).resolve()),
    ]
    if args.work_dir:
        command.extend(["--work-dir", str(Path(args.work_dir).resolve())])
    if args.resume:
        command.append("--resume")
    return subprocess.run(command, cwd=ROOT, check=False).returncode


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    commands = root.add_subparsers(dest="command", required=True)
    freeze = commands.add_parser("freeze", help="freeze corpus, rubric, order, binaries, and repos")
    freeze.add_argument("draft")
    freeze.add_argument("--out-dir", required=True)
    freeze.add_argument("--codex-bin", default="codex")
    freeze.add_argument("--codemap-bin")
    run = commands.add_parser("run", help="run one frozen split through the existing A/B owner")
    run.add_argument("manifest")
    run.add_argument("split", choices=["calibration", "holdout"])
    run.add_argument("--out-dir", required=True)
    run.add_argument("--work-dir")
    run.add_argument("--resume", action="store_true")
    blind = commands.add_parser("prepare-judging", help="make arm-blind analysis assignments")
    blind.add_argument("manifest")
    blind.add_argument("--calibration-dir", required=True)
    blind.add_argument("--holdout-dir", required=True)
    blind.add_argument("--out-dir", required=True)
    score = commands.add_parser("evaluate", help="score calibration separately and gate holdout")
    score.add_argument("manifest")
    score.add_argument("--calibration-dir", required=True)
    score.add_argument("--holdout-dir", required=True)
    score.add_argument("--assignments", required=True)
    score.add_argument("--assignment-key", required=True)
    score.add_argument("--ratings", required=True)
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
            return run_split(args)
        manifest_path = Path(args.manifest).resolve()
        manifest, tasks = load_frozen(manifest_path)
        require_current_contract(manifest, manifest_path)
        if args.command == "prepare-judging":
            public, key = prepare_assignments(
                manifest_path,
                tasks,
                [Path(args.calibration_dir), Path(args.holdout_dir)],
                Path(args.out_dir),
            )
            print(json.dumps({"assignments": str(public), "assignment_key": str(key)}))
            return 0
        output = evaluate(
            manifest_path,
            Path(args.calibration_dir),
            Path(args.holdout_dir),
            Path(args.assignments),
            Path(args.assignment_key),
            Path(args.ratings),
            Path(args.out_dir),
        )
        report = json.loads(output.read_text())
        print(output)
        return 0 if report["acceptance"]["accepted"] else 1
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as exc:
        print(f"codemap flagship benchmark: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
