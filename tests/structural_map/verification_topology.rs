#[test]
fn external_python_verifier_is_an_observed_process_not_a_verdict() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("src/contract.py"),
        "def contract_value():\n    return 7\n",
    );
    write(
        &repo.path().join("tests/test_contract.py"),
        r#"import subprocess
from src.contract import contract_value

def test_contract():
    assert contract_value() == 7
    subprocess.run(["python3", "tools/verify_external.py"], check=True)
"#,
    );
    write(
        &repo.path().join("tools/verify_external.py"),
        "raise SystemExit(0)\n",
    );
    write(&repo.path().join("pyproject.toml"), "[project]\nname='topology'\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "verification topology fixture"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "src/contract.py", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let topology = &proof["verification_topology"];
    assert!(
        topology["direct"]
            .as_array()
            .expect("direct topology")
            .iter()
            .any(|relation| relation["relation"] == "verifies_directly"
                && relation["subject"] == "src/contract.py"
                && relation["object"] == "tests/test_contract.py"),
        "an exact test import should be the direct verification relation: {proof:#}"
    );
    assert!(
        topology["runnable"]
            .as_array()
            .expect("runnable topology")
            .iter()
            .any(|relation| relation["relation"] == "contains_sensor"
                && relation["subject"]
                    .as_str()
                    .is_some_and(|command| command.contains("pytest"))),
        "the runnable command should remain a separate contains_sensor relation: {proof:#}"
    );
    assert!(
        topology["runnable"]
            .as_array()
            .expect("runnable topology")
            .iter()
            .any(|relation| relation["relation"] == "invokes_process"
                && relation["subject"] == "tests/test_contract.py"
                && relation["object"] == "tools/verify_external.py"),
        "a static subprocess verifier should be linked through invokes_process: {proof:#}"
    );
    assert!(
        proof["wiring"].as_array().expect("wiring").iter().any(|fact| {
            fact["stage"] == "invokes_process"
                && fact["status"] == "wired"
                && fact["effect"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("sufficiency are not claimed")
        }),
        "process linkage must stay observational rather than claiming a verdict: {proof:#}"
    );
    assert!(
        topology["horizon"]["certificate_id"]
            .as_str()
            .is_some_and(|id| id.starts_with("verification-v1:")),
        "verification topology needs a machine-checkable coverage horizon: {proof:#}"
    );
}

#[test]
fn dynamic_subprocess_target_stays_unknown_external() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    write(&repo.path().join("src/value.py"), "VALUE = 1\n");
    write(
        &repo.path().join("tests/test_value.py"),
        "import subprocess\nfrom src.value import VALUE\n\ndef test_value():\n    command = load_command()\n    subprocess.run(command, check=True)\n    assert VALUE == 1\n",
    );
    write(&repo.path().join("pyproject.toml"), "[project]\nname='dynamic'\n");

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "src/value.py", "--format", "json"],
    );
    let topology = &proof["verification_topology"];
    assert!(
        !topology["unknown_external"]
            .as_array()
            .expect("unknown external")
            .is_empty()
            && topology["horizon"]["status"] == "open",
        "dynamic subprocess targets must open the external verification horizon: {proof:#}"
    );
    assert!(
        topology["horizon"]["reasons"]
            .as_array()
            .expect("horizon reasons")
            .iter()
            .any(|reason| reason == "external_runtime_boundary"),
        "the shared horizon must name the external runtime boundary: {proof:#}"
    );
}
