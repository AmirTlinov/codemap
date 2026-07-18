#[test]
fn release_contract_covers_supported_downloads_and_attested_identity() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow = fs::read_to_string(root.join(".github/workflows/release.yml"))
        .expect("release workflow");
    for target in [
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "x86_64-unknown-linux-gnu",
        "x86_64-pc-windows-msvc",
    ] {
        assert!(workflow.contains(target), "missing release target {target}");
    }
    for owner in [
        "scripts/package-release.py",
        "scripts/release-upgrade-smoke.py",
        "actions/attest-build-provenance@v3",
        "downloaded-smoke:",
        "cargo package --locked",
    ] {
        assert!(workflow.contains(owner), "missing release owner {owner}");
    }
    let manifest = fs::read_to_string(root.join("Cargo.toml")).expect("Cargo manifest");
    assert!(!manifest.contains("\"/fixtures/**\""));
    assert!(!manifest.contains("\"/tests/**\""));
    for path in ["CHANGELOG.md", "SECURITY.md", "docs/DISTRIBUTION.md"] {
        assert!(root.join(path).is_file(), "missing distribution contract {path}");
    }
}

#[test]
fn release_archive_is_deterministic_and_verifies_after_extraction() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = python()
        .arg(root.join("tests/release_package_fixture.py"))
        .env("CODEMAP_FIXTURE_BIN", env!("CARGO_BIN_EXE_codemap"))
        .output()
        .expect("release package fixture should run");
    assert!(
        output.status.success(),
        "release package fixture failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn flagship_evidence_archive_keeps_failed_and_accepted_attempts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = python()
        .arg(root.join("tests/flagship_evidence_package_fixture.py"))
        .output()
        .expect("flagship evidence package fixture should run");
    assert!(
        output.status.success(),
        "flagship evidence fixture failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
