"""Freeze the task, verifier, repository, binary, and arm identities for flagship A/B."""

from __future__ import annotations

import hashlib
import json
import shutil
import subprocess
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from codemap_identity import benchmark_binary_identity, command_artifacts, resolve_codemap_command
from flagship_contract import PAIR_ORDER, TASK_CLASSES, validate_draft


ARMS = ("control", "codemap")


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def stable_sha256(value: Any) -> str:
    body = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(body.encode("utf-8")).hexdigest()


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


def resolve_command(value: str | list[str]) -> list[str]:
    command = [value] if isinstance(value, str) else list(value)
    if not command or not all(isinstance(part, str) and part for part in command):
        raise ValueError("command must be a non-empty argv array")
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


def command_version(command: list[str]) -> str:
    result = subprocess.run(
        [*command, "--version"], capture_output=True, text=True, timeout=30, check=False
    )
    if result.returncode:
        raise ValueError(f"cannot identify {command[0]}: {result.stderr.strip()}")
    return (result.stdout or result.stderr).strip().splitlines()[0]


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


def task_meta(task: dict[str, Any]) -> dict[str, Any]:
    meta = task.get("benchmark")
    if not isinstance(meta, dict):
        raise ValueError(f"task {task.get('id')}: benchmark metadata is required")
    return meta


def validate_tasks(tasks: list[dict[str, Any]]) -> dict[str, Any]:
    ids: set[str] = set()
    classes: Counter[str] = Counter()
    repo_classes: dict[str, Counter[str]] = defaultdict(Counter)
    repo_variants: dict[tuple[str, str], tuple[str, str]] = {}
    repo_ecosystems: dict[str, str] = {}
    ecosystems: set[str] = set()
    for task in tasks:
        task_id = task.get("id")
        if not isinstance(task_id, str) or not task_id or task_id in ids:
            raise ValueError(f"invalid or duplicate task id: {task_id!r}")
        ids.add(task_id)
        meta = task_meta(task)
        task_class = meta.get("task_class")
        repo_id = meta.get("repo_id")
        repo_variant = meta.get("repo_variant", "default")
        ecosystem = meta.get("ecosystem")
        if task_class not in TASK_CLASSES:
            raise ValueError(f"task {task_id}: invalid task_class")
        expected_mode = "analysis" if task_class == "investigation" else "implementation"
        if task.get("mode", "implementation") != expected_mode:
            raise ValueError(f"task {task_id}: {task_class} requires {expected_mode} mode")
        if not all(isinstance(value, str) and value for value in (repo_id, ecosystem)):
            raise ValueError(f"task {task_id}: repo_id and ecosystem are required")
        repo = task.get("repo")
        if not isinstance(repo, str) or not repo:
            raise ValueError(f"task {task_id}: repo path is required")
        previous = repo_variants.setdefault(
            (repo_id, repo_variant), (str(Path(repo).expanduser().resolve()), ecosystem)
        )
        if previous != (str(Path(repo).expanduser().resolve()), ecosystem):
            raise ValueError(f"repo {repo_id}:{repo_variant}: inconsistent path or ecosystem")
        if repo_ecosystems.setdefault(repo_id, ecosystem) != ecosystem:
            raise ValueError(f"repo {repo_id}: inconsistent ecosystem")
        prompt = task.get("prompt")
        if not isinstance(prompt, str) or not prompt.strip():
            raise ValueError(f"task {task_id}: prompt is required")
        verifiers = task.get("verify")
        if not isinstance(verifiers, list) or not verifiers:
            raise ValueError(f"task {task_id}: deterministic verifier required")
        names: set[str] = set()
        required = 0
        for verifier in verifiers:
            if not isinstance(verifier, dict):
                raise ValueError(f"task {task_id}: verifier must be an object")
            name = verifier.get("name")
            command = verifier.get("command")
            weight = verifier.get("weight")
            if not isinstance(name, str) or not name or name in names:
                raise ValueError(f"task {task_id}: verifier names must be unique")
            if verifier.get("scoring") != "deterministic":
                raise ValueError(f"task {task_id}: every criterion must be deterministic")
            if not isinstance(verifier.get("category"), str) or not verifier["category"]:
                raise ValueError(f"task {task_id}: verifier category is required")
            if isinstance(weight, bool) or not isinstance(weight, (int, float)) or weight <= 0:
                raise ValueError(f"task {task_id}: verifier weight must be positive")
            if not isinstance(verifier.get("required", True), bool):
                raise ValueError(f"task {task_id}: verifier required must be boolean")
            if verifier.get("required", True):
                required += 1
            if not isinstance(command, list) or not command or not all(
                isinstance(part, str) and part for part in command
            ):
                raise ValueError(f"task {task_id}: verifier command must be argv")
            if not isinstance(verifier.get("evidence_surface"), str):
                raise ValueError(f"task {task_id}: verifier evidence_surface is required")
            names.add(name)
        if required == 0:
            raise ValueError(f"task {task_id}: at least one required criterion is mandatory")
        classes[task_class] += 1
        repo_classes[repo_id][task_class] += 1
        ecosystems.add(ecosystem)
    if classes != Counter(TASK_CLASSES):
        raise ValueError(f"task matrix must equal {TASK_CLASSES}; got {dict(classes)}")
    if len(repo_ecosystems) != 6 or len(ecosystems) < 4:
        raise ValueError("corpus requires exactly 6 repos across at least 4 ecosystems")
    expected_per_repo = Counter({name: 1 for name in TASK_CLASSES})
    for repo_id, counts in repo_classes.items():
        if counts != expected_per_repo:
            raise ValueError(f"repo {repo_id}: requires one task of each class")
    return {"tasks": len(tasks), "classes": dict(classes), "repos": 6, "ecosystems": sorted(ecosystems)}


def verifier_artifacts(tasks: list[dict[str, Any]], tasks_path: Path) -> list[dict[str, str]]:
    artifacts: dict[str, str] = {}
    for task in tasks:
        declared = task_meta(task).get("verifier_artifacts", [])
        if not isinstance(declared, list):
            raise ValueError(f"task {task['id']}: verifier_artifacts must be an array")
        candidates = list(declared)
        for verifier in task["verify"]:
            candidates.extend(part for part in verifier["command"] if "{" not in part)
        for index, raw in enumerate(candidates):
            candidate = Path(raw).expanduser()
            resolved_command = shutil.which(raw) if index == 0 else None
            if resolved_command:
                candidate = Path(resolved_command)
            elif not candidate.is_absolute():
                candidate = tasks_path.parent / candidate
            if candidate.is_file():
                resolved = candidate.resolve()
                artifacts[str(resolved)] = file_sha256(resolved)
            elif raw in declared:
                raise ValueError(f"task {task['id']}: verifier artifact missing: {candidate}")
    return [{"path": path, "sha256": digest} for path, digest in sorted(artifacts.items())]


def pair_schedule(tasks: list[dict[str, Any]]) -> list[dict[str, Any]]:
    rows = []
    for task_index, task in enumerate(tasks):
        for repetition in (1, 2):
            arms = list(ARMS) if (task_index + repetition) % 2 else list(reversed(ARMS))
            rows.append({"task_id": task["id"], "repetition": repetition, "arms": arms})
    return rows


def task_records(tasks: list[dict[str, Any]]) -> list[dict[str, str]]:
    return [
        {
            "id": task["id"],
            "prompt_sha256": hashlib.sha256(task["prompt"].strip().encode()).hexdigest(),
            "verifiers_sha256": stable_sha256(task["verify"]),
        }
        for task in tasks
    ]


def freeze_corpus(
    draft_path: Path,
    out_dir: Path,
    codex_bin: str | list[str] = "codex",
    codemap_bin: str | list[str] | None = None,
) -> Path:
    draft_path = draft_path.resolve()
    draft = json.loads(draft_path.read_text(encoding="utf-8"))
    if draft.get("kind") != "codemap_flagship_corpus" or draft.get("version") != 1:
        raise ValueError("draft must be codemap_flagship_corpus v1")
    validate_draft(draft)
    source = Path(draft["tasks_file"])
    source = (draft_path.parent / source).resolve() if not source.is_absolute() else source.resolve()
    tasks = read_jsonl(source)
    matrix = validate_tasks(tasks)
    repositories: dict[str, dict[str, str]] = {}
    frozen_tasks = []
    for task in tasks:
        meta = task_meta(task)
        repo = Path(task["repo"]).expanduser().resolve()
        commit = git_commit(repo, task.get("base_ref", "HEAD"))
        entry = {
            "path": str(repo),
            "commit": commit,
            "ecosystem": meta["ecosystem"],
        }
        key = f"{meta['repo_id']}:{meta.get('repo_variant', 'default')}"
        previous = repositories.setdefault(key, entry)
        if previous != entry:
            raise ValueError(f"repo {key}: tasks must share one frozen commit")
        frozen_tasks.append({**task, "repo": str(repo), "base_ref": commit})
    root = Path(__file__).resolve().parents[1]
    codex_command = resolve_command(codex_bin)
    codemap_command, resolution = resolve_codemap_command(codemap_bin, root)
    codemap_identity = benchmark_binary_identity(codemap_command, resolution, root)
    out_dir.mkdir(parents=True, exist_ok=False)
    tasks_path = out_dir / "tasks.jsonl"
    tasks_path.write_text(
        "\n".join(json.dumps(task, sort_keys=True) for task in frozen_tasks) + "\n",
        encoding="utf-8",
    )
    manifest = {
        key: draft[key]
        for key in ("kind", "version", "model", "reasoning_effort", "pair_order", "limits", "acceptance")
    }
    manifest.update(
        {
            "state": "frozen",
            "frozen_at": datetime.now(timezone.utc).isoformat(),
            "tasks_file": "tasks.jsonl",
            "tasks_sha256": file_sha256(tasks_path),
            "task_records": task_records(frozen_tasks),
            "matrix": matrix,
            "repositories": repositories,
            "verifier_artifacts": verifier_artifacts(frozen_tasks, tasks_path),
            "codex": {
                "command_argv": codex_command,
                "version": command_version(codex_command),
                "artifacts": command_artifacts(codex_command),
            },
            "codemap_identity": codemap_identity,
            "pair_schedule": pair_schedule(frozen_tasks),
        }
    )
    output = out_dir / "manifest.json"
    output.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return output


def load_frozen(path: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    path = path.resolve()
    manifest = json.loads(path.read_text(encoding="utf-8"))
    if manifest.get("state") != "frozen" or manifest.get("version") != 1:
        raise ValueError("flagship corpus is not frozen v1")
    validate_draft(manifest)
    tasks_path = path.parent / manifest.get("tasks_file", "")
    if not tasks_path.is_file() or file_sha256(tasks_path) != manifest.get("tasks_sha256"):
        raise ValueError("frozen task bytes changed")
    tasks = read_jsonl(tasks_path)
    if validate_tasks(tasks) != manifest.get("matrix"):
        raise ValueError("frozen task matrix changed")
    if task_records(tasks) != manifest.get("task_records"):
        raise ValueError("frozen prompts or criteria changed")
    if pair_schedule(tasks) != manifest.get("pair_schedule"):
        raise ValueError("frozen pair schedule changed")
    for artifact in manifest.get("verifier_artifacts", []):
        candidate = Path(artifact["path"])
        if not candidate.is_file() or file_sha256(candidate) != artifact.get("sha256"):
            raise ValueError(f"verifier artifact changed: {candidate}")
    return manifest, tasks
