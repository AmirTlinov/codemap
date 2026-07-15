"""Run frozen arm-blind ordinal judges and merge a separately reviewed audit sample."""

from __future__ import annotations

import hashlib
import json
import os
import signal
import subprocess
import time
from collections import defaultdict
from pathlib import Path
from typing import Any

from benchmark_parallel import run_ordered
from codemap_identity import command_artifacts
from flagship_judging import read_jsonl
from flagship_manifest import command_version, file_sha256, load_frozen


JUDGE_IDS = ("blind-judge-1", "blind-judge-2")


def _json_message(path: Path) -> dict[str, Any]:
    body = path.read_text(encoding="utf-8").strip()
    if body.startswith("```"):
        body = body.split("\n", 1)[1].rsplit("```", 1)[0].strip()
    start, end = body.find("{"), body.rfind("}")
    if start < 0 or end < start:
        raise ValueError(f"judge did not return a JSON object: {path}")
    value = json.loads(body[start : end + 1])
    if not isinstance(value, dict):
        raise ValueError(f"judge output must be an object: {path}")
    return value


def _run_process(argv: list[str], cwd: Path, timeout: int) -> tuple[int, int, str, str]:
    started = time.monotonic_ns()
    process = subprocess.Popen(
        argv,
        cwd=cwd,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace",
        start_new_session=True,
    )
    try:
        stdout, stderr = process.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        try:
            os.killpg(process.pid, signal.SIGTERM)
            stdout, stderr = process.communicate(timeout=5)
        except (ProcessLookupError, subprocess.TimeoutExpired):
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            stdout, stderr = process.communicate()
        return 124, int((time.monotonic_ns() - started) // 1_000_000), stdout, stderr
    return process.returncode, int((time.monotonic_ns() - started) // 1_000_000), stdout, stderr


def _request(task: dict[str, Any], assignments: list[dict[str, Any]], criteria: set[str] | None) -> dict:
    rubric = []
    for row in task["benchmark"]["ordinal_criteria"]:
        if criteria is None or row["id"] in criteria:
            rubric.append(
                {
                    "id": row["id"],
                    "max_score": row["max_score"],
                    "protocol": row["judge_protocol"],
                    "evidence_surface": row["evidence_surface"],
                }
            )
    candidates = []
    for row in assignments:
        selected = [item["id"] for item in rubric if item["id"] in row["criteria"]]
        if selected:
            candidates.append(
                {
                    "assignment_id": row["assignment_id"],
                    "candidate_id": row["candidate_id"],
                    "artifact": row["artifact"],
                    "artifact_sha256": row["artifact_sha256"],
                    "criteria": selected,
                }
            )
    return {"task_id": task["id"], "rubric": rubric, "candidates": candidates}


def _prompt(request: dict, adjudication: bool) -> str:
    role = "blind adjudicator" if adjudication else "independent blind judge"
    return f"""You are an {role} for a frozen repository-analysis benchmark.
You do not know which candidate used which navigation condition. Do not infer or discuss arms,
resource usage, or tools. Read every candidate artifact and verify its material claims against the
read-only frozen repository. Use read-only file/VCS inspection only: no builds, tests, hooks,
installs, network, or repository edits. Apply each supplied ordinal protocol strictly.

Return one JSON object and no prose: {{"ratings":[{{"assignment_id":"...",
"candidate_id":"A","scores":{{"criterion":0}},"reasons":{{"criterion":"brief evidence"}}}}]}}.
Include every requested candidate and criterion exactly once. Scores must be integers within the
declared range. Candidate letters and ordering are counterbalanced and carry no quality meaning.

REQUEST_JSON:
{json.dumps(request, indent=2, sort_keys=True)}
"""


def _validate_output(
    payload: dict[str, Any], request: dict, judge_id: str, role: str
) -> list[dict[str, Any]]:
    expected = {
        (row["assignment_id"], row["candidate_id"]): set(row["criteria"])
        for row in request["candidates"]
    }
    maxima = {row["id"]: row["max_score"] for row in request["rubric"]}
    ratings = payload.get("ratings")
    if not isinstance(ratings, list):
        raise ValueError(f"{judge_id}: ratings must be an array")
    observed: dict[tuple[str, str], dict[str, Any]] = {}
    for row in ratings:
        key = (row.get("assignment_id"), row.get("candidate_id"))
        if key not in expected or key in observed:
            raise ValueError(f"{judge_id}: unexpected or duplicate candidate {key}")
        scores = row.get("scores")
        if not isinstance(scores, dict) or set(scores) != expected[key]:
            raise ValueError(f"{judge_id}: incomplete criteria for {key}")
        for criterion, score in scores.items():
            if not isinstance(score, int) or isinstance(score, bool) or not 0 <= score <= maxima[criterion]:
                raise ValueError(f"{judge_id}: invalid {criterion} score for {key}")
        observed[key] = {
            "assignment_id": key[0],
            "candidate_id": key[1],
            "judge_id": judge_id,
            "role": role,
            "scores": scores,
            "reasons": row.get("reasons", {}),
        }
    if set(observed) != set(expected):
        raise ValueError(f"{judge_id}: missing candidates {sorted(set(expected) - set(observed))}")
    return [observed[key] for key in sorted(observed)]


def _judge_job(
    job: tuple[str, dict[str, Any], list[dict[str, Any]], set[str] | None, bool],
    manifest: dict[str, Any],
    out_dir: Path,
) -> list[dict[str, Any]]:
    judge_id, task, assignments, criteria, adjudication = job
    request = _request(task, assignments, criteria)
    job_dir = out_dir / "receipts" / f"{judge_id}-{task['id']}"
    job_dir.mkdir(parents=True, exist_ok=False)
    prompt = _prompt(request, adjudication)
    (job_dir / "prompt.txt").write_text(prompt, encoding="utf-8")
    message = job_dir / "last-message.json"
    events = job_dir / "events.jsonl"
    stderr_path = job_dir / "stderr.log"
    judging = manifest["judging"]
    argv = [
        *manifest["codex_command"],
        "exec", "--json", "--ephemeral", "--ignore-user-config", "--ignore-rules",
        "--strict-config", "--color", "never", "--disable", "multi_agent",
        "--disable", "enable_fanout", "-m", judging["model"], "-c",
        f'model_reasoning_effort="{judging["reasoning_effort"]}"', "-c",
        'approval_policy="never"', "-s", "read-only", "-C", task["repo"],
        "--add-dir", str(Path(assignments[0]["artifact"]).parent), "-o", str(message), prompt,
    ]
    status, elapsed, stdout, stderr = _run_process(argv, Path(task["repo"]), judging["timeout_seconds"])
    events.write_text(stdout, encoding="utf-8")
    stderr_path.write_text(stderr, encoding="utf-8")
    if status != 0 or not message.is_file():
        raise ValueError(f"{judge_id}/{task['id']}: Codex judge failed with status {status}")
    ratings = _validate_output(
        _json_message(message), request, judge_id, "adjudicator" if adjudication else "judge"
    )
    receipt = {
        "judge_id": judge_id,
        "task_id": task["id"],
        "role": "adjudicator" if adjudication else "judge",
        "status": status,
        "elapsed_ms": elapsed,
        "prompt_sha256": hashlib.sha256(prompt.encode()).hexdigest(),
        "message_sha256": file_sha256(message),
        "events_sha256": file_sha256(events),
    }
    (job_dir / "receipt.json").write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n")
    return ratings


def run_judging(manifest_path: Path, assignments_path: Path, out_dir: Path) -> Path:
    manifest, tasks = load_frozen(manifest_path)
    if command_version(manifest["codex_command"]) != manifest["codex_version"]:
        raise ValueError("judge Codex version differs from frozen manifest")
    if command_artifacts(manifest["codex_command"]) != manifest["codex_artifacts"]:
        raise ValueError("judge Codex executable bytes differ from frozen manifest")
    assignments = read_jsonl(assignments_path)
    task_map = {task["id"]: task for task in tasks}
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in assignments:
        artifact = Path(row["artifact"])
        if not artifact.is_file() or file_sha256(artifact) != row["artifact_sha256"]:
            raise ValueError(f"blind candidate changed: {artifact}")
        grouped[row["task_id"]].append(row)
    out_dir.mkdir(parents=True, exist_ok=False)
    jobs = [
        (judge_id, task_map[task_id], rows, None, False)
        for judge_id in JUDGE_IDS
        for task_id, rows in sorted(grouped.items())
    ]
    ratings = [
        row
        for batch in run_ordered(
            jobs,
            lambda job: _judge_job(job, manifest, out_dir),
            manifest["judging"]["parallel_jobs"],
        )
        for row in batch
    ]
    by_candidate: dict[tuple[str, str], list[dict[str, Any]]] = defaultdict(list)
    for row in ratings:
        by_candidate[(row["assignment_id"], row["candidate_id"])].append(row)
    disputes: dict[str, set[str]] = defaultdict(set)
    for key, rows in by_candidate.items():
        for criterion in rows[0]["scores"]:
            if rows[0]["scores"][criterion] != rows[1]["scores"][criterion]:
                task_id = next(row["task_id"] for row in assignments if (row["assignment_id"], row["candidate_id"]) == key)
                disputes[task_id].add(criterion)
    adjudication_jobs = [
        ("blind-adjudicator", task_map[task_id], grouped[task_id], criteria, True)
        for task_id, criteria in sorted(disputes.items())
    ]
    for batch in run_ordered(
        adjudication_jobs,
        lambda job: _judge_job(job, manifest, out_dir),
        manifest["judging"]["parallel_jobs"],
    ):
        ratings.extend(batch)
    ratings_path = out_dir / "ratings.jsonl"
    ratings_path.write_text("\n".join(json.dumps(row, sort_keys=True) for row in ratings) + "\n")
    audit = [row for row in assignments if row.get("manual_audit_required") is True]
    audit_path = out_dir / "manual-audit-packet.jsonl"
    audit_path.write_text("\n".join(json.dumps(row, sort_keys=True) for row in audit) + "\n")
    receipt = {
        "kind": "codemap_blind_judging",
        "manifest_sha256": file_sha256(manifest_path),
        "assignments_sha256": file_sha256(assignments_path),
        "ratings_sha256": file_sha256(ratings_path),
        "manual_audit_packet_sha256": file_sha256(audit_path),
        "judge_ids": list(JUDGE_IDS),
        "adjudicated_tasks": sorted(disputes),
    }
    (out_dir / "judging-receipt.json").write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n")
    return ratings_path


def merge_audits(assignments_path: Path, ratings_path: Path, decisions_path: Path, output: Path) -> Path:
    assignments = read_jsonl(assignments_path)
    required = {
        (row["assignment_id"], row["candidate_id"])
        for row in assignments
        if row.get("manual_audit_required") is True
    }
    decisions = read_jsonl(decisions_path)
    observed = {(row.get("assignment_id"), row.get("candidate_id")): row for row in decisions}
    if set(observed) != required or len(observed) != len(decisions):
        raise ValueError("manual audit decisions must cover the frozen sample exactly once")
    if not all(isinstance(row.get("audit_passed"), bool) for row in decisions):
        raise ValueError("every manual audit decision requires audit_passed boolean")
    ratings = read_jsonl(ratings_path)
    ratings.extend(
        {
            "assignment_id": key[0],
            "candidate_id": key[1],
            "judge_id": "manual-auditor",
            "role": "auditor",
            "audit_passed": row["audit_passed"],
            "notes": row.get("notes", ""),
        }
        for key, row in sorted(observed.items())
    )
    output.write_text("\n".join(json.dumps(row, sort_keys=True) for row in ratings) + "\n")
    return output
