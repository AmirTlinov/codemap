#[test]
fn changed_default_markdown_collapses_extra_proof_command_groups() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join(".codemap.yml"),
        r#"version: 1

proof:
  changed:
    - make verify-alpha
    - make verify-beta
    - make verify-gamma
    - make verify-delta
    - make verify-epsilon
    - make verify-zeta
"#,
    );
    write(
        &repo.path().join("Makefile"),
        "verify-alpha:\n\ttrue\n\nverify-beta:\n\ttrue\n\nverify-gamma:\n\ttrue\n\nverify-delta:\n\ttrue\n\nverify-epsilon:\n\ttrue\n\nverify-zeta:\n\ttrue\n",
    );
    for name in ["alpha", "beta", "gamma", "delta", "epsilon", "zeta"] {
        write(
            &repo.path().join(format!("src/{name}.ts")),
            &format!("export const {name}Value = 1;\n"),
        );
    }
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "proof command fixture"]);
    for name in ["alpha", "beta", "gamma", "delta", "epsilon", "zeta"] {
        write(
            &repo.path().join(format!("src/{name}.ts")),
            &format!("export const {name}Value = 2;\n"),
        );
    }

    let json = run_json(repo.path(), cache.path(), &["changed", "--format", "json"]);
    let json_command_count = json["proof"]["commands"]
        .as_array()
        .expect("proof commands")
        .len();
    assert!(
        json_command_count >= 6,
        "JSON report should keep complete proof command groups: {json:#}"
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed"])
        .output()
        .expect("changed markdown should run");
    assert!(
        output.status.success(),
        "changed failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert_eq!(
        markdown.matches("`make verify-").count(),
        3,
        "default changed should show a bounded proof command sample: {markdown}"
    );
    assert!(
        markdown.contains("- hidden runnable command surface groups: `3`")
            && markdown.contains("runnable command surface groups hidden by compact changed view"),
        "default changed should expose hidden runnable command surface groups with expand: {markdown}"
    );

    let proof_section = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--section", "proof"])
        .output()
        .expect("changed proof section should run");
    assert!(
        proof_section.status.success(),
        "changed --section proof failed: {}",
        String::from_utf8_lossy(&proof_section.stderr)
    );
    let proof_markdown = String::from_utf8(proof_section.stdout).expect("markdown utf8");
    assert_eq!(
        proof_markdown.matches("\n### `make verify-").count(),
        json_command_count,
        "changed --section proof should expand all proof command groups: {proof_markdown}"
    );
}

#[test]
fn changed_default_markdown_compacts_small_sets_with_many_unknowns() {
    let (repo, cache) = fixture();
    for name in ["alpha", "beta", "gamma", "delta"] {
        write(
            &repo.path().join(format!("packages/replay/src/{name}.ts")),
            &format!("export const {name}Value = 1;\n"),
        );
    }

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed"])
        .output()
        .expect("changed markdown should run");
    assert!(
        output.status.success(),
        "changed failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    let line_count = markdown.lines().count();
    assert!(
        line_count <= 120,
        "default changed should stay within the daily line budget for small noisy changes; lines={line_count}\n{markdown}"
    );
    assert!(
        markdown.contains("- `direct_test_import_not_found`: `4`; sample:")
            && markdown.contains("- `nearest_proof_scope`: `4`; sample:")
            && markdown.contains("changed --section unknown"),
        "default changed should compact repeated Unknowns but keep exact expand paths: {markdown}"
    );
    assert!(
        !markdown.contains("reason: no direct test import"),
        "default changed should leave verbose Unknown detail to --section unknown: {markdown}"
    );
}

#[test]
fn changed_large_link_summary_keeps_soft_matches_out_of_verification_mass() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"large-soft-fixture","private":true,"scripts":{"test":"vitest run"}}"#,
    );
    for index in 0..21 {
        write(
            &repo.path().join(format!("src/feature-{index}.ts")),
            &format!("export function feature{index}Route() {{ return {index}; }}\n"),
        );
        write(
            &repo.path().join(format!("tests/feature-{index}.test.ts")),
            &format!(
                "test('feature {index} route smoke', () => {{ expect('feature {index} route').toBeTruthy(); }});\n"
            ),
        );
    }
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "large soft fixture"]);
    for index in 0..21 {
        write(
            &repo.path().join(format!("src/feature-{index}.ts")),
            &format!("export function feature{index}Route() {{ return {index} + 1; }}\n"),
        );
    }

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .arg("changed")
        .output()
        .expect("changed should run");
    assert!(
        output.status.success(),
        "changed failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("changed markdown utf8");
    assert!(
        markdown.contains("- clusters: `21`")
            && markdown.contains("verification=0; soft=")
            && !markdown.contains("clusters: `; soft=")
            && !markdown.contains("verification=021"),
        "large changed link summary should keep cluster count and soft evidence separate: {markdown}"
    );
}
