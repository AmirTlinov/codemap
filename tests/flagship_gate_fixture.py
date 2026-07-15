#!/usr/bin/env python3
"""Synthetic black-box proof for the frozen S15 gate and independent verifier."""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from flagship_acceptance import evaluate  # noqa: E402
from flagship_judging import prepare_assignments, read_jsonl  # noqa: E402
from flagship_manifest import EXCLUSION_REASONS, freeze_corpus, load_frozen  # noqa: E402


def write(path: Path, body: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body, encoding="utf-8")


def git(repo: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo), *args], capture_output=True, text=True, check=True
    )
    return result.stdout.strip()


def fixture_repos(root: Path) -> list[tuple[Path, str]]:
    repos = []
    for index in range(6):
        repo = root / f"repo-{index}"
        repo.mkdir()
        git(repo, "init", "-q")
        git(repo, "config", "user.email", "fixture@example.com")
        git(repo, "config", "user.name", "fixture")
        write(repo / "README.md", f"repository {index}\n")
        git(repo, "add", ".")
        git(repo, "commit", "-qm", "fixture")
        repos.append((repo, git(repo, "rev-parse", "HEAD")))
    return repos


def criterion(name: str, category: str, verifier: Path, required: bool = False) -> dict:
    return {
        "name": name,
        "category": category,
        "weight": 1,
        "required": required,
        "scoring": "deterministic",
        "evidence_surface": f"external:{name}",
        "command": ["python3", str(verifier), "{last_message}"],
    }


def task_rows(root: Path, repos: list[tuple[Path, str]], verifier: Path) -> list[dict]:
    classes = [("analysis", 12), ("implementation", 12), ("negative_control", 6)]
    ecosystems = ["rust", "typescript", "python", "go", "rust", "typescript"]
    rows = []
    oracle = root / "hidden-oracle.txt"
    write(oracle, "immutable external evidence\n")
    ordinal = {
        "id": "analysis-depth",
        "category": "behavior",
        "weight": 2,
        "scoring": "ordinal",
        "max_score": 4,
        "judge_protocol": "two blind judges plus blind adjudicator",
        "evidence_surface": "candidate report with repository citations",
    }
    for task_class, count in classes:
        for index in range(count):
            repo_index = len(rows) % len(repos)
            repo = repos[repo_index][0]
            task_id = f"{task_class}-{index + 1:02}"
            split = "calibration" if index < count // 2 else "holdout"
            meta = {
                "task_class": task_class,
                "split": split,
                "repo_id": f"repo-{repo_index}",
                "ecosystem": ecosystems[repo_index],
                "ordinal_criteria": [ordinal] if task_class == "analysis" else [],
                "exception_criteria": ["analysis-depth" if task_class == "analysis" else "behavior"],
                "verifier_artifacts": [str(oracle)],
            }
            if task_class == "negative_control":
                meta.update(
                    {
                        "expected_same_outcome": True,
                        "allowed_exact_entries": ["cone README.md"],
                    }
                )
            rows.append(
                {
                    "id": task_id,
                    "mode": "analysis" if task_class == "analysis" else "implementation",
                    "repo": str(repo),
                    "base_ref": "HEAD",
                    "prompt": f"Complete frozen task {task_id} from README.md.",
                    "verify": [
                        criterion("required", "required", verifier, True),
                        criterion("behavior", "behavior", verifier),
                        criterion("contract", "contract", verifier),
                        criterion("downstream", "downstream", verifier),
                        criterion("regression", "regression", verifier),
                        criterion("provenance", "provenance", verifier),
                    ],
                    "benchmark": meta,
                }
            )
    return rows


def make_tools(root: Path) -> tuple[Path, Path, Path]:
    codex = root / "fake-codex.py"
    codemap = root / "fake-codemap.py"
    verifier = root / "verify.py"
    write(
        codex,
        "import sys\nprint('codex-cli 9.8.7') if '--version' in sys.argv else None\n",
    )
    write(
        codemap,
        """import json, sys
if '--version' in sys.argv:
    print('codemap 7.6.5')
elif 'doctor' in sys.argv:
    print(json.dumps({}))
""",
    )
    write(verifier, "raise SystemExit(0)\n")
    return codex, codemap, verifier


def freeze(root: Path, tasks: list[dict], codex: Path, codemap: Path) -> tuple[Path, dict]:
    tasks_path = root / "tasks.jsonl"
    write(tasks_path, "\n".join(json.dumps(task, sort_keys=True) for task in tasks) + "\n")
    draft = {
        "kind": "codemap_flagship_corpus",
        "version": 1,
        "tasks_file": str(tasks_path),
        "model": "gpt-5.6-sol",
        "reasoning_effort": "xhigh",
        "repetitions": 3,
        "timeout_seconds": 30,
        "verifier_timeout_seconds": 30,
        "bootstrap_iterations": 10_000,
        "bootstrap_seed": 1729,
        "pair_order": "split_task_index_plus_repetition_v1",
        "allowed_exclusions": sorted(EXCLUSION_REASONS),
        "acceptance": {
            "primary_alpha": 0.05,
            "min_task_win_rate": 0.60,
            "max_complex_time_overhead": 0.20,
            "max_complex_input_overhead": 0.15,
            "max_negative_overhead": 0.10,
            "min_agreement_alpha": 0.67,
            "allow_completeness_exception": True,
        },
        "judging": {
            "assignment_seed": 99,
            "manual_audit_seed": 101,
            "manual_audit_sample_size": 6,
            "judges_per_candidate": 2,
            "blind_adjudication": True,
        },
    }
    draft_path = root / "corpus-draft.json"
    write(draft_path, json.dumps(draft))
    manifest_path = freeze_corpus(
        draft_path,
        root / "frozen",
        f"{sys.executable} {codex}",
        f"{sys.executable} {codemap}",
    )
    return manifest_path, json.loads(manifest_path.read_text())


def verifier_results(task: dict, treatment: bool, negative: bool, trial_dir: Path) -> list[dict]:
    rows = []
    for verifier in task["verify"]:
        passed = not (verifier["name"] == "behavior" and not treatment and not negative)
        stdout = trial_dir / f"{verifier['name']}.stdout.log"
        stderr = trial_dir / f"{verifier['name']}.stderr.log"
        write(stdout, "external verifier\n")
        write(stderr, "")
        rows.append(
            {
                "name": verifier["name"],
                "category": verifier["category"],
                "weight": verifier["weight"],
                "required": verifier["required"],
                "passed": passed,
                "status": 0 if passed else 1,
                "timed_out": False,
                "stdout_artifact": str(stdout),
                "stderr_artifact": str(stderr),
            }
        )
    return rows


def result_row(
    task: dict,
    manifest: dict,
    repetition: int,
    arm: str,
    order: int,
    artifact: Path,
) -> dict:
    treatment = arm == "codemap"
    negative = task["benchmark"]["task_class"] == "negative_control"
    trial_dir = artifact.parent
    write(artifact, f"blind candidate {task['id']} {repetition} {arm}\n")
    events = trial_dir / "events.jsonl"
    stderr = trial_dir / "codex.stderr.log"
    write(events, "{}\n")
    write(stderr, "")
    repo_id = task["benchmark"]["repo_id"]
    protocol = {
        "invocation_count": 1 if treatment else 0,
        "compliant": True,
        "entry_kind": "exact" if treatment else "none",
        "root_entry": False,
        "first_entry": "cone README.md" if treatment else None,
    }
    row = {
        "task_id": task["id"],
        "mode": task["mode"],
        "task_prompt_sha256": hashlib.sha256(task["prompt"].encode()).hexdigest(),
        "repo": task["repo"],
        "base_commit": manifest["repositories"][f"{repo_id}:default"]["commit"],
        "repetition": repetition,
        "arm": arm,
        "order": order,
        "model": manifest["model"],
        "reasoning_effort": manifest["reasoning_effort"],
        "codex_version": manifest["codex_version"],
        "codex_artifacts": manifest["codex_artifacts"],
        "report_prelude": {"codemap": manifest["codemap_identity"]},
        "codemap_protocol": protocol,
        "codex": {
            "elapsed_ms": 105 if treatment else 100,
            "usage": {"input_tokens": 105 if treatment else 100},
            "events_artifact": str(events),
            "stderr_artifact": str(stderr),
            "last_message_artifact": str(artifact),
        },
        "verifiers": verifier_results(task, treatment, negative, trial_dir),
        "analysis_no_repo_changes": True,
        "outcome_passed": True,
        "run_valid": True,
    }
    write(trial_dir / "result.json", json.dumps(row, indent=2, sort_keys=True) + "\n")
    return row


def run_receipt(root: Path, split: str, tasks: list[dict], manifest: dict) -> Path:
    run_dir = root / split
    run_dir.mkdir()
    source = root / "frozen" / f"{split}.tasks.jsonl"
    (run_dir / "input-tasks.jsonl").write_bytes(source.read_bytes())
    selected = [task for task in tasks if task["benchmark"]["split"] == split]
    rows = []
    schedule = {
        (row["task_id"], row["repetition"]): row["arms"]
        for row in manifest["pair_schedule"][split]
    }
    for task in selected:
        for repetition in range(1, manifest["repetitions"] + 1):
            for arm in ("control", "codemap"):
                order = schedule[(task["id"], repetition)].index(arm) + 1
                artifact = run_dir / "trials" / f"{task['id']}-r{repetition}-{arm}" / "last-message.md"
                rows.append(result_row(task, manifest, repetition, arm, order, artifact))
    write(run_dir / "results.jsonl", "\n".join(json.dumps(row) for row in rows) + "\n")
    summary = {"preflight": [{"task_id": task["id"], "baseline_passed": False} for task in selected]}
    write(run_dir / "summary.json", json.dumps(summary))
    return run_dir


def ratings(assignments: Path, key_path: Path, output: Path) -> None:
    keys = {
        (row["assignment_id"], row["candidate_id"]): row["arm"] for row in read_jsonl(key_path)
    }
    rows = []
    for assignment in read_jsonl(assignments):
        arm = keys[(assignment["assignment_id"], assignment["candidate_id"])]
        score = 4 if arm == "codemap" else 2
        for judge in ("judge-1", "judge-2"):
            rows.append(
                {
                    "assignment_id": assignment["assignment_id"],
                    "candidate_id": assignment["candidate_id"],
                    "judge_id": judge,
                    "role": "judge",
                    "scores": {criterion: score for criterion in assignment["criteria"]},
                }
            )
        if assignment["manual_audit_required"]:
            rows.append(
                {
                    "assignment_id": assignment["assignment_id"],
                    "candidate_id": assignment["candidate_id"],
                    "judge_id": "blind-auditor",
                    "role": "auditor",
                    "audit_passed": True,
                }
            )
    write(output, "\n".join(json.dumps(row) for row in rows) + "\n")


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="codemap-flagship-test-") as temporary:
        root = Path(temporary)
        repos = fixture_repos(root)
        codex, codemap, verifier = make_tools(root)
        tasks = task_rows(root, repos, verifier)
        manifest_path, manifest = freeze(root, tasks, codex, codemap)
        calibration = run_receipt(root, "calibration", tasks, manifest)
        holdout = run_receipt(root, "holdout", tasks, manifest)
        assignments, key = prepare_assignments(
            manifest_path, tasks, [calibration, holdout], root / "judging"
        )
        ratings_path = root / "ratings.jsonl"
        ratings(assignments, key, ratings_path)
        receipt = evaluate(
            manifest_path,
            calibration,
            holdout,
            assignments,
            key,
            ratings_path,
            root / "acceptance",
        )
        report = json.loads(receipt.read_text())
        assert report["acceptance"]["accepted"] is True
        verify = ROOT / "scripts/verify-flagship-acceptance.py"
        valid = subprocess.run([sys.executable, str(verify), str(receipt)], capture_output=True)
        assert valid.returncode == 0, valid.stderr.decode()
        result_path = holdout / "results.jsonl"
        original_rows = result_path.read_text().splitlines()
        write(result_path, "\n".join(original_rows[:-1]) + "\n")
        rejected = evaluate(
            manifest_path,
            calibration,
            holdout,
            assignments,
            key,
            ratings_path,
            root / "rejected-acceptance",
        )
        rejected_report = json.loads(rejected.read_text())
        assert rejected_report["acceptance"]["accepted"] is False
        assert rejected_report["acceptance"]["checks"]["complete_valid_denominator"] is False
        tampered = subprocess.run([sys.executable, str(verify), str(receipt)], capture_output=True)
        assert tampered.returncode == 1
        write(root / "hidden-oracle.txt", "tampered\n")
        try:
            load_frozen(manifest_path)
        except ValueError as error:
            assert "verifier artifact changed" in str(error)
        else:
            raise AssertionError("transitive verifier artifact tampering was accepted")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
