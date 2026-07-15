#!/usr/bin/env python3
"""Materialize history-free S15 task repositories and a runnable frozen-corpus draft."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
VERIFY = ROOT / "benchmarks/flagship/verify.py"


def run(argv: list[str], cwd: Path | None = None, timeout: int = 300) -> str:
    result = subprocess.run(argv, cwd=cwd, capture_output=True, text=True, timeout=timeout)
    if result.returncode:
        raise ValueError(f"command failed ({result.returncode}): {' '.join(argv)}\n{result.stderr}")
    return result.stdout.strip()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def clone_snapshot(repo: dict[str, Any], target: Path, remote_only: bool) -> dict[str, Any]:
    target.mkdir(parents=True)
    run(["git", "init", "-q", "-b", "benchmark", str(target)])
    source = repo["remote"] if remote_only else repo.get("source", repo["remote"])
    run(["git", "-C", str(target), "remote", "add", "source", source])
    run(["git", "-C", str(target), "fetch", "--depth", "1", "source", repo["base"]], timeout=1800)
    run(["git", "-C", str(target), "checkout", "-q", "--detach", "FETCH_HEAD"])
    actual = run(["git", "-C", str(target), "rev-parse", "HEAD"])
    if actual != repo["base"]:
        raise ValueError(f"{repo['id']}: expected {repo['base']}, got {actual}")
    for mutation in repo["negative_mutations"]:
        path = target / mutation["path"]
        body = path.read_text(encoding="utf-8")
        if body.count(mutation["before"]) != 1:
            raise ValueError(f"{repo['id']}: mutation anchor is not unique: {mutation['path']}")
        path.write_text(body.replace(mutation["before"], mutation["after"], 1), encoding="utf-8")
    run(["git", "-C", str(target), "add", "."])
    run(
        [
            "git",
            "-C",
            str(target),
            "-c",
            "user.name=codemap flagship corpus",
            "-c",
            "user.email=flagship@codemap.invalid",
            "commit",
            "-qm",
            "benchmark: seed exact local control",
        ]
    )
    run(["git", "-C", str(target), "remote", "remove", "source"])
    run(["git", "-C", str(target), "remote", "add", "origin", repo["remote"]])
    return {
        "repo_id": repo["id"],
        "remote": repo["remote"],
        "source_commit": repo["base"],
        "benchmark_commit": run(["git", "-C", str(target), "rev-parse", "HEAD"]),
        "mutation_paths": [row["path"] for row in repo["negative_mutations"]],
    }


def oracle_source(repo: dict[str, Any], remote_only: bool, oracle_root: Path) -> Path:
    if not remote_only and Path(repo.get("source", "")).is_dir():
        return Path(repo["source"])
    path = oracle_root / repo["id"]
    run(["git", "clone", "-q", "--filter=blob:none", "--no-checkout", repo["remote"], str(path)], timeout=1800)
    return path


def extract_oracles(
    repo: dict[str, Any], tasks: list[dict[str, Any]], source: Path, out_dir: Path
) -> dict[str, list[dict[str, str]]]:
    extracted: dict[str, list[dict[str, str]]] = {}
    for task in tasks:
        if task["repo_id"] != repo["id"]:
            continue
        rows = []
        for overlay in task.get("overlays", []):
            body = run(["git", "-C", str(source), "show", f"{overlay['commit']}:{overlay['path']}"], timeout=120)
            target = out_dir / task["id"] / overlay["path"]
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(body + ("" if body.endswith("\n") else "\n"), encoding="utf-8")
            rows.append({"source": str(target.resolve()), "target": overlay["path"]})
        extracted[task["id"]] = rows
    return extracted


def replace_overlays(value: Any, overlays: list[dict[str, str]]) -> Any:
    if isinstance(value, dict):
        return {key: replace_overlays(item, overlays) for key, item in value.items()}
    if isinstance(value, list):
        return [replace_overlays(item, overlays) for item in value]
    if value == "{task_overlays}":
        return overlays
    return value


def verifier_row(spec: Path, task: dict[str, Any], criterion: str) -> dict[str, Any]:
    weights = {"required": 3, "behavior": 4, "contract": 3, "downstream": 2, "regression": 2, "provenance": 1}
    return {
        "name": criterion,
        "category": criterion,
        "weight": weights[criterion],
        "required": criterion in {"required", "provenance"},
        "scoring": "deterministic",
        "evidence_surface": f"external verifier receipt:{task['id']}:{criterion}",
        "command": [
            sys.executable,
            str(VERIFY),
            str(spec),
            task["id"],
            criterion,
            "{worktree}",
            "{last_message}",
        ],
        "timeout_seconds": task.get("verifier_timeout_seconds", 900),
    }


def materialize(blueprint_path: Path, out_dir: Path, remote_only: bool) -> Path:
    blueprint = json.loads(blueprint_path.read_text(encoding="utf-8"))
    out_dir.mkdir(parents=True, exist_ok=False)
    repos_dir = out_dir / "repositories"
    oracles_dir = out_dir / "oracles"
    oracle_sources = out_dir / ".oracle-sources"
    receipts = []
    repo_paths: dict[str, Path] = {}
    all_overlays: dict[str, list[dict[str, str]]] = {}
    for repo in blueprint["repositories"]:
        repo_path = repos_dir / repo["id"]
        receipts.append(clone_snapshot(repo, repo_path, remote_only))
        repo_paths[repo["id"]] = repo_path
        source = oracle_source(repo, remote_only, oracle_sources)
        all_overlays.update(extract_oracles(repo, blueprint["tasks"], source, oracles_dir))
    if oracle_sources.exists():
        shutil.rmtree(oracle_sources)
    spec = {"kind": "codemap_flagship_verification_spec", "version": 1, "tasks": {}}
    spec_path = out_dir / "verification-spec.json"
    for task in blueprint["tasks"]:
        actions = replace_overlays(task["criteria"], all_overlays[task["id"]])
        actions["provenance"] = {
            "kind": "git_head",
            "commit": next(row["benchmark_commit"] for row in receipts if row["repo_id"] == task["repo_id"]),
        }
        spec["tasks"][task["id"]] = actions
    spec_path.write_text(json.dumps(spec, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    artifacts = [str(VERIFY.resolve()), str(spec_path.resolve())]
    artifacts.extend(row["source"] for rows in all_overlays.values() for row in rows)
    artifacts.extend(
        str((blueprint_path.parent / path).resolve())
        for path in blueprint.get("verifier_artifacts", [])
    )
    tasks_path = out_dir / "tasks.jsonl"
    task_rows = []
    for task in blueprint["tasks"]:
        criteria = list(spec["tasks"][task["id"]])
        meta = {
            "repo_id": task["repo_id"],
            "ecosystem": task["ecosystem"],
            "task_class": task["task_class"],
            "split": task["split"],
            "ordinal_criteria": task.get("ordinal_criteria", []),
            "exception_criteria": task.get("exception_criteria", []),
            "verifier_artifacts": sorted(set(artifacts)),
        }
        if task["task_class"] == "negative_control":
            meta.update({"expected_same_outcome": True, "allowed_exact_entries": task["allowed_exact_entries"]})
        task_rows.append(
            {
                "id": task["id"],
                "mode": "analysis" if task["task_class"] == "analysis" else "implementation",
                "repo": str(repo_paths[task["repo_id"]].resolve()),
                "base_ref": "HEAD",
                "prompt": task["prompt"],
                "verify": [verifier_row(spec_path.resolve(), task, name) for name in criteria],
                "protected_paths": task.get("protected_paths", []),
                "benchmark": meta,
            }
        )
    tasks_path.write_text("\n".join(json.dumps(row, sort_keys=True) for row in task_rows) + "\n")
    draft = {**blueprint["experiment"], "kind": "codemap_flagship_corpus", "version": 1, "tasks_file": str(tasks_path.resolve())}
    draft_path = out_dir / "corpus-draft.json"
    draft_path.write_text(json.dumps(draft, indent=2, sort_keys=True) + "\n")
    receipt = {"kind": "codemap_flagship_materialization", "version": 1, "blueprint_sha256": sha256(blueprint_path), "repositories": receipts, "tasks_sha256": sha256(tasks_path), "spec_sha256": sha256(spec_path)}
    (out_dir / "materialization-receipt.json").write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n")
    return draft_path


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("blueprint")
    parser.add_argument("--out-dir", required=True)
    parser.add_argument("--remote-only", action="store_true")
    args = parser.parse_args(argv)
    try:
        output = materialize(Path(args.blueprint).resolve(), Path(args.out_dir).resolve(), args.remote_only)
        print(output)
        return 0
    except (OSError, ValueError, KeyError, json.JSONDecodeError, subprocess.TimeoutExpired) as exc:
        print(f"codemap flagship materializer: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
