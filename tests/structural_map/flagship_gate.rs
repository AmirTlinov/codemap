#[test]
fn frozen_flagship_gate_accepts_complete_evidence_and_rejects_tampering() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = python()
        .arg(root.join("tests/flagship_gate_fixture.py"))
        .output()
        .expect("flagship fixture should run");
    assert!(
        output.status.success(),
        "flagship gate failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn flagship_materializer_exposes_the_frozen_provenance_criterion() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = python()
        .arg(root.join("tests/flagship_materialize_fixture.py"))
        .output()
        .expect("flagship materializer fixture should run");
    assert!(
        output.status.success(),
        "flagship materializer failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn flagship_trajectory_keeps_raw_actions_diff_verifiers_and_cost() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = python()
        .arg(root.join("tests/flagship_trajectory_fixture.py"))
        .output()
        .expect("flagship trajectory fixture should run");
    assert!(
        output.status.success(),
        "flagship trajectory failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn flagship_investigations_verify_source_backed_outcomes_not_path_lists() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = python()
        .arg(root.join("tests/flagship_verifier_fixture.py"))
        .output()
        .expect("flagship verifier fixture should run");
    assert!(
        output.status.success(),
        "flagship verifier failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
