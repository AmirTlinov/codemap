#[test]
fn frozen_flagship_gate_accepts_complete_evidence_and_rejects_tampering() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new("python3")
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
