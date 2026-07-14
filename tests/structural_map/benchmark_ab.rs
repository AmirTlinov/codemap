#[test]
fn behavioral_ab_pairs_identical_tasks_and_scores_external_verifiers() {
    let repo = TempDir::new().expect("A/B fixture repo");
    let support = TempDir::new().expect("A/B support dir");
    let out = TempDir::new().expect("A/B output dir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(&repo.path().join("README.md"), "A/B fixture\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let fake_codemap = support.path().join("fake-codemap.py");
    write(
        &fake_codemap,
        r#"import sys
sys.exit(0)
"#,
    );
    let fake_codex = support.path().join("fake-codex.py");
    write(
        &fake_codex,
        r#"import json
import pathlib
import subprocess
import sys

args = sys.argv[1:]
if "--version" in args:
    print("codex-cli fake-ab")
    raise SystemExit(0)
worktree = pathlib.Path(args[args.index("-C") + 1])
last_message = pathlib.Path(args[args.index("-o") + 1])
prompt = args[-1]
treatment = "CODEMAP TREATMENT ARM" in prompt
analysis = "repository-analysis task" in prompt
if treatment and analysis:
    for command in (["codemap", "ls", "."], ["codemap", "cone", "README.md"]):
        subprocess.run(command, cwd=worktree, check=True)
elif treatment:
    for command in (["codemap", "ls", "."], ["codemap", "changed"], ["codemap", "proof", "changed"]):
        subprocess.run(command, cwd=worktree, check=True)
if analysis:
    last_message.write_text("Confirmed finding with evidence at README.md:1\n")
else:
    (worktree / "answer.txt").write_text("42\n" if treatment else "0\n")
    last_message.write_text("completed\n")
print(json.dumps({"type": "thread.started", "thread_id": "fake-thread"}))
print(json.dumps({"type": "item.completed", "item": {"type": "agent_message", "text": "completed"}}))
print(json.dumps({"type": "turn.completed", "usage": {"input_tokens": 100, "cached_input_tokens": 20, "output_tokens": 10, "reasoning_output_tokens": 5}}))
"#,
    );
    let verifier = support.path().join("verify.py");
    write(
        &verifier,
        r#"import pathlib
import sys
answer = (pathlib.Path(sys.argv[1]) / "answer.txt").read_text().strip()
(pathlib.Path(sys.argv[1]) / "verifier-side-effect.txt").write_text("trusted verifier artifact\n")
raise SystemExit(0 if answer == "42" else 1)
"#,
    );
    let contract_verifier = support.path().join("verify-contract.py");
    write(
        &contract_verifier,
        r#"import pathlib
import sys
readme = (pathlib.Path(sys.argv[1]) / "README.md").read_text()
raise SystemExit(0 if readme == "A/B fixture\n" else 1)
"#,
    );
    let analysis_verifier = support.path().join("verify-analysis.py");
    write(
        &analysis_verifier,
        r#"import pathlib
import sys
message = pathlib.Path(sys.argv[1]).read_text()
raise SystemExit(0 if "README.md:1" in message else 1)
"#,
    );
    let tasks = support.path().join("tasks.jsonl");
    let task = serde_json::json!({
        "id": "paired-answer",
        "repo": repo.path(),
        "base_ref": "HEAD",
        "prompt": "Write answer.txt with the correct answer to six times seven.",
        "verify": [
            {
                "name": "external-answer",
                "category": "behavior",
                "weight": 2,
                "required": true,
                "command": ["python3", verifier, "{worktree}"],
                "timeout_seconds": 30
            },
            {
                "name": "public-contract",
                "category": "contract",
                "weight": 1,
                "required": false,
                "command": ["python3", contract_verifier, "{worktree}"],
                "timeout_seconds": 30
            }
        ],
        "protected_paths": ["README.md"]
    });
    let analysis_task = serde_json::json!({
        "id": "repository-analysis",
        "mode": "analysis",
        "repo": repo.path(),
        "base_ref": "HEAD",
        "prompt": "Identify one repository problem and cite exact evidence.",
        "verify": [{
            "name": "evidence-bearing-report",
            "category": "evidence",
            "weight": 1,
            "required": true,
            "command": ["python3", analysis_verifier, "{last_message}"],
            "timeout_seconds": 30
        }]
    });
    write(
        &tasks,
        &format!(
            "{}\n{}\n",
            serde_json::to_string(&task).unwrap(),
            serde_json::to_string(&analysis_task).unwrap()
        ),
    );

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new("python3")
        .arg(repo_root.join("scripts/benchmark-codemap-ab.py"))
        .arg(&tasks)
        .args(["--codex-bin", &format!("python3 {}", fake_codex.display())])
        .args([
            "--codemap-bin",
            &format!("python3 {}", fake_codemap.display()),
        ])
        .args(["--out-dir", out.path().to_str().unwrap()])
        .args([
            "--work-dir",
            out.path().join("worktrees").to_str().unwrap(),
        ])
        .output()
        .expect("A/B harness should run");
    assert!(
        output.status.success(),
        "A/B failed: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let summary: Value = serde_json::from_str(
        &fs::read_to_string(out.path().join("summary.json")).expect("summary json"),
    )
    .expect("valid summary");
    assert_eq!(summary["model"], "gpt-5.6-sol");
    assert_eq!(summary["reasoning_effort"], "xhigh");
    assert_eq!(
        summary["scoring_contract"]["primary_metric"],
        "weighted_external_completeness"
    );
    assert_eq!(
        summary["scoring_contract"]["resource_metrics_role"],
        "secondary_cost_only"
    );
    assert_eq!(summary["preflight"][0]["baseline_passed"], false);
    assert_eq!(summary["preflight"][1]["baseline_passed"], false);
    assert_eq!(summary["paired"]["valid_pairs"], 2);
    assert_eq!(summary["paired"]["codemap_wins"], 1);
    assert_eq!(summary["paired"]["control_wins"], 0);
    assert_eq!(summary["paired"]["ties"], 1);
    assert_eq!(summary["arms"]["control"]["passed_trials"], 1);
    assert_eq!(summary["arms"]["codemap"]["passed_trials"], 2);
    assert_eq!(summary["arms"]["codemap"]["mean_completeness_score"], 1.0);
    assert_eq!(
        summary["arms"]["control"]["category_coverage"]["behavior"]["score"],
        0.0
    );
    assert_eq!(
        summary["arms"]["control"]["category_coverage"]["contract"]["score"],
        1.0
    );
    assert!(summary["arms"]["control"]["category_coverage"]["evidence"]["score"] == 1.0);

    let results_text = fs::read_to_string(out.path().join("results.jsonl"))
        .expect("per-trial results jsonl");
    let results: Vec<Value> = results_text
        .lines()
        .map(|line| serde_json::from_str(line).expect("trial result"))
        .collect();
    assert_eq!(results.len(), 4);
    assert_eq!(
        results[0]["task_prompt_sha256"], results[1]["task_prompt_sha256"],
        "both arms must receive the same task text"
    );
    assert_eq!(
        results[0]["base_commit"], results[1]["base_commit"],
        "both arms must stay pinned to one resolved commit"
    );
    let control = results
        .iter()
        .find(|row| row["task_id"] == "paired-answer" && row["arm"] == "control")
        .expect("control result");
    let treatment = results
        .iter()
        .find(|row| row["task_id"] == "paired-answer" && row["arm"] == "codemap")
        .expect("codemap result");
    assert_eq!(control["codemap_protocol"]["invocation_count"], 0);
    assert_eq!(treatment["codemap_protocol"]["invocation_count"], 3);
    assert_eq!(treatment["codemap_protocol"]["compliant"], true);
    assert_eq!(treatment["codex"]["usage"]["input_tokens"], 100);
    assert_eq!(control["completeness"]["passed_criteria"], 1);
    assert_eq!(control["completeness"]["criteria"], 2);
    assert_eq!(control["completeness"]["score"], 0.333333);
    assert_eq!(treatment["completeness"]["score"], 1.0);
    let analysis_control = results
        .iter()
        .find(|row| row["task_id"] == "repository-analysis" && row["arm"] == "control")
        .expect("analysis control result");
    let analysis_treatment = results
        .iter()
        .find(|row| row["task_id"] == "repository-analysis" && row["arm"] == "codemap")
        .expect("analysis treatment result");
    assert_eq!(analysis_control["mode"], "analysis");
    assert_eq!(analysis_control["analysis_no_repo_changes"], true);
    assert_eq!(analysis_control["changed_paths"].as_array().unwrap().len(), 0);
    assert_eq!(analysis_control["verifiers"][0]["passed"], true);
    assert_eq!(analysis_control["codemap_protocol"]["invocation_count"], 0);
    assert_eq!(analysis_treatment["codemap_protocol"]["invocation_count"], 2);
    assert_eq!(analysis_treatment["codemap_protocol"]["root_ls"], true);
    assert_eq!(analysis_treatment["codemap_protocol"]["focused"], true);
    assert_eq!(analysis_treatment["codemap_protocol"]["compliant"], true);
    let markdown = fs::read_to_string(out.path().join("summary.md")).expect("summary markdown");
    assert!(markdown.contains("Externally verified result"));
    assert!(markdown.contains("Resource cost (secondary)"));
    assert!(markdown.contains("Token use never decides the winner"));
    assert!(
        out.path()
            .join("trials/paired-answer-r1-codemap/patch.diff")
            .exists(),
        "treatment patch should be preserved"
    );
    let treatment_patch = fs::read_to_string(
        out.path()
            .join("trials/paired-answer-r1-codemap/patch.diff"),
    )
    .expect("treatment patch");
    assert!(
        !treatment_patch.contains("verifier-side-effect"),
        "trusted verifier mutations must not be attributed to the model patch"
    );
    assert!(
        fs::read_dir(out.path().join("worktrees"))
            .expect("worktree parent")
            .next()
            .is_none(),
        "disposable worktrees should be removed"
    );
    let source_status = Command::new("git")
        .args(["status", "--short"])
        .current_dir(repo.path())
        .output()
        .expect("source git status");
    assert!(
        String::from_utf8_lossy(&source_status.stdout).trim().is_empty(),
        "benchmark source repository must remain unchanged"
    );
}
