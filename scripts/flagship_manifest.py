"""Freeze and validate the pre-registered codemap flagship A/B corpus."""

from __future__ import annotations

import hashlib
import json
import shlex
import shutil
import subprocess
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from codemap_identity import benchmark_binary_identity, command_artifacts, resolve_codemap_command
from flagship_contract import PAIR_ORDER, validate_draft


TASK_CLASSES = {"analysis": 12, "implementation": 12, "negative_control": 6}
SPLITS = {"calibration", "holdout"}
CRITERION_CATEGORIES = {
    "required",
    "behavior",
    "contract",
    "downstream",
    "regression",
    "provenance",
}
EXCLUSION_REASONS = {
    "codex_crash",
    "codex_timeout",
    "protocol_violation",
    "missing_arm",
    "provenance_mismatch",
    "verifier_infrastructure_failure",
    "preflight_no_gap",
}
GATE_FILES = (
    "benchmark-codemap-flagship.py",
    "flagship_acceptance.py",
    "flagship_artifacts.py",
    "flagship_contract.py",
    "flagship_judging.py",
    "flagship_manifest.py",
    "flagship_receipts.py",
    "flagship_stats.py",
    "verify-flagship-acceptance.py",
)


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    rows = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        value = json.loads(line)
        if not isinstance(value, dict):
            raise ValueError(f"{path}:{line_number}: expected JSON object")
        rows.append(value)
    return rows


def command_version(command: str | list[str]) -> str:
    argv = [command] if isinstance(command, str) else command
    result = subprocess.run(
        [*argv, "--version"], capture_output=True, text=True, timeout=30, check=False
    )
    if result.returncode:
        raise ValueError(f"cannot identify {argv[0]}: {result.stderr.strip()}")
    return (result.stdout or result.stderr).strip().splitlines()[0]


def resolve_command(value: str) -> list[str]:
    command = shlex.split(value)
    if not command:
        raise ValueError("empty command")
    executable = Path(command[0]).expanduser()
    if executable.is_absolute() or "/" in command[0] or "\\" in command[0]:
        command[0] = str(executable.resolve())
    else:
        resolved = shutil.which(command[0])
        if not resolved:
            raise ValueError(f"command not found: {command[0]}")
        command[0] = str(Path(resolved).resolve())
    if not Path(command[0]).is_file():
        raise ValueError(f"command not found: {command[0]}")
    return command


def git_commit(repo: Path, ref: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo), "rev-parse", "--verify", f"{ref}^{{commit}}"],
        capture_output=True,
        text=True,
        timeout=30,
        check=False,
    )
    if result.returncode:
        raise ValueError(f"cannot resolve {repo}@{ref}: {result.stderr.strip()}")
    return result.stdout.strip()


def _benchmark_meta(task: dict[str, Any]) -> dict[str, Any]:
    meta = task.get("benchmark")
    if not isinstance(meta, dict):
        raise ValueError(f"task {task.get('id')}: benchmark metadata is required")
    return meta


def validate_tasks(tasks: list[dict[str, Any]]) -> dict[str, Any]:
    ids: set[str] = set()
    class_counts: Counter[str] = Counter()
    split_counts: dict[str, Counter[str]] = defaultdict(Counter)
    repos: dict[str, tuple[str, str]] = {}
    ecosystems: set[str] = set()
    split_repos: dict[str, set[str]] = defaultdict(set)
    split_ecosystems: dict[str, set[str]] = defaultdict(set)
    for task in tasks:
        task_id = task.get("id")
        if not isinstance(task_id, str) or not task_id or task_id in ids:
            raise ValueError(f"invalid or duplicate task id: {task_id!r}")
        ids.add(task_id)
        meta = _benchmark_meta(task)
        task_class = meta.get("task_class")
        split = meta.get("split")
        repo_id = meta.get("repo_id")
        ecosystem = meta.get("ecosystem")
        if task_class not in TASK_CLASSES or split not in SPLITS:
            raise ValueError(f"task {task_id}: invalid class/split")
        if task_class == "analysis" and task.get("mode") != "analysis":
            raise ValueError(f"task {task_id}: analysis class requires analysis mode")
        if task_class != "analysis" and task.get("mode", "implementation") != "implementation":
            raise ValueError(f"task {task_id}: implementation class requires implementation mode")
        if not all(isinstance(value, str) and value for value in (repo_id, ecosystem)):
            raise ValueError(f"task {task_id}: repo_id and ecosystem are required")
        repo = task.get("repo")
        if not isinstance(repo, str) or not repo:
            raise ValueError(f"task {task_id}: repo path is required")
        previous = repos.setdefault(repo_id, (repo, ecosystem))
        if previous != (repo, ecosystem):
            raise ValueError(f"repo_id {repo_id}: inconsistent repo/ecosystem")
        ecosystems.add(ecosystem)
        split_repos[split].add(repo_id)
        split_ecosystems[split].add(ecosystem)
        class_counts[task_class] += 1
        split_counts[task_class][split] += 1
        verifiers = task.get("verify")
        if not isinstance(verifiers, list) or not verifiers:
            raise ValueError(f"task {task_id}: deterministic verifier required")
        names: set[str] = set()
        categories: set[str] = set()
        for verifier in verifiers:
            name = verifier.get("name") if isinstance(verifier, dict) else None
            category = verifier.get("category") if isinstance(verifier, dict) else None
            weight = verifier.get("weight") if isinstance(verifier, dict) else None
            if not isinstance(name, str) or not name or name in names:
                raise ValueError(f"task {task_id}: unique criterion id/name required")
            if category not in CRITERION_CATEGORIES:
                raise ValueError(f"task {task_id}: invalid criterion category {category!r}")
            if isinstance(weight, bool) or not isinstance(weight, (int, float)) or weight <= 0:
                raise ValueError(f"task {task_id}: criterion weight must be positive")
            if verifier.get("scoring") != "deterministic":
                raise ValueError(f"task {task_id}: verifier scoring must be deterministic")
            if not isinstance(verifier.get("evidence_surface"), str) or not verifier["evidence_surface"]:
                raise ValueError(f"task {task_id}: verifier evidence_surface is required")
            command = verifier.get("command")
            if not isinstance(command, list) or not command or not all(
                isinstance(part, str) and part for part in command
            ):
                raise ValueError(f"task {task_id}: verifier command must be an argv array")
            names.add(name)
            categories.add(category)
        ordinal = meta.get("ordinal_criteria", [])
        if task_class == "analysis" and not ordinal:
            raise ValueError(f"task {task_id}: analysis requires pre-registered ordinal criteria")
        for criterion in ordinal:
            if not isinstance(criterion, dict):
                raise ValueError(f"task {task_id}: ordinal criterion must be an object")
            if criterion.get("id") in names or not isinstance(criterion.get("id"), str):
                raise ValueError(f"task {task_id}: criterion ids must be unique")
            if criterion.get("category") not in CRITERION_CATEGORIES:
                raise ValueError(f"task {task_id}: invalid ordinal category")
            if not isinstance(criterion.get("weight"), (int, float)) or criterion["weight"] <= 0:
                raise ValueError(f"task {task_id}: ordinal weight must be positive")
            if not isinstance(criterion.get("max_score"), int) or criterion["max_score"] < 2:
                raise ValueError(f"task {task_id}: ordinal max_score must be >= 2")
            if criterion.get("scoring") != "ordinal":
                raise ValueError(f"task {task_id}: ordinal criterion scoring must be ordinal")
            if not all(
                isinstance(criterion.get(field), str) and criterion[field]
                for field in ("judge_protocol", "evidence_surface")
            ):
                raise ValueError(f"task {task_id}: ordinal judge protocol/evidence required")
            names.add(criterion["id"])
            categories.add(criterion["category"])
        if task_class == "implementation" and not {
            "behavior",
            "contract",
            "downstream",
            "regression",
        }.issubset(categories):
            raise ValueError(f"task {task_id}: implementation rubric lacks a consequence category")
        if "provenance" not in categories or "required" not in categories:
            raise ValueError(f"task {task_id}: required/provenance criteria are mandatory")
        exception = meta.get("exception_criteria")
        if task_class != "negative_control" and (
            not isinstance(exception, list)
            or not exception
            or not set(exception).issubset(names)
        ):
            raise ValueError(f"task {task_id}: pre-registered exception criteria are required")
        if task_class == "negative_control":
            if meta.get("expected_same_outcome") is not True:
                raise ValueError(f"task {task_id}: negative control must require same outcome")
            allowed = meta.get("allowed_exact_entries")
            if not isinstance(allowed, list) or not allowed:
                raise ValueError(f"task {task_id}: allowed_exact_entries are required")
    if class_counts != Counter(TASK_CLASSES):
        raise ValueError(f"task matrix must equal {TASK_CLASSES}; got {dict(class_counts)}")
    for task_class, total in TASK_CLASSES.items():
        expected = total // 2
        if split_counts[task_class] != Counter({"calibration": expected, "holdout": expected}):
            raise ValueError(f"{task_class}: calibration/holdout must be {expected}/{expected}")
    if len(repos) < 6 or len(ecosystems) < 4:
        raise ValueError("corpus requires >=6 repos and >=4 ecosystem families")
    for split in SPLITS:
        if len(split_repos[split]) < 6 or len(split_ecosystems[split]) < 4:
            raise ValueError(f"{split}: requires >=6 repos and >=4 ecosystem families")
    return {
        "tasks": len(tasks),
        "classes": dict(class_counts),
        "splits": {name: dict(counts) for name, counts in split_counts.items()},
        "repos": len(repos),
        "ecosystems": sorted(ecosystems),
    }


def verifier_artifacts(tasks: list[dict[str, Any]], tasks_path: Path) -> list[dict[str, str]]:
    artifacts: dict[str, str] = {}
    for task in tasks:
        declared = _benchmark_meta(task).get("verifier_artifacts", [])
        if not isinstance(declared, list) or not all(
            isinstance(value, str) and value for value in declared
        ):
            raise ValueError(f"task {task['id']}: verifier_artifacts must be a path array")
        for raw in declared:
            candidate = Path(raw).expanduser()
            if not candidate.is_absolute():
                candidate = tasks_path.parent / candidate
            if not candidate.is_file():
                raise ValueError(f"task {task['id']}: verifier artifact missing: {candidate}")
            resolved = candidate.resolve()
            artifacts[str(resolved)] = file_sha256(resolved)
        for verifier in task["verify"]:
            for index, raw in enumerate(verifier["command"]):
                if "{" in raw:
                    continue
                candidate = Path(raw).expanduser()
                resolved_command = shutil.which(raw) if index == 0 else None
                if resolved_command:
                    candidate = Path(resolved_command)
                elif not candidate.is_absolute():
                    candidate = tasks_path.parent / candidate
                if candidate.is_file():
                    resolved = candidate.resolve()
                    artifacts[str(resolved)] = file_sha256(resolved)
    return [{"path": path, "sha256": digest} for path, digest in sorted(artifacts.items())]


def pair_schedule(tasks: list[dict[str, Any]], repetitions: int) -> dict[str, list[dict[str, Any]]]:
    schedule: dict[str, list[dict[str, Any]]] = {}
    for split in sorted(SPLITS):
        selected = [task for task in tasks if _benchmark_meta(task)["split"] == split]
        rows = []
        for task_index, task in enumerate(selected):
            for repetition in range(1, repetitions + 1):
                order = ["control", "codemap"] if (task_index + repetition) % 2 else ["codemap", "control"]
                rows.append({"task_id": task["id"], "repetition": repetition, "arms": order})
        schedule[split] = rows
    return schedule


def freeze_corpus(
    draft_path: Path,
    out_dir: Path,
    codex_bin: str = "codex",
    codemap_bin: str | None = None,
) -> Path:
    draft_path = draft_path.resolve()
    draft = json.loads(draft_path.read_text(encoding="utf-8"))
    if draft.get("kind") != "codemap_flagship_corpus" or draft.get("version") != 1:
        raise ValueError("draft must be codemap_flagship_corpus v1")
    validate_draft(draft)
    tasks_path = Path(draft["tasks_file"])
    if not tasks_path.is_absolute():
        tasks_path = draft_path.parent / tasks_path
    tasks_path = tasks_path.resolve()
    tasks = read_jsonl(tasks_path)
    matrix = validate_tasks(tasks)
    repo_commits: dict[str, dict[str, str]] = {}
    for task in tasks:
        meta = _benchmark_meta(task)
        repo = Path(task["repo"]).expanduser().resolve()
        commit = git_commit(repo, task.get("base_ref", "HEAD"))
        entry = {"path": str(repo), "commit": commit, "ecosystem": meta["ecosystem"]}
        previous = repo_commits.setdefault(meta["repo_id"], entry)
        if previous != entry:
            raise ValueError(f"repo {meta['repo_id']}: tasks must share one frozen commit")
    root = Path(__file__).resolve().parents[1]
    codex_command = resolve_command(codex_bin)
    codemap_command, resolution = resolve_codemap_command(codemap_bin, root)
    codemap_identity = benchmark_binary_identity(codemap_command, resolution, root)
    frozen = dict(draft)
    frozen.update(
        {
            "state": "frozen",
            "frozen_at": datetime.now(timezone.utc).isoformat(),
            "tasks_file": str(tasks_path),
            "tasks_sha256": file_sha256(tasks_path),
            "matrix": matrix,
            "repositories": repo_commits,
            "codex_command": codex_command,
            "codex_version": command_version(codex_command),
            "codex_artifacts": command_artifacts(codex_command),
            "codemap_identity": codemap_identity,
            "harness_sha256": file_sha256(root / "scripts/benchmark-codemap-ab.py"),
            "protocol_sha256": file_sha256(root / "scripts/codemap_protocol.py"),
            "manifest_owner_sha256": file_sha256(Path(__file__)),
            "gate_artifacts": [
                {"path": f"scripts/{name}", "sha256": file_sha256(root / "scripts" / name)}
                for name in GATE_FILES
            ],
            "verifier_artifacts": verifier_artifacts(tasks, tasks_path),
            "pair_schedule": pair_schedule(tasks, draft["repetitions"]),
        }
    )
    exclusions = set(frozen.get("allowed_exclusions", []))
    if exclusions != EXCLUSION_REASONS:
        raise ValueError(f"allowed_exclusions must equal {sorted(EXCLUSION_REASONS)}")
    out_dir.mkdir(parents=True, exist_ok=False)
    for split in sorted(SPLITS):
        selected = [task for task in tasks if _benchmark_meta(task)["split"] == split]
        body = "\n".join(json.dumps(task, sort_keys=True) for task in selected) + "\n"
        (out_dir / f"{split}.tasks.jsonl").write_text(body, encoding="utf-8")
        frozen[f"{split}_tasks_sha256"] = hashlib.sha256(body.encode()).hexdigest()
    output = out_dir / "corpus-manifest.json"
    output.write_text(json.dumps(frozen, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return output


def load_frozen(path: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    manifest = json.loads(path.read_text(encoding="utf-8"))
    if manifest.get("state") != "frozen" or manifest.get("version") != 1:
        raise ValueError("flagship corpus is not frozen v1")
    tasks_path = Path(manifest["tasks_file"])
    if file_sha256(tasks_path) != manifest.get("tasks_sha256"):
        raise ValueError("frozen task manifest hash mismatch")
    tasks = read_jsonl(tasks_path)
    validate_tasks(tasks)
    validate_draft(manifest)
    if pair_schedule(tasks, manifest["repetitions"]) != manifest.get("pair_schedule"):
        raise ValueError("frozen pair schedule mismatch")
    for artifact in manifest.get("verifier_artifacts", []):
        artifact_path = Path(artifact["path"])
        if not artifact_path.is_file() or file_sha256(artifact_path) != artifact["sha256"]:
            raise ValueError(f"verifier artifact changed: {artifact_path}")
    return manifest, tasks
