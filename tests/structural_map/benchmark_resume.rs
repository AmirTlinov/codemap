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
