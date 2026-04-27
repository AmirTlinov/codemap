use std::process::Command;

#[test]
fn doctor_runs() {
    let output = Command::new(env!("CARGO_BIN_EXE_ctx"))
        .arg("doctor")
        .output()
        .expect("failed to run ctx doctor");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("# ctx doctor"));
    assert!(stdout.contains("Zero-footprint default: true"));
}

#[test]
fn start_stub_is_honest_about_low_confidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_ctx"))
        .args(["start", "--task", "fix broken save"])
        .output()
        .expect("failed to run ctx start");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("# Task Context Capsule"));
    assert!(stdout.contains("Low"));
    assert!(stdout.contains("routing engine is not implemented yet"));
}
