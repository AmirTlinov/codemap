#[test]
fn ab_resume_discards_incomplete_trial_artifacts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let temp = TempDir::new().expect("resume fixture");
    let trial = temp.path().join("incomplete-trial");
    fs::create_dir_all(&trial).expect("trial dir");
    write(&trial.join("codemap-cache/invocations.log"), "stale invocation\n");
    let probe = r#"import pathlib, runpy, sys
sys.path.insert(0, sys.argv[1])
module = runpy.run_path(sys.argv[2])
trial = pathlib.Path(sys.argv[3])
try:
    module["existing_trial"](trial, "fingerprint", False)
    raise AssertionError("incomplete trial must fail closed without --resume")
except ValueError:
    pass
assert trial.exists()
assert module["existing_trial"](trial, "fingerprint", True) is None
assert not trial.exists()
"#;
    let output = python()
        .args([
            "-c",
            probe,
            root.join("scripts").to_str().unwrap(),
            root.join("scripts/benchmark-codemap-ab.py")
                .to_str()
                .unwrap(),
            trial.to_str().unwrap(),
        ])
        .output()
        .expect("resume probe");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn ab_retries_only_the_first_infrastructure_failure_and_preserves_its_evidence() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let temp = TempDir::new().expect("infrastructure retry fixture");
    let probe = r#"import json, pathlib, runpy, sys
sys.path.insert(0, sys.argv[1])
module = runpy.run_path(sys.argv[2])
root = pathlib.Path(sys.argv[3])

def result(path, reason, attempt=None):
    path.mkdir(parents=True, exist_ok=True)
    events = path / "events.jsonl"
    events.write_text('{"event":"kept"}\n', encoding="utf-8")
    body = {
        "trial_fingerprint": "fingerprint",
        "invalidation_reason": reason,
        "outcome_passed": False,
        "codex": {"events_artifact": str(events)},
    }
    if attempt is not None:
        body["infrastructure_attempt"] = attempt
    (path / "result.json").write_text(json.dumps(body), encoding="utf-8")
    return body

trial = root / "retried"
result(trial, "codex_timeout")
assert module["existing_trial"](trial, "fingerprint", True) is None
archived = trial / "attempts" / "attempt-1"
archived_result = json.loads((archived / "result.json").read_text(encoding="utf-8"))
assert pathlib.Path(archived_result["codex"]["events_artifact"]) == archived / "events.jsonl"
assert (archived / "events.jsonl").is_file()
assert module["current_attempt"](trial) == 2

(trial / "partial-attempt-2.log").write_text("stale", encoding="utf-8")
assert module["existing_trial"](trial, "fingerprint", True) is None
assert not (trial / "partial-attempt-2.log").exists()
assert (archived / "events.jsonl").is_file()

second = result(trial, "codex_crash", 2)
assert module["existing_trial"](trial, "fingerprint", True) == second
assert (trial / "result.json").is_file()
assert (archived / "result.json").is_file()

for name, reason in [("verifier-loss", None), ("protocol-invalid", "treatment_protocol_noncompliant")]:
    candidate = root / name
    expected = result(candidate, reason, 1)
    assert module["existing_trial"](candidate, "fingerprint", True) == expected
    assert not (candidate / "attempts").exists()
"#;
    let output = python()
        .args([
            "-c",
            probe,
            root.join("scripts").to_str().unwrap(),
            root.join("scripts/benchmark-codemap-ab.py")
                .to_str()
                .unwrap(),
            temp.path().to_str().unwrap(),
        ])
        .output()
        .expect("infrastructure retry probe");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn ab_automatically_retries_one_crashed_agent_run_with_the_same_fingerprint() {
    let repo = TempDir::new().expect("retry fixture repo");
    let support = TempDir::new().expect("retry support");
    let out = TempDir::new().expect("retry output");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(&repo.path().join("README.md"), "retry fixture\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let fake_codemap = support.path().join("fake-codemap.py");
    write(
        &fake_codemap,
        r#"import sys
if "--version" in sys.argv:
    print("codemap retry-fixture")
"#,
    );
    let fake_codex = support.path().join("fake-codex.py");
    write(
        &fake_codex,
        r#"import json, os, pathlib, sys
if "--version" in sys.argv:
    print("codex-cli retry-fixture")
    raise SystemExit(0)
counter = pathlib.Path(os.environ["FAKE_CODEX_COUNTER"])
attempt = int(counter.read_text()) + 1 if counter.exists() else 1
counter.write_text(str(attempt))
if attempt == 1:
    print("first attempt crashed", file=sys.stderr)
    raise SystemExit(17)
args = sys.argv[1:]
last_message = pathlib.Path(args[args.index("-o") + 1])
last_message.write_text("Confirmed issue at README.md:1\n")
with open(os.environ["CODEMAP_AB_INVOCATION_LOG"], "a") as stream:
    stream.write(json.dumps({"argv": ["cone", "README.md"], "status": 0, "agent_direct": True}) + "\n")
print(json.dumps({"type":"item.completed","item":{"type":"command_execution","command":"codemap cone README.md","exit_code":0,"status":"completed"}}))
print(json.dumps({"type":"turn.completed","usage":{"input_tokens":10,"output_tokens":2}}))
"#,
    );
    let verifier = support.path().join("verify.py");
    write(
        &verifier,
        r#"import pathlib, sys
path = pathlib.Path(sys.argv[1])
raise SystemExit(0 if path.exists() and "README.md:1" in path.read_text() else 1)
"#,
    );
    let python_bin = python_executable();
    let task = serde_json::json!({
        "id": "retry-agent",
        "mode": "analysis",
        "repo": repo.path(),
        "base_ref": "HEAD",
        "prompt": "Find the issue and cite its exact README evidence.",
        "verify": [{
            "name": "evidence",
            "category": "investigation",
            "required": true,
            "command": [python_bin, verifier, "{last_message}"]
        }]
    });
    let tasks = support.path().join("tasks.jsonl");
    write(&tasks, &(serde_json::to_string(&task).unwrap() + "\n"));
    let codex_argv = serde_json::to_string(&[python_bin, fake_codex.to_str().unwrap()]).unwrap();
    let codemap_argv =
        serde_json::to_string(&[python_bin, fake_codemap.to_str().unwrap()]).unwrap();
    let output = python()
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/benchmark-codemap-ab.py"))
        .arg(&tasks)
        .args(["--codex-argv-json", &codex_argv])
        .args(["--codemap-argv-json", &codemap_argv])
        .args(["--out-dir", out.path().to_str().unwrap()])
        .args(["--work-dir", out.path().join("worktrees").to_str().unwrap()])
        .arg("--treatment-preflight")
        .env("FAKE_CODEX_COUNTER", support.path().join("counter"))
        .output()
        .expect("automatic retry run");
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(support.path().join("counter")).unwrap(), "2");
    let trial = out.path().join("trials/retry-agent-r1-codemap");
    let first: Value = serde_json::from_str(
        &fs::read_to_string(trial.join("attempts/attempt-1/result.json")).unwrap(),
    )
    .unwrap();
    let second: Value =
        serde_json::from_str(&fs::read_to_string(trial.join("result.json")).unwrap()).unwrap();
    assert_eq!(first["infrastructure_attempt"], 1);
    assert_eq!(first["invalidation_reason"], "codex_crash");
    assert_eq!(second["infrastructure_attempt"], 2);
    assert_eq!(second["invalidation_reason"], Value::Null);
    assert_eq!(second["prior_attempts"][0], "attempts/attempt-1/result.json");
    assert_eq!(first["trial_fingerprint"], second["trial_fingerprint"]);
}

#[test]
fn benchmark_cleanup_stops_registered_process_without_competing_output_collection() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let temp = TempDir::new().expect("process lifecycle fixture");
    let probe = r#"import pathlib, runpy, sys, threading, time
module = runpy.run_path(sys.argv[2])
result = {}
def run():
    result["value"] = module["run_process"](
        [sys.executable, "-c", "import time; time.sleep(60)"],
        pathlib.Path(sys.argv[3]),
        60,
    )
worker = threading.Thread(target=run)
worker.start()
deadline = time.monotonic() + 5
while not module["_ACTIVE_PROCESSES"] and time.monotonic() < deadline:
    time.sleep(0.01)
assert module["_ACTIVE_PROCESSES"]
module["terminate_active_processes"]()
worker.join(10)
assert not worker.is_alive()
assert result["value"].status != 0
assert not module["_ACTIVE_PROCESSES"]
"#;
    let output = python()
        .args([
            "-c",
            probe,
            root.join("scripts").to_str().unwrap(),
            root.join("scripts/benchmark_parallel.py")
                .to_str()
                .unwrap(),
            temp.path().to_str().unwrap(),
        ])
        .output()
        .expect("process lifecycle probe");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
