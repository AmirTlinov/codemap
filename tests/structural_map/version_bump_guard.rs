#[test]
fn version_bump_guard_protects_released_identity_not_each_commit() {
    let repo = TempDir::new().expect("repo tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"version-guard-fixture\"\nversion = \"0.2.0\"\nedition = \"2024\"\n",
    );
    write(&repo.path().join("src/main.rs"), "fn main() {}\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "baseline"]);
    git(repo.path(), &["tag", "v0.2.0"]);

    write(&repo.path().join("src/main.rs"), "fn main() { println!(\"changed\"); }\n");
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/check-version-bump.py");
    let missing_bump = python()
        .arg(&script)
        .current_dir(repo.path())
        .output()
        .expect("version guard should run");
    assert!(
        !missing_bump.status.success(),
        "changed source without a version bump should fail"
    );
    let stderr = String::from_utf8_lossy(&missing_bump.stderr);
    assert!(
        stderr.contains("src/main.rs")
            && stderr.contains("changed files require Cargo.toml package version bump")
            && stderr.contains("0.2.0 -> 0.2.0"),
        "guard should explain the missing version bump: {stderr}"
    );

    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"version-guard-fixture\"\nversion = \"0.2.1\"\nedition = \"2024\"\n",
    );
    let bumped = python()
        .arg(&script)
        .current_dir(repo.path())
        .output()
        .expect("version guard should run");
    assert!(
        bumped.status.success(),
        "changed source with a higher version should pass: stderr={}",
        String::from_utf8_lossy(&bumped.stderr)
    );
    assert!(
        String::from_utf8_lossy(&bumped.stderr).contains("version bump ok: 0.2.0 -> 0.2.1"),
        "guard should report the visible version move"
    );

    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "prepare unreleased version"]);
    write(
        &repo.path().join("src/main.rs"),
        "fn main() { println!(\"another unreleased change\"); }\n",
    );
    let accumulated = python()
        .arg(&script)
        .current_dir(repo.path())
        .output()
        .expect("version guard should run");
    assert!(
        accumulated.status.success(),
        "more work may accumulate under one unreleased identity: stderr={}",
        String::from_utf8_lossy(&accumulated.stderr)
    );
}
