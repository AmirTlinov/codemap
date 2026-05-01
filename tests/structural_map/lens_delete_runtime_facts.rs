#[test]
fn delete_lens_reports_package_manifest_export_blocker() {
    let (repo, cache) = fixture();

    let delete_map = run_json(
        repo.path(),
        cache.path(),
        &["delete", "packages/replay/src/index.ts", "--format", "json"],
    );
    assert_schema("schemas/delete.schema.json", &delete_map);
    assert!(
        delete_map["package_exports"]
            .as_array()
            .expect("package exports")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/package.json"
                && edge["to"] == "packages/replay/src/index.ts"
                && edge["type"] == "package_export"),
        "delete lens must show package manifest exports as deletion blockers: {delete_map:#}"
    );
    assert!(
        delete_map["checklist"]
            .as_array()
            .expect("checklist")
            .iter()
            .any(|item| item
                .as_str()
                .is_some_and(|text| text.contains("package public exports"))),
        "delete lens checklist should point at the manifest blocker without claiming safety: {delete_map:#}"
    );
}

#[test]
fn delete_lens_uses_runtime_fact_index_for_static_route_refs() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/server.ts"),
        "router.get('/delete/runtime', deleteRuntime);\nexport function deleteRuntime() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/tests/e2e/delete-runtime.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('delete runtime route', async ({ page }) => {\n  await page.goto('/delete/runtime');\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "delete route refs fixture"]);

    let delete_map = run_json(
        repo.path(),
        cache.path(),
        &["delete", "packages/app/src/server.ts", "--format", "json"],
    );
    assert_schema("schemas/delete.schema.json", &delete_map);
    assert!(
        delete_map["runtime_refs"]
            .as_array()
            .expect("runtime refs")
            .iter()
            .any(|edge| edge["from"] == "packages/app/tests/e2e/delete-runtime.spec.ts"
                && edge["to"] == "packages/app/src/server.ts"
                && edge["evidence"] == "e2e_visited_route"
                && edge["locations"][0]["kind"] == "route_visit"),
        "delete lens should use the same static route facts as runtime/proof-map, not only file-convention routes: {delete_map:#}"
    );
}

#[test]
fn delete_lens_does_not_choose_between_duplicate_runtime_route_refs() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/delete-a.ts"),
        "router.get('/delete/same', firstDelete);\nexport function firstDelete() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/src/delete-b.ts"),
        "router.get('/delete/same', secondDelete);\nexport function secondDelete() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/tests/e2e/delete-same.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('delete same route', async ({ page }) => {\n  await page.goto('/delete/same');\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "duplicate delete route refs fixture"]);

    let delete_map = run_json(
        repo.path(),
        cache.path(),
        &["delete", "packages/app/src/delete-a.ts", "--format", "json"],
    );
    assert_schema("schemas/delete.schema.json", &delete_map);
    assert!(
        delete_map["runtime_refs"]
            .as_array()
            .expect("runtime refs")
            .is_empty(),
        "delete lens must fail closed when a page visit has duplicate compatible route owners: {delete_map:#}"
    );
    assert!(
        delete_map["checklist"]
            .as_array()
            .expect("checklist")
            .iter()
            .all(|item| !item
                .as_str()
                .unwrap_or_default()
                .contains("runtime references")),
        "delete lens checklist should not imply a concrete runtime ref when ownership is ambiguous: {delete_map:#}"
    );
}
