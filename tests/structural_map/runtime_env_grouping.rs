#[test]
fn runtime_groups_repeated_env_references_by_file_and_name() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/repeated-env.ts"),
        "export const first = process.env.CI;\nexport const second = process.env.CI;\nexport const third = process.env.CI;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "repeated env fixture"]);

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "packages/app/src", "--format", "json"],
    );
    assert_schema("schemas/runtime.schema.json", &runtime);
    let repeated = runtime["env"]
        .as_array()
        .expect("env surfaces")
        .iter()
        .filter(|surface| {
            surface["name"] == "CI" && surface["used_by"] == "packages/app/src/repeated-env.ts"
        })
        .collect::<Vec<_>>();
    assert_eq!(
        repeated.len(),
        1,
        "runtime env map should show one surface per file/name, not one row per repeated line: {runtime:#}"
    );
    assert_eq!(
        repeated[0]["locations"]
            .as_array()
            .expect("locations")
            .len(),
        3,
        "grouped env surface should preserve every exact line as locations: {runtime:#}"
    );
}

#[test]
fn runtime_hidden_expands_use_the_actual_scope() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/repeated-env.ts"),
        "export const first = process.env.CI;\nexport const second = process.env.NODE_ENV;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "runtime hidden expand fixture"]);

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &[
            "runtime",
            "packages/app/src",
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/runtime.schema.json", &runtime);
    let hidden = runtime["hidden"].as_array().expect("hidden");
    assert!(!hidden.is_empty(), "fixture should create runtime hidden groups: {runtime:#}");
    assert!(
        hidden.iter().all(|group| group["expand"]
            .as_str()
            .is_some_and(|expand| expand.starts_with("codemap runtime packages/app/src --include-hidden --limit ")
                && !expand.contains("<larger-number>"))),
        "runtime hidden expand should be an executable command for the current scope, not a placeholder: {runtime:#}"
    );
}

#[test]
fn runtime_exact_file_scope_exposes_env_and_routes() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/runtime-file.ts"),
        "const token = process.env.RUNTIME_TOKEN;\nrouter.get('/runtime-file', runtimeFileHandler);\nexport function runtimeFileHandler() {\n  return token;\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "runtime exact file scope fixture"]);

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &[
            "runtime",
            "packages/app/src/runtime-file.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/runtime.schema.json", &runtime);
    assert!(
        runtime["env"]
            .as_array()
            .expect("env")
            .iter()
            .any(|surface| surface["name"] == "RUNTIME_TOKEN"
                && surface["used_by"] == "packages/app/src/runtime-file.ts"),
        "runtime exact-file scope should inspect that file instead of treating it as an empty directory: {runtime:#}"
    );
    assert!(
        runtime["routes"]
            .as_array()
            .expect("routes")
            .iter()
            .any(|route| route["path"] == "/runtime-file"
                && route["file"] == "packages/app/src/runtime-file.ts"),
        "runtime exact-file scope should expose route facts from that file: {runtime:#}"
    );
}

#[test]
fn runtime_reports_hidden_proof_edges_when_limited() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/routes.ts"),
        "router.get('/runtime/one', oneHandler);\nrouter.get('/runtime/two', twoHandler);\nexport function oneHandler() { return true; }\nexport function twoHandler() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/tests/e2e/runtime.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('runtime routes', async ({ page }) => {\n  await page.goto('/runtime/one');\n  await page.goto('/runtime/two');\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "runtime proof edges fixture"]);

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "packages/app/src", "--limit", "1", "--format", "json"],
    );
    assert_schema("schemas/runtime.schema.json", &runtime);
    assert!(
        runtime["proof"]
            .as_array()
            .expect("proof")
            .iter()
            .all(|edge| edge["evidence"] == "e2e_visited_route"),
        "runtime proof should be tied to runtime surfaces, not generic file-level proof: {runtime:#}"
    );
    assert!(
        runtime["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|group| group["reason"] == "runtime proof edges hidden by limit"
                && group["expand"].as_str().is_some_and(|expand| {
                    expand.starts_with("codemap runtime packages/app/src --include-hidden --limit ")
                        && !expand.contains("<larger-number>")
                })),
        "runtime proof truncation must be visible and expandable, not silent: {runtime:#}"
    );
}
