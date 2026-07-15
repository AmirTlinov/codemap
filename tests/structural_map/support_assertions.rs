fn run_json(repo: &Path, cache: &Path, args: &[&str]) -> Value {
    let output = codemap()
        .current_dir(repo)
        .env("CODEMAP_CACHE_DIR", cache)
        .args(args)
        .output()
        .expect("codemap should run");
    let report: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "codemap {args:?} emitted invalid JSON after exit {:?}: {error}; stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    let expected_result = match output.status.code() {
        Some(0) => None,
        Some(10) => Some("valid_empty_map"),
        Some(20) => Some("invalid_anchor"),
        code => panic!(
            "codemap {args:?} failed with exit {code:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        ),
    };
    if let Some(expected_result) = expected_result {
        assert_eq!(
            report["agent"]["result"], expected_result,
            "typed non-zero map exit must agree with the stable agent envelope: {report:#}"
        );
    }
    report
}

fn assert_no_export_dialog_role_name_proof(
    app_source: &str,
    design_source: Option<&str>,
    message: &str,
) {
    assert_no_export_role_name_proof(app_source, design_source, "dialog", message);
}

fn assert_no_export_role_name_proof(
    app_source: &str,
    design_source: Option<&str>,
    e2e_role: &str,
    message: &str,
) {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "dialog-accessible-same-line-negative-fixture",
  "private": true,
  "scripts": {
    "test": "vitest run",
    "test:e2e": "playwright test"
  }
}
"#,
    );
    write(
        &repo.path().join("src/features/studio/export-dialog.tsx"),
        app_source,
    );
    if let Some(design_source) = design_source {
        write(&repo.path().join("src/design/dialog.tsx"), design_source);
    }
    write(
        &repo
            .path()
            .join("tests/e2e/canvas-blueprint-overlay-workflows.spec.ts"),
        &format!(
            r#"import {{ expect, test }} from '@playwright/test';

test('command palette opens export dialog', async ({{ page }}) => {{
  await page.goto('/studio');
  await expect(page.getByRole('{e2e_role}', {{ name: 'Export' }})).toBeVisible();
}});
"#
        ),
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "src/features/studio/export-dialog.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"].as_array().expect("proofs").is_empty(),
        "{message}: {proof:#}"
    );
    assert!(
        !proof["fallback"].as_array().expect("fallback").is_empty(),
        "without a structural proof, broad fallback must remain visible: {proof:#}"
    );
}

fn assert_no_export_dialog_role_name_proof_with_e2e_source(e2e_source: &str, message: &str) {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "dialog-playwright-negative-fixture",
  "private": true,
  "scripts": {
    "test": "vitest run",
    "test:e2e": "playwright test"
  }
}
"#,
    );
    write(
        &repo.path().join("src/features/studio/export-dialog.tsx"),
        r#"export function ExportDialog() {
  return <div role="dialog" aria-labelledby="export-title"><h2 id="export-title">Export</h2></div>;
}
"#,
    );
    write(
        &repo.path().join("tests/e2e/canvas-dialog.spec.ts"),
        e2e_source,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "src/features/studio/export-dialog.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"].as_array().expect("proofs").is_empty(),
        "{message}: {proof:#}"
    );
    assert!(
        !proof["fallback"].as_array().expect("fallback").is_empty(),
        "without a structural proof, broad fallback must remain visible: {proof:#}"
    );
}
