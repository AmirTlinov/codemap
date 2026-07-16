#!/usr/bin/env python3
"""Materialize six pinned repositories and the 18-task flagship corpus draft."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
VERIFY = ROOT / "benchmarks/flagship/verify.py"


def run(
    argv: list[str],
    cwd: Path | None = None,
    timeout: int = 300,
    env: dict[str, str] | None = None,
) -> str:
    result = subprocess.run(
        argv, cwd=cwd, capture_output=True, text=True, timeout=timeout, env=env
    )
    if result.returncode:
        raise ValueError(f"command failed ({result.returncode}): {argv!r}\n{result.stderr}")
    return result.stdout.strip()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def clone_snapshot(
    repo: dict[str, Any],
    target: Path,
    remote_only: bool,
    variant: str,
    source_override: Path | None = None,
) -> dict[str, str]:
    target.mkdir(parents=True)
    run(["git", "init", "-q", "-b", "benchmark", str(target)])
    local_source = Path(repo.get("source", ""))
    source = str(source_override) if source_override is not None else repo["remote"]
    if source_override is None and not remote_only and local_source.is_dir():
        source = str(local_source)
    run(["git", "-C", str(target), "remote", "add", "source", source])
    run(
        ["git", "-C", str(target), "fetch", "--depth", "1", "source", repo["base"]],
        timeout=1800,
    )
    run(["git", "-C", str(target), "checkout", "-q", "--detach", "FETCH_HEAD"])
    actual = run(["git", "-C", str(target), "rev-parse", "HEAD"])
    if actual != repo["base"]:
        raise ValueError(f"{repo['id']}: expected {repo['base']}, got {actual}")
    mutations = repo.get("exact_mutations", []) if variant == "exact" else []
    for mutation in mutations:
        path = target / mutation["path"]
        body = path.read_text(encoding="utf-8")
        if body.count(mutation["before"]) != 1:
            raise ValueError(f"{repo['id']}: mutation anchor is not unique: {mutation['path']}")
        path.write_text(body.replace(mutation["before"], mutation["after"], 1), encoding="utf-8")
    if mutations:
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
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-qm",
                "benchmark: seed exact local control",
            ],
            env={
                **os.environ,
                "GIT_AUTHOR_DATE": "2000-01-01T00:00:00Z",
                "GIT_COMMITTER_DATE": "2000-01-01T00:00:00Z",
            },
        )
    run(["git", "-C", str(target), "remote", "remove", "source"])
    run(["git", "-C", str(target), "remote", "add", "origin", repo["remote"]])
    return {
        "repo_id": repo["id"],
        "variant": variant,
        "remote": repo["remote"],
        "source_commit": repo["base"],
        "benchmark_commit": run(["git", "-C", str(target), "rev-parse", "HEAD"]),
    }


def oracle_source(repo: dict[str, Any], remote_only: bool, root: Path) -> Path:
    source = Path(repo.get("source", ""))
    if not remote_only and source.is_dir():
        return source
    path = root / repo["id"]
    run(
        ["git", "clone", "-q", "--filter=blob:none", "--no-checkout", repo["remote"], str(path)],
        timeout=1800,
    )
    return path


def extract_overlays(
    repo: dict[str, Any],
    tasks: list[dict[str, Any]],
    source: Path,
    artifact_root: Path,
    out_dir: Path,
) -> dict[str, list[dict[str, str]]]:
    extracted = {}
    for task in tasks:
        if task["repo_id"] != repo["id"]:
            continue
        rows = []
        for overlay in task.get("overlays", []):
            target = out_dir / task["id"] / overlay["path"]
            target.parent.mkdir(parents=True, exist_ok=True)
            if "artifact" in overlay:
                artifact = artifact_root / overlay["artifact"]
                if not artifact.is_file():
                    raise ValueError(f"missing verifier artifact: {artifact}")
                shutil.copyfile(artifact, target)
            else:
                result = subprocess.run(
                    [
                        "git",
                        "-C",
                        str(source),
                        "show",
                        f"{overlay['commit']}:{overlay['path']}",
                    ],
                    capture_output=True,
                    timeout=120,
                )
                if result.returncode:
                    raise ValueError(
                        f"cannot extract verifier overlay {overlay['path']}: "
                        f"{result.stderr.decode(errors='replace')}"
                    )
                target.write_bytes(result.stdout)
            rows.append({"source": str(target.resolve()), "target": overlay["path"]})
        extracted[task["id"]] = rows
    return extracted


def replace_overlays(value: Any, overlays: list[dict[str, str]]) -> Any:
    if isinstance(value, dict):
        return {key: replace_overlays(item, overlays) for key, item in value.items()}
    if isinstance(value, list):
        return [replace_overlays(item, overlays) for item in value]
    return overlays if value == "{task_overlays}" else value


def ordered_criteria(task: dict[str, Any], overlays: list[dict[str, str]]) -> dict[str, Any]:
    criteria = task["criteria"]
    names = ["required"] if "required" in criteria else []
    names.extend(name for name in criteria if name != "required")
    return {name: replace_overlays(criteria[name], overlays) for name in names}


def verifier_row(spec: Path, task: dict[str, Any], criterion: str) -> dict[str, Any]:
    action = task["criteria"].get(
        criterion, {"category": "provenance", "weight": 1, "required": True}
    )
    return {
        "name": criterion,
        "category": action.get("category", criterion),
        "weight": action.get("weight", 1),
        "required": action.get("required", True),
        "scoring": "deterministic",
        "evidence_surface": f"external verifier:{task['id']}:{criterion}",
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
    repositories: dict[tuple[str, str], Path] = {}
    receipts = []
    overlays = {}
    oracle_root = out_dir / ".oracle-sources"
    for repo in blueprint["repositories"]:
        clean = out_dir / "repositories" / f"{repo['id']}-clean"
        exact = out_dir / "repositories" / f"{repo['id']}-exact"
        receipts.append(clone_snapshot(repo, clean, remote_only, "clean"))
        receipts.append(clone_snapshot(repo, exact, remote_only, "exact", clean))
        repositories[(repo["id"], "clean")] = clean
        repositories[(repo["id"], "exact")] = exact
        source = oracle_source(repo, remote_only, oracle_root)
        overlays.update(
            extract_overlays(
                repo,
                blueprint["tasks"],
                source,
                blueprint_path.parent,
                out_dir / "oracles",
            )
        )
    if oracle_root.exists():
        shutil.rmtree(oracle_root)
    spec = {"kind": "codemap_flagship_verification_spec", "version": 1, "tasks": {}}
    for task in blueprint["tasks"]:
        variant = "exact" if task["task_class"] == "exact_control" else "clean"
        actions = ordered_criteria(task, overlays[task["id"]])
        actions["provenance"] = {
            "kind": "git_head",
            "commit": next(
                row["benchmark_commit"]
                for row in receipts
                if row["repo_id"] == task["repo_id"] and row["variant"] == variant
            ),
            "category": "provenance",
            "weight": 1,
            "required": True,
        }
        spec["tasks"][task["id"]] = actions
    spec_path = out_dir / "verification-spec.json"
    spec_path.write_text(json.dumps(spec, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    artifacts = [str(VERIFY.resolve()), str(spec_path.resolve())]
    artifacts.extend(row["source"] for rows in overlays.values() for row in rows)
    artifacts.extend(
        str((blueprint_path.parent / path).resolve())
        for path in blueprint.get("verifier_artifacts", [])
    )
    task_rows = []
    for task in blueprint["tasks"]:
        task_class = task["task_class"]
        variant = "exact" if task_class == "exact_control" else "clean"
        meta = {
            "repo_id": task["repo_id"],
            "repo_variant": variant,
            "ecosystem": task["ecosystem"],
            "task_class": task_class,
            "verifier_artifacts": sorted(set(artifacts)),
        }
        if task_class == "exact_control":
            meta["allowed_exact_entries"] = task["allowed_exact_entries"]
        task_rows.append(
            {
                "id": task["id"],
                "mode": "analysis" if task_class == "investigation" else "implementation",
                "repo": str(repositories[(task["repo_id"], variant)].resolve()),
                "base_ref": "HEAD",
                "prompt": task["prompt"],
                "verify": [
                    verifier_row(spec_path.resolve(), task, name)
                    for name in spec["tasks"][task["id"]]
                ],
                "benchmark": meta,
            }
        )
    tasks_path = out_dir / "tasks.jsonl"
    tasks_path.write_text(
        "\n".join(json.dumps(row, sort_keys=True) for row in task_rows) + "\n",
        encoding="utf-8",
    )
    draft = {
        **blueprint["experiment"],
        "kind": "codemap_flagship_corpus",
        "version": 1,
        "tasks_file": str(tasks_path.resolve()),
    }
    draft_path = out_dir / "corpus-draft.json"
    draft_path.write_text(json.dumps(draft, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    receipt = {
        "kind": "codemap_flagship_materialization",
        "version": 1,
        "blueprint_sha256": sha256(blueprint_path),
        "repositories": receipts,
        "tasks_sha256": sha256(tasks_path),
        "spec_sha256": sha256(spec_path),
    }
    (out_dir / "materialization-receipt.json").write_text(
        json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    return draft_path


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("blueprint")
    parser.add_argument("--out-dir", required=True)
    parser.add_argument("--remote-only", action="store_true")
    args = parser.parse_args(argv)
    try:
        print(materialize(Path(args.blueprint).resolve(), Path(args.out_dir).resolve(), args.remote_only))
        return 0
    except (OSError, ValueError, KeyError, json.JSONDecodeError, subprocess.TimeoutExpired) as exc:
        print(f"codemap flagship materializer: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
