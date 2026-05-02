#[test]
fn ls_help_exposes_stable_rfc_sections() {
    let output = codemap()
        .args(["ls", "--help"])
        .output()
        .expect("ls help should run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help utf8");
    for expected in ["observed", "links", "roles", "proof", "unknown", "hidden"] {
        assert!(
            stdout.contains(expected),
            "ls help should expose RFC section `{expected}`: {stdout}"
        );
    }
}
