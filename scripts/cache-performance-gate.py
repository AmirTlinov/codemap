#!/usr/bin/env python3
"""Deterministic cache truth and warm-latency gate."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import shutil
import statistics
import subprocess
import tempfile
import time
from pathlib import Path


BUDGETS_MS = {
    "cold_scan": 15_000,
    "warm_where": 700,
    "warm_cone": 700,
    "warm_ls_root": 1_000,
    "warm_changed_100": 2_000,
    "warm_proof_changed_100": 2_000,
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", default="target/release/codemap")
    parser.add_argument("--files", type=int, default=1_200)
    parser.add_argument("--dirty", type=int, default=100)
    parser.add_argument("--output", default="target/cache-performance.json")
    parser.add_argument("--strict", action="store_true")
    parser.add_argument("--keep", action="store_true")
    return parser.parse_args()


def command(
    binary: Path,
    repo: Path,
    cache: Path,
    args: list[str],
    *,
    expect_json: bool = False,
) -> tuple[int, str, str, int]:
    env = dict(os.environ, CODEMAP_CACHE_DIR=str(cache), CODEMAP_BRIEF="1")
    started = time.perf_counter_ns()
    result = subprocess.run(
        [str(binary), "--root", str(repo), *args],
        cwd=repo,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    elapsed_ms = (time.perf_counter_ns() - started) // 1_000_000
    if result.returncode != 0:
        raise RuntimeError(
            f"command failed ({result.returncode}): {' '.join(args)}\n{result.stderr}"
        )
    if expect_json:
        json.loads(result.stdout)
    return result.returncode, result.stdout, result.stderr, elapsed_ms


def write_fixture(repo: Path, files: int) -> None:
    (repo / "src").mkdir(parents=True)
    (repo / "tests").mkdir(parents=True)
    (repo / "package.json").write_text(
        json.dumps(
            {
                "name": "codemap-cache-performance",
                "private": True,
                "scripts": {"test": "node --test", "check": "tsc --noEmit"},
            }
        )
        + "\n"
    )
    (repo / "src/shared_a.ts").write_text("export const sharedA = 1;\n")
    (repo / "src/shared_b.ts").write_text("export const sharedB = 2;\n")
    for index in range(files):
        (repo / f"src/mod{index:04d}.ts").write_text(
            "import { sharedA } from './shared_a';\n"
            f"export function Symbol{index:04d}() {{ return sharedA + {index}; }}\n"
        )
    for index in range(max(12, files // 20)):
        (repo / f"tests/mod{index:04d}.test.ts").write_text(
            f"import {{ Symbol{index:04d} }} from '../src/mod{index:04d}';\n"
            f"void Symbol{index:04d}();\n"
        )
    git(repo, "init", "-q")
    git(repo, "config", "user.email", "cache@example.com")
    git(repo, "config", "user.name", "cache-gate")
    git(repo, "add", ".")
    git(repo, "commit", "-qm", "fixture")


def dirty_fixture(repo: Path, dirty: int) -> None:
    for index in range(dirty):
        (repo / f"src/mod{index:04d}.ts").write_text(
            "import { sharedB } from './shared_b';\n"
            f"export function Symbol{index:04d}() {{ return sharedB + {index} + 1; }}\n"
        )


def git(repo: Path, *args: str) -> None:
    subprocess.run(["git", "-C", str(repo), *args], check=True)


def project_cache(cache: Path) -> Path:
    candidates = [path for path in cache.iterdir() if (path / "status.json").exists()]
    if len(candidates) != 1:
        raise RuntimeError(f"expected one project cache, found {candidates}")
    return candidates[0]


def timed_probe(
    results: dict[str, object],
    name: str,
    binary: Path,
    repo: Path,
    cache: Path,
    args: list[str],
    *,
    samples: int = 1,
) -> str:
    observed = [command(binary, repo, cache, args) for _ in range(samples)]
    stdout = observed[-1][1]
    elapsed_samples = [item[3] for item in observed]
    elapsed = int(statistics.median(elapsed_samples))
    results[name] = {
        "elapsed_ms": elapsed,
        "samples_ms": elapsed_samples,
        "budget_ms": BUDGETS_MS[name],
        "status": "ok" if elapsed <= BUDGETS_MS[name] else "over",
        "command": ["codemap", *args],
    }
    return stdout


def semantic_payload(text: str) -> object:
    value = json.loads(text)
    identity = value.get("build_identity")
    if isinstance(identity, dict):
        identity.pop("binary_sha256", None)
        identity.pop("binary_sha256_state", None)
    return value


def run_gate(args: argparse.Namespace) -> dict[str, object]:
    binary = Path(args.binary).resolve()
    if not binary.is_file():
        raise RuntimeError(f"binary not found: {binary}")
    work = Path(tempfile.mkdtemp(prefix="codemap-cache-gate-"))
    repo, cache = work / "repo", work / "cache"
    repo.mkdir()
    cache.mkdir()
    try:
        write_fixture(repo, args.files)
        probes: dict[str, object] = {}
        timed_probe(probes, "cold_scan", binary, repo, cache, ["where", "Symbol0000"])
        baseline = json.loads((project_cache(cache) / "status.json").read_text())["fingerprint"]
        timed_probe(
            probes, "warm_where", binary, repo, cache, ["where", "Symbol0000"], samples=3
        )
        timed_probe(
            probes,
            "warm_cone",
            binary,
            repo,
            cache,
            ["cone", "src/shared_a.ts"],
            samples=3,
        )
        timed_probe(probes, "warm_ls_root", binary, repo, cache, ["ls", "."], samples=3)

        dirty_fixture(repo, args.dirty)
        _, profile_text, _, _ = command(
            binary, repo, cache, ["doctor", "--format", "json"], expect_json=True
        )
        profile = json.loads(profile_text)
        changed_args = ["changed", "--since", baseline]
        timed_probe(probes, "warm_changed_100", binary, repo, cache, changed_args, samples=3)
        changed_json_args = [*changed_args, "--format", "json"]
        _, first_changed, _, _ = command(
            binary, repo, cache, changed_json_args, expect_json=True
        )
        _, second_changed, _, _ = command(
            binary, repo, cache, changed_json_args, expect_json=True
        )
        proof_args = ["proof", "changed", "--since", baseline]
        timed_probe(
            probes, "warm_proof_changed_100", binary, repo, cache, proof_args, samples=3
        )
        proof_json_args = [*proof_args, "--format", "json"]
        _, first_proof, _, _ = command(
            binary, repo, cache, proof_json_args, expect_json=True
        )
        _, second_proof, _, _ = command(
            binary, repo, cache, proof_json_args, expect_json=True
        )

        changed_parity = semantic_payload(first_changed) == semantic_payload(second_changed)
        proof_parity = semantic_payload(first_proof) == semantic_payload(second_proof)
        cache_work = profile["cache_work"]
        causal_checks = {
            "selected_files": json.loads(first_changed)["selection"]["selected_files"],
            "changed_output_parity": changed_parity,
            "proof_output_parity": proof_parity,
            "per_file_facts_rebuilt": cache_work["per_file_facts_rebuilt"],
            "per_file_facts_reused": cache_work["per_file_facts_reused"],
            "reverse_import_strategy": cache_work["reverse_import_strategy"],
            "reverse_import_targets_rebuilt": cache_work[
                "reverse_import_targets_rebuilt"
            ],
        }
        checks_ok = (
            causal_checks["selected_files"] == args.dirty
            and changed_parity
            and proof_parity
            and cache_work["per_file_facts_rebuilt"] == args.dirty
            and cache_work["per_file_facts_reused"] >= args.files - args.dirty
            and cache_work["reverse_import_strategy"] == "affected"
            and 0 < cache_work["reverse_import_targets_rebuilt"] < args.dirty
        )
        budgets_ok = all(probe["status"] == "ok" for probe in probes.values())
        changed_total_ms = probes["warm_changed_100"]["elapsed_ms"]
        project_total_ms = profile["timings"]["total_ms"]
        report = {
            "kind": "cache_performance_gate",
            "schema_version": 1,
            "strict": args.strict,
            "status": "pass" if checks_ok and budgets_ok else "fail",
            "hardware": {
                "os": platform.platform(),
                "machine": platform.machine(),
                "processor": platform.processor(),
                "cpu_count": os.cpu_count(),
            },
            "binary": {
                "path": str(binary),
                "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
            },
            "repo_scale": {"files": args.files, "dirty_files": args.dirty},
            "cache_state": {"cold_probe": "empty", "warm_probes": "primed"},
            "phase_profile": {
                "project_load": profile["timings"],
                "changed_total_ms": changed_total_ms,
                "changed_map_and_render_remainder_ms": max(
                    0, changed_total_ms - project_total_ms
                ),
            },
            "causal_checks": causal_checks,
            "probes": probes,
        }
        if args.keep:
            report["fixture"] = str(work)
        return report
    finally:
        if not args.keep:
            shutil.rmtree(work, ignore_errors=True)


def main() -> int:
    args = parse_args()
    report = run_gate(args)
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))
    return 1 if args.strict and report["status"] != "pass" else 0


if __name__ == "__main__":
    raise SystemExit(main())
