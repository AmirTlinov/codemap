#[test]
fn source_role_classifiers_keep_doctor_unclassified_noise_low() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("src/app/orders.service.ts"),
        "export function listOrders() { return []; }\n",
    );
    write(
        &repo.path().join("src/app/orders.controller.ts"),
        "export function routeOrders() { return Response.json({ ok: true }); }\n",
    );
    write(
        &repo.path().join("src/domain/order.ts"),
        "export type Order = { id: string };\n",
    );
    write(
        &repo.path().join("src/modules/billing.module.ts"),
        "export const billingModule = true;\n",
    );
    write(
        &repo.path().join("src/repositories/order_repository.ts"),
        "export const orderRepository = {};\n",
    );
    write(
        &repo.path().join("src/map/lenses/diff_map.rs"),
        "pub fn diff_map() {}\n",
    );
    write(
        &repo.path().join("src/repo/surfaces_core.rs"),
        "pub fn extract_surfaces() {}\n",
    );
    write(
        &repo.path().join("src/repo/js_imports.rs"),
        "pub fn scan_js_imports() {}\n",
    );
    write(
        &repo.path().join("src/repo/scripts_make.rs"),
        "pub fn make_targets() {}\n",
    );
    write(
        &repo.path().join("src/cli/args.rs"),
        "pub fn parse_args() {}\n",
    );
    write(
        &repo.path().join("src/repo/project.rs"),
        "pub struct Project { pub root: String }\n",
    );
    write(
        &repo.path().join("src/repo/tests.rs"),
        "#[cfg(test)]\nmod tests {}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "source role fixture"]);

    for (path, expected_role) in [
        ("src/app/orders.service.ts", "service"),
        ("src/app/orders.controller.ts", "controller"),
        ("src/domain/order.ts", "domain"),
        ("src/modules/billing.module.ts", "module"),
        ("src/repositories/order_repository.ts", "repository"),
        ("src/map/lenses/diff_map.rs", "map_surface"),
        ("src/repo/surfaces_core.rs", "extractor"),
        ("src/repo/js_imports.rs", "extractor"),
        ("src/repo/scripts_make.rs", "script_catalog"),
        ("src/cli/args.rs", "cli_surface"),
        ("src/repo/project.rs", "state_model"),
        ("src/repo/tests.rs", "test_support"),
    ] {
        let ls = run_json(repo.path(), cache.path(), &["ls", path, "--format", "json"]);
        assert_schema("schemas/ls.schema.json", &ls);
        assert!(
            ls["anchor"]["roles"]
                .as_array()
                .expect("roles")
                .iter()
                .any(|role| role == expected_role),
            "{path} should carry deterministic role `{expected_role}`: {ls:#}"
        );
    }

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(
        doctor["unclassified_count"], 0,
        "fixture source files should not show up as unclassified doctor noise: {doctor:#}"
    );

    let changed = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args([
            "changed",
            "--files",
            "src/app/orders.service.ts,src/map/lenses/diff_map.rs,src/cli/args.rs",
            "--section",
            "roles",
        ])
        .output()
        .expect("changed roles should run");
    assert!(
        changed.status.success(),
        "changed roles failed: {}",
        String::from_utf8_lossy(&changed.stderr)
    );
    let markdown = String::from_utf8(changed.stdout).expect("markdown utf8");
    for role in ["`service`", "`map_surface`", "`cli_surface`"] {
        assert!(
            markdown.contains(role),
            "changed roles should use the same source role catalog as scanner for {role}: {markdown}"
        );
    }
}

#[test]
fn source_file_extensions_do_not_become_extractor_roles() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(&repo.path().join("src/plain.js"), "export const plain = true;\n");
    write(
        &repo.path().join("src/plain.jsx"),
        "export const alsoPlain = true;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "plain js sources"]);

    for path in ["src/plain.js", "src/plain.jsx"] {
        let ls = run_json(repo.path(), cache.path(), &["ls", path, "--format", "json"]);
        assert_schema("schemas/ls.schema.json", &ls);
        if path.ends_with(".js") {
            assert_eq!(
                ls["anchor"]["kind"], "source",
                "{path} should stay source: {ls:#}"
            );
        }
        assert!(
            !ls["anchor"]["roles"]
                .as_array()
                .expect("roles")
                .iter()
                .any(|role| role == "extractor"),
            "file extension `{path}` must not count as extractor evidence: {ls:#}"
        );
    }

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(
        doctor["unclassified_count"], 1,
        "plain .js should remain honest unclassified source, not be hidden by extension tokens: {doctor:#}"
    );
}
