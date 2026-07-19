"""Synthetic corpus and run builder for the flagship acceptance fixture."""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from flagship_acceptance import evaluate  # noqa: E402
from flagship_manifest import freeze_corpus, load_frozen  # noqa: E402
from flagship_trajectory_support import build_trajectory  # noqa: E402


def write(path: Path, body: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body, encoding="utf-8")


def git(repo: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo), *args], capture_output=True, text=True, check=True
    )
    return result.stdout.strip()


def fixture_repos(root: Path) -> list[Path]:
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
        repos.append(repo)
    return repos


def verifier(name: str, path: Path, required: bool) -> dict:
    return {
        "name": name,
        "category": "required" if required else "completeness",
        "weight": 1,
        "required": required,
        "scoring": "deterministic",
        "evidence_surface": f"external:{name}",
        "command": [sys.executable, str(path), "{last_message}"],
    }


def task_rows(repos: list[Path], verifier_path: Path) -> list[dict]:
    ecosystems = ["rust", "typescript", "python", "go", "rust", "typescript"]
    rows = []
    for index, repo in enumerate(repos):
        for task_class in ("investigation", "implementation", "exact_control"):
            task_id = f"repo-{index}-{task_class}"
            meta = {
                "task_class": task_class,
                "repo_id": f"repo-{index}",
                "ecosystem": ecosystems[index],
                "verifier_artifacts": [str(verifier_path)],
            }
            rows.append(
                {
                    "id": task_id,
                    "mode": "analysis" if task_class == "investigation" else "implementation",
                    "repo": str(repo),
                    "base_ref": "HEAD",
                    "prompt": f"Complete deterministic {task_class} task for repo {index}.",
                    "verify": [
                        verifier("required", verifier_path, True),
                        verifier("completeness", verifier_path, False),
                    ],
                    "benchmark": meta,
                }
            )
    return rows


def make_tools(root: Path) -> tuple[Path, Path, Path]:
    codex = root / "fake-codex.py"
    codemap = root / "fake-codemap.py"
    verifier_path = root / "verify.py"
    write(codex, "import sys\nprint('codex-cli 9.8.7') if '--version' in sys.argv else None\n")
    write(
        codemap,
        """import json, sys
if '--version' in sys.argv:
    print('codemap 7.6.5')
elif 'doctor' in sys.argv:
    print(json.dumps({}))
""",
    )
    write(verifier_path, "raise SystemExit(0)\n")
    return codex, codemap, verifier_path


def freeze(root: Path, tasks: list[dict], codex: Path, codemap: Path) -> tuple[Path, dict]:
    tasks_path = root / "tasks.jsonl"
    write(tasks_path, "\n".join(json.dumps(task, sort_keys=True) for task in tasks) + "\n")
    draft = {
        "kind": "codemap_flagship_corpus",
        "version": 1,
        "tasks_file": str(tasks_path),
        "model": "gpt-5.6-sol",
        "reasoning_effort": "high",
        "pair_order": "task_index_plus_repetition_v1",
        "limits": {
            "repetitions": 2,
            "parallel_pairs": 2,
            "timeout_seconds": 30,
            "verifier_timeout_seconds": 30,
            "infrastructure_retries": 1,
        },
        "acceptance": {
            "min_complex_wins": 8,
            "max_complex_time_overhead": 0.20,
            "max_complex_input_overhead": 0.15,
            "max_exact_overhead": 0.10,
        },
    }
    draft_path = root / "corpus-draft.json"
    write(draft_path, json.dumps(draft))
    manifest_path = freeze_corpus(
        draft_path,
        root / "frozen",
        [sys.executable, str(codex)],
        [sys.executable, str(codemap)],
    )
    return manifest_path, json.loads(manifest_path.read_text(encoding="utf-8"))


def verifier_results(
    task: dict, arm: str, win: bool, required_loss: bool, trial_dir: Path
) -> list[dict]:
    rows = []
    for item in task["verify"]:
        passed = True
        if item["name"] == "completeness" and task["benchmark"]["task_class"] != "exact_control":
            passed = arm == "codemap" if win else True
        if item["name"] == "required" and arm == "codemap" and required_loss:
            passed = False
        stdout = trial_dir / f"{item['name']}.stdout.log"
        stderr = trial_dir / f"{item['name']}.stderr.log"
        write(stdout, "external verifier\n")
        write(stderr, "")
        rows.append(
            {
                "name": item["name"],
                "category": item["category"],
                "weight": item["weight"],
                "required": item["required"],
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
    win: bool,
    required_loss: bool = False,
    exact_regression: bool = False,
    complex_over: bool = False,
    exact_over: bool = False,
    infrastructure_failure: bool = False,
    repo_write: bool = False,
) -> dict:
    treatment = arm == "codemap"
    task_class = task["benchmark"]["task_class"]
    required_loss = treatment and (required_loss or exact_regression)
    trial_dir = artifact.parent
    write(artifact, f"candidate {task['id']} {repetition} {arm}\n")
    events = trial_dir / "events.jsonl"
    stderr = trial_dir / "codex.stderr.log"
    patch = trial_dir / "patch.diff"
    write(events, "{}\n")
    write(stderr, "")
    write(patch, "")
    exact_without_map = treatment and task_class == "exact_control"
    calls = []
    if treatment and not exact_without_map:
        calls.append(
            {
                "invocation_index": 0,
                "command": "cone",
                "argument": "README.md",
                "scope_kind": "scoped",
                "status": 0,
                "succeeded": True,
                "argv": ["cone", "README.md"],
                "display": "cone README.md",
            }
        )
    activity = {
        "invocation_count": 1 if treatment and not exact_without_map else 0,
        "calls": calls,
    }
    elapsed = 1000
    input_tokens = 1000
    if treatment:
        if task_class == "exact_control":
            elapsed = input_tokens = 1110 if exact_over else 1100
        else:
            elapsed = 1210 if complex_over else 1200
            input_tokens = 1160 if complex_over else 1150
    verifiers = verifier_results(task, arm, win, required_loss, trial_dir)
    row = {
        "task_id": task["id"],
        "mode": task["mode"],
        "task_prompt_sha256": hashlib.sha256(task["prompt"].strip().encode()).hexdigest(),
        "repo": task["repo"],
        "base_commit": manifest["repositories"][
            f"{task['benchmark']['repo_id']}:default"
        ]["commit"],
        "repetition": repetition,
        "arm": arm,
        "order": order,
        "model": manifest["model"],
        "reasoning_effort": manifest["reasoning_effort"],
        "codex_version": manifest["codex"]["version"],
        "codex_artifacts": manifest["codex"]["artifacts"],
        "report_prelude": {"codemap": manifest["codemap_identity"]},
        "trial_fingerprint": f"{task['id']}:{repetition}:{arm}",
        "codemap_activity": activity,
        "patch_artifact": str(patch),
        "codex": {
            "elapsed_ms": elapsed,
            "usage": {"input_tokens": input_tokens},
            "events_artifact": str(events),
            "stderr_artifact": str(stderr),
            "last_message_artifact": str(artifact),
        },
        "verifiers": verifiers,
        "analysis_no_repo_changes": not repo_write,
        "outcome_passed": all(row["passed"] for row in verifiers if row["required"]),
        "run_valid": not infrastructure_failure,
        "invalidation_reason": "codex_timeout" if infrastructure_failure else None,
        "infrastructure_attempt": 2 if infrastructure_failure else 1,
        "prior_attempts": ["attempts/attempt-1/result.json"] if infrastructure_failure else [],
    }
    if infrastructure_failure:
        first = {
            "trial_fingerprint": row["trial_fingerprint"],
            "infrastructure_attempt": 1,
            "prior_attempts": [],
            "invalidation_reason": "codex_timeout",
            "run_valid": False,
        }
        path = trial_dir / "attempts/attempt-1/result.json"
        write(path, json.dumps(first, indent=2, sort_keys=True) + "\n")
    write(trial_dir / "result.json", json.dumps(row, indent=2, sort_keys=True) + "\n")
    return row

def make_run(
    root: Path,
    name: str,
    tasks: list[dict],
    manifest: dict,
    *,
    wins: int = 8,
    missing_arm: bool = False,
    loss: bool = False,
    required_loss: bool = False,
    exact_regression: bool = False,
    complex_over: bool = False,
    exact_over: bool = False,
    infrastructure_failure: bool = False,
    repo_write: bool = False,
) -> Path:
    run_dir = root / name
    run_dir.mkdir()
    (run_dir / "input-tasks.jsonl").write_bytes((root / "frozen/tasks.jsonl").read_bytes())
    schedule = {
        (row["task_id"], row["repetition"]): row["arms"]
        for row in manifest["pair_schedule"]
    }
    complex_ids = [
        task["id"] for task in tasks if task["benchmark"]["task_class"] != "exact_control"
    ]
    rows = []
    for task in tasks:
        task_win = task["id"] in complex_ids[:wins]
        for repetition in (1, 2):
            for arm in ("control", "codemap"):
                artifact = run_dir / "trials" / f"{task['id']}-r{repetition}-{arm}" / "last-message.md"
                rows.append(
                    result_row(
                        task,
                        manifest,
                        repetition,
                        arm,
                        schedule[(task["id"], repetition)].index(arm) + 1,
                        artifact,
                        task_win,
                        required_loss=required_loss and task["id"] == complex_ids[0],
                        exact_regression=exact_regression
                        and task["benchmark"]["task_class"] == "exact_control"
                        and task["benchmark"]["repo_id"] == "repo-0",
                        complex_over=complex_over,
                        exact_over=exact_over,
                        infrastructure_failure=infrastructure_failure
                        and task["id"] == complex_ids[0]
                        and repetition == 1
                        and arm == "codemap",
                        repo_write=repo_write
                        and task["mode"] == "analysis"
                        and task["id"] == complex_ids[0],
                    )
                )
    if loss:
        target = complex_ids[-1]
        for row in rows:
            if row["task_id"] == target and row["arm"] == "codemap":
                bonus = next(item for item in row["verifiers"] if item["name"] == "completeness")
                bonus.update({"passed": False, "status": 1})
                write(Path(bonus["stdout_artifact"]), "external verifier\n")
                write(Path(row["codex"]["last_message_artifact"]).parent / "result.json", json.dumps(row, indent=2, sort_keys=True) + "\n")
    if missing_arm:
        rows.pop()
    write(run_dir / "results.jsonl", "\n".join(json.dumps(row) for row in rows) + "\n")
    preflight = [{"task_id": task["id"], "baseline_passed": False} for task in tasks]
    write(run_dir / "summary.json", json.dumps({"preflight": preflight}))
    return run_dir


def report(
    root: Path,
    name: str,
    manifest: Path,
    run_dir: Path,
    trajectory_state: str = "complete",
) -> dict:
    tasks = load_frozen(manifest)[1]
    trajectory = None
    if trajectory_state != "missing":
        trajectory = build_trajectory(root / f"trajectory-{name}", tasks, manifest)
        if trajectory_state == "incomplete":
            summary = json.loads(trajectory.read_text(encoding="utf-8"))
            summary["pairs"].pop()
            summary["complete"] = False
            write(trajectory, json.dumps(summary, indent=2, sort_keys=True) + "\n")
        elif trajectory_state == "corrupt":
            write(trajectory, "{\n")
    path = evaluate(manifest, run_dir, root / f"acceptance-{name}", trajectory)
    return json.loads(path.read_text(encoding="utf-8"))
