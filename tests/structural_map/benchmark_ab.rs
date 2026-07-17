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
import os
import pathlib
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
codemap_commands = []
if treatment and analysis:
    codemap_commands.append(["cone", "README.md"])
elif treatment:
    codemap_commands.extend((["ls", "."], ["changed"], ["proof", "changed"], ["changed"], ["proof", "changed"]))
for command in codemap_commands:
    with open(os.environ["CODEMAP_AB_INVOCATION_LOG"], "a") as stream:
        stream.write(json.dumps({"argv": command, "status": 0, "agent_direct": True}) + "\n")
    print(json.dumps({"type": "item.completed", "item": {"type": "command_execution", "command": "/bin/zsh -lc 'codemap " + " ".join(command) + "'", "exit_code": 0, "status": "completed"}}))
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
    let python_bin = python_executable();
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
                "command": [python_bin, verifier, "{worktree}"],
                "timeout_seconds": 30
            },
            {
                "name": "public-contract",
                "category": "contract",
                "weight": 1,
                "required": false,
                "command": [python_bin, contract_verifier, "{worktree}"],
                "timeout_seconds": 30
            }
        ]
    });
    let analysis_task = serde_json::json!({
        "id": "repository-analysis",
        "mode": "analysis",
        "repo": repo.path(),
        "base_ref": "HEAD",
        "prompt": "Inspect README.md, identify one repository problem, and cite exact evidence.",
        "verify": [{
            "name": "evidence-bearing-report",
            "category": "evidence",
            "weight": 1,
            "required": true,
            "command": [python_bin, analysis_verifier, "{last_message}"],
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
    let codex_argv = serde_json::to_string(&[python_bin, fake_codex.to_str().unwrap()]).unwrap();
    let codemap_argv =
        serde_json::to_string(&[python_bin, fake_codemap.to_str().unwrap()]).unwrap();
    let output = python()
        .arg(repo_root.join("scripts/benchmark-codemap-ab.py"))
        .arg(&tasks)
        .args(["--codex-argv-json", &codex_argv])
        .args(["--codemap-argv-json", &codemap_argv])
        .args(["--out-dir", out.path().to_str().unwrap()])
        .args([
            "--work-dir",
            out.path().join("worktrees").to_str().unwrap(),
        ])
        .args(["--parallel-pairs", "2"])
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
    assert_eq!(summary["reasoning_effort"], "high");
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
    let summary_identity = &summary["report_prelude"]["codemap"];
    assert_eq!(summary_identity["resolution"], "explicit");
    assert_eq!(summary_identity["diagnostic_state"], "unavailable");
    assert_eq!(
        summary_identity["build_identity"]["binary_sha256"],
        executable_sha256(&fake_codemap)
    );
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
    assert_eq!(treatment["codemap_protocol"]["invocation_count"], 5);
    assert_eq!(treatment["codemap_protocol"]["first_entry"], "ls .");
    assert_eq!(treatment["codemap_protocol"]["entry_kind"], "root");
    assert_eq!(treatment["codemap_protocol"]["root_entry"], true);
    assert_eq!(treatment["codemap_protocol"]["exact_entry"], false);
    assert_eq!(treatment["codemap_protocol"]["compliant"], false);
    assert_eq!(treatment["run_valid"], true);
    assert_eq!(treatment["outcome_passed"], true);
    assert_eq!(treatment["runtime"]["codex_home"], "isolated");
    assert_eq!(treatment["runtime"]["auth"], "linked");
    assert_eq!(treatment["runtime"]["extensions"], "disabled");
    assert_eq!(treatment["report_prelude"]["codemap"], *summary_identity);
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
    assert_eq!(analysis_treatment["codemap_protocol"]["invocation_count"], 1);
    assert_eq!(analysis_treatment["codemap_protocol"]["first_entry"], "cone README.md");
    assert_eq!(analysis_treatment["codemap_protocol"]["entry_kind"], "exact");
    assert_eq!(analysis_treatment["codemap_protocol"]["root_entry"], false);
    assert_eq!(analysis_treatment["codemap_protocol"]["exact_entry"], true);
    assert_eq!(analysis_treatment["codemap_protocol"]["mixed"], false);
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
    let treatment_out = TempDir::new().expect("treatment preflight output");
    let treatment_preflight = python()
        .arg(repo_root.join("scripts/benchmark-codemap-ab.py"))
        .arg(&tasks)
        .args(["--codex-argv-json", &codex_argv])
        .args(["--codemap-argv-json", &codemap_argv])
        .args(["--out-dir", treatment_out.path().to_str().unwrap()])
        .args(["--parallel-pairs", "2", "--treatment-preflight"])
        .output()
        .expect("treatment preflight should run");
    assert!(treatment_preflight.status.success());
    let treatment_results = fs::read_to_string(
        treatment_out.path().join("treatment-preflight-results.jsonl"),
    )
    .expect("treatment preflight results");
    assert_eq!(treatment_results.lines().count(), 2);
    assert!(treatment_results.lines().all(|line| {
        serde_json::from_str::<Value>(line).unwrap()["arm"] == "codemap"
    }));
    let source_status = Command::new("git")
        .args(["status", "--short"])
        .current_dir(repo.path())
        .output()
        .expect("source git status");
    assert!(
        String::from_utf8_lossy(&source_status.stdout).trim().is_empty(),
        "benchmark source repository must remain unchanged"
    );
    use std::io::Write;
    writeln!(
        fs::OpenOptions::new()
            .append(true)
            .open(&fake_codemap)
            .expect("open fake codemap"),
        "# identity-changing bytes"
    )
    .expect("change fake codemap bytes");
    let resumed = python()
        .arg(repo_root.join("scripts/benchmark-codemap-ab.py"))
        .arg(&tasks)
        .args(["--codex-argv-json", &codex_argv])
        .args(["--codemap-argv-json", &codemap_argv])
        .args(["--out-dir", out.path().to_str().unwrap()])
        .args([
            "--work-dir",
            out.path().join("worktrees").to_str().unwrap(),
        ])
        .arg("--resume")
        .output()
        .expect("resume with changed binary should run");
    assert!(!resumed.status.success());
    assert!(
        String::from_utf8_lossy(&resumed.stderr).contains("different configuration"),
        "binary bytes must participate in the trial fingerprint: {}",
        String::from_utf8_lossy(&resumed.stderr)
    );
}

#[test]
fn ab_identity_does_not_treat_config_as_the_executable() {
    let caller = TempDir::new().expect("A/B identity caller");
    let target = TempDir::new().expect("A/B identity target");
    let wrapper = caller.path().join("relative tools/ab codemap.py");
    let config = target.path().join("benchmark config.json");
    write(
        &wrapper,
        r#"import pathlib
import sys
if pathlib.Path(sys.argv[1]).read_text().strip() != "ab-config":
    raise SystemExit(42)
if "--version" in sys.argv:
    print("codemap 8.7.6")
elif "doctor" in sys.argv:
    print("{}")
"#,
    );
    write(&config, "ab-config\n");
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let explicit = serde_json::to_string(&[
        python_executable(),
        "relative tools/ab codemap.py",
        config.to_str().unwrap(),
    ])
    .unwrap();
    let probe = r#"import json, pathlib, sys
sys.path.insert(0, sys.argv[1])
from codemap_identity import benchmark_binary_identity, resolve_codemap_command
command, source = resolve_codemap_command(json.loads(sys.argv[5]), pathlib.Path(sys.argv[2]), cwd=pathlib.Path(sys.argv[3]))
print(json.dumps(benchmark_binary_identity(command, source, pathlib.Path(sys.argv[4]))))
"#;
    let output = python()
        .args([
            "-c",
            probe,
            repo_root.join("scripts").to_str().unwrap(),
            repo_root.to_str().unwrap(),
            caller.path().to_str().unwrap(),
            target.path().to_str().unwrap(),
            &explicit,
        ])
        .output()
        .expect("A/B wrapper identity probe");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let identity: Value = serde_json::from_slice(&output.stdout).expect("identity json");
    let wrapper_path = comparable_canonical_path(&wrapper);
    assert_eq!(identity["command_argv"][1], wrapper_path);
    assert_eq!(identity["command_argv"][2], config.to_string_lossy().as_ref());
    assert_eq!(identity["build_identity"]["executable_path"], wrapper_path);
    assert_eq!(identity["build_identity"]["binary_sha256"], executable_sha256(&wrapper));
    let artifacts = identity["command_artifacts"].as_array().unwrap();
    assert_eq!(artifacts.len(), 2, "config must not enter executable artifacts: {identity:#}");
    assert!(artifacts.iter().all(|artifact| artifact["path"] != config.to_string_lossy().as_ref()));
}
