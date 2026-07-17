// Responsibility: exact ls proof signal ordering

#[test]
fn exact_file_ls_leads_with_mediated_proof_before_role_noise() {
    let repo = TempDir::new().expect("mediated ls repo");
    let cache = TempDir::new().expect("mediated ls cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"mediated-ls","private":true}"#,
    );
    write(
        &repo.path().join("src/map/classify.ts"),
        "export function classifyResponse() { return true; }\n",
    );
    write(
        &repo.path().join("src/map/paths.ts"),
        "import { classifyResponse } from './classify';\nexport const runtimePaths = classifyResponse();\n",
    );
    write(
        &repo.path().join("tests/runtime_transform_paths.test.ts"),
        "test('runtime paths', () => { expect(classify).toBeDefined(); });\n",
    );
    for name in ["classification_alpha", "classification_beta"] {
        write(
            &repo.path().join(format!("tests/{name}.test.ts")),
            "test('role noise', () => { expect(true).toBe(true); });\n",
        );
    }
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "mediated ls fixture"]);

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["ls", "src/map/classify.ts", "--limit", "3"])
        .output()
        .expect("bounded exact ls should run");
    assert!(
        output.status.success(),
        "exact ls failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("exact ls utf8");
    let mediated = markdown
        .find("test_surface_tokens_via_direct_consumer")
        .expect("mediated proof should be visible");
    let role = markdown
        .find("test_role_surface_match")
        .expect("role fallback should remain visible after stronger proof");
    assert!(
        markdown.contains("tests/runtime_transform_paths.test.ts") && mediated < role,
        "exact ls should lead with the mediated behavior test before role noise: {markdown}"
    );
}
