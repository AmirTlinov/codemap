"""Blind analysis-candidate packaging and ordinal judgment loading."""

from __future__ import annotations

import hashlib
import json
import re
from collections import defaultdict
from pathlib import Path
from typing import Any

from flagship_manifest import file_sha256
from flagship_stats import ordinal_alpha


MARKDOWN_CITATION = re.compile(r"\[([^\]\n]+):(\d+)\]\([^\n)]+\)")


def blind_candidate_text(source: Path) -> str:
    """Keep visible evidence while removing worktree URLs that disclose the arm."""
    body = source.read_text(encoding="utf-8")
    return MARKDOWN_CITATION.sub(lambda match: f"{match.group(1)}:{match.group(2)}", body)


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.is_file():
        raise ValueError(f"missing JSONL artifact: {path}")
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


def _candidate_order(seed: int, task_id: str, repetition: int) -> list[str]:
    digest = hashlib.sha256(f"{seed}:{task_id}:{repetition}".encode()).digest()
    return ["control", "codemap"] if digest[0] % 2 else ["codemap", "control"]


def prepare_assignments(
    manifest_path: Path,
    tasks: list[dict[str, Any]],
    result_dirs: list[Path],
    out_dir: Path,
) -> tuple[Path, Path]:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    judging = manifest["judging"]
    seed = judging["assignment_seed"]
    results: dict[tuple[str, int, str], dict[str, Any]] = {}
    for result_dir in result_dirs:
        for row in read_jsonl(result_dir / "results.jsonl"):
            results[(row["task_id"], row["repetition"], row["arm"])] = row
    analysis = [task for task in tasks if task["benchmark"]["task_class"] == "analysis"]
    out_dir.mkdir(parents=True, exist_ok=False)
    candidates_dir = out_dir / "candidates"
    candidates_dir.mkdir()
    public: list[dict[str, Any]] = []
    secret: list[dict[str, Any]] = []
    for task in analysis:
        task_id = task["id"]
        criteria = [criterion["id"] for criterion in task["benchmark"]["ordinal_criteria"]]
        for repetition in range(1, 1 + max(row["repetition"] for row in results.values())):
            assignment_id = f"{task_id}-r{repetition}"
            for index, arm in enumerate(_candidate_order(seed, task_id, repetition)):
                candidate_id = chr(ord("A") + index)
                row = results.get((task_id, repetition, arm))
                if row is None:
                    raise ValueError(f"missing analysis result: {task_id} r{repetition} {arm}")
                source = Path(row["codex"]["last_message_artifact"])
                if not source.is_file():
                    raise ValueError(f"missing candidate artifact: {source}")
                target = candidates_dir / f"{assignment_id}-{candidate_id}.md"
                target.write_text(blind_candidate_text(source), encoding="utf-8")
                public.append(
                    {
                        "assignment_id": assignment_id,
                        "task_id": task_id,
                        "split": task["benchmark"]["split"],
                        "repetition": repetition,
                        "candidate_id": candidate_id,
                        "artifact": str(target),
                        "artifact_sha256": file_sha256(target),
                        "criteria": criteria,
                    }
                )
                secret.append(
                    {
                        "assignment_id": assignment_id,
                        "candidate_id": candidate_id,
                        "arm": arm,
                        "result_key": f"{task_id}|{repetition}|{arm}",
                        "source_artifact_sha256": file_sha256(source),
                    }
                )
    audit_ids: set[str] = set()
    sample_size = judging["manual_audit_sample_size"]
    for split, count in (("calibration", sample_size // 2), ("holdout", sample_size - sample_size // 2)):
        assignment_ids = sorted(
            {row["assignment_id"] for row in public if row["split"] == split},
            key=lambda value: hashlib.sha256(
                f"{judging['manual_audit_seed']}:{split}:{value}".encode()
            ).hexdigest(),
        )
        audit_ids.update(assignment_ids[:count])
    for row in public:
        row["manual_audit_required"] = row["assignment_id"] in audit_ids
    public_path = out_dir / "assignments.jsonl"
    key_path = out_dir / "assignment-key.jsonl"
    public_path.write_text("\n".join(json.dumps(row, sort_keys=True) for row in public) + "\n")
    key_path.write_text("\n".join(json.dumps(row, sort_keys=True) for row in secret) + "\n")
    receipt = {
        "kind": "codemap_blind_assignments",
        "manifest_sha256": file_sha256(manifest_path),
        "assignments_sha256": file_sha256(public_path),
        "key_sha256": file_sha256(key_path),
        "seed": seed,
        "manual_audit_seed": judging["manual_audit_seed"],
        "manual_audit_assignments": sorted(audit_ids),
    }
    (out_dir / "assignment-receipt.json").write_text(
        json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    )
    return public_path, key_path


def load_judgments(
    tasks: list[dict[str, Any]],
    assignments_path: Path,
    key_path: Path,
    ratings_path: Path,
    min_alpha: float,
) -> tuple[dict[tuple[str, int, str, str], float], dict[str, Any]]:
    assignments = read_jsonl(assignments_path)
    keys = read_jsonl(key_path)
    ratings = read_jsonl(ratings_path)
    mapping = {(row["assignment_id"], row["candidate_id"]): row["arm"] for row in keys}
    assigned = {(row["assignment_id"], row["candidate_id"]): row for row in assignments}
    if set(mapping) != set(assigned) or len(mapping) != len(keys) or len(assigned) != len(assignments):
        raise ValueError("blind assignment/key identities are incomplete or duplicated")
    for blind_key, assignment in assigned.items():
        artifact = Path(assignment["artifact"])
        if not artifact.is_file() or file_sha256(artifact) != assignment.get("artifact_sha256"):
            raise ValueError(f"blind candidate artifact changed: {blind_key}")
        if mapping[blind_key] not in {"control", "codemap"}:
            raise ValueError(f"invalid sealed arm mapping: {blind_key}")
    assignment_ids = {row["assignment_id"] for row in assignments}
    for assignment_id in assignment_ids:
        candidates = {key[1] for key in assigned if key[0] == assignment_id}
        arms = {arm for key, arm in mapping.items() if key[0] == assignment_id}
        if candidates != {"A", "B"} or arms != {"control", "codemap"}:
            raise ValueError(f"blind pair is not counterbalanced: {assignment_id}")
    task_map = {task["id"]: task for task in tasks}
    grouped: dict[tuple[str, str], list[dict[str, Any]]] = defaultdict(list)
    for rating in ratings:
        key = (rating.get("assignment_id"), rating.get("candidate_id"))
        if key not in assigned:
            raise ValueError(f"rating references unknown blind candidate: {key}")
        if rating.get("role") not in {"judge", "adjudicator", "auditor"}:
            raise ValueError(f"invalid rating role for {key}")
        if rating.get("role") != "auditor" and not isinstance(rating.get("scores"), dict):
            raise ValueError(f"scores are required for {key}")
        grouped[key].append(rating)
    values: dict[tuple[str, int, str, str], float] = {}
    alpha_units: dict[str, list[list[int]]] = defaultdict(list)
    max_scores: dict[str, int] = {}
    for key, assignment in assigned.items():
        task = task_map[assignment["task_id"]]
        criteria = {row["id"]: row for row in task["benchmark"]["ordinal_criteria"]}
        rows = grouped.get(key, [])
        judges = [row for row in rows if row["role"] == "judge"]
        adjudicators = [row for row in rows if row["role"] == "adjudicator"]
        auditors = [row for row in rows if row["role"] == "auditor"]
        if len(judges) != 2 or len({row.get("judge_id") for row in judges}) != 2:
            raise ValueError(f"{key}: exactly two independent blind judges required")
        if assignment.get("manual_audit_required") and (
            len(auditors) != 1 or auditors[0].get("audit_passed") is not True
        ):
            raise ValueError(f"{key}: required blind manual audit is missing or failed")
        for criterion_id, criterion in criteria.items():
            scores = [row["scores"].get(criterion_id) for row in judges]
            maximum = criterion["max_score"]
            if not all(isinstance(score, int) and 0 <= score <= maximum for score in scores):
                raise ValueError(f"{key}: invalid score for {criterion_id}")
            alpha_units[criterion_id].append(scores)
            max_scores[criterion_id] = maximum
            if scores[0] == scores[1]:
                final = scores[0]
            else:
                matching = [row for row in adjudicators if criterion_id in row["scores"]]
                if len(matching) != 1:
                    raise ValueError(f"{key}: disagreement requires one blind adjudication")
                final = matching[0]["scores"][criterion_id]
                if not isinstance(final, int) or not 0 <= final <= maximum:
                    raise ValueError(f"{key}: invalid adjudicated score for {criterion_id}")
            arm = mapping[key]
            values[(assignment["task_id"], assignment["repetition"], arm, criterion_id)] = (
                final / maximum
            )
    agreement = {}
    for criterion_id, units in alpha_units.items():
        alpha = ordinal_alpha(units, max_scores[criterion_id])
        valid = alpha is not None and alpha >= min_alpha
        agreement[criterion_id] = {"alpha": alpha, "units": len(units), "valid": valid}
    audited = sum(row.get("manual_audit_required") is True for row in assignments)
    agreement["manual_audit"] = {"alpha": None, "units": audited, "valid": audited > 0}
    return values, agreement
