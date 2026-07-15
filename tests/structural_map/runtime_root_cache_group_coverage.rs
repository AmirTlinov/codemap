// Responsibility: runtime-root-cache-group-semantic-repair
#[test]
fn runtime_root_cache_missing_non_route_horizon_misses_and_repairs() {
    assert_runtime_root_corruption_rebuilds(
        remove_entrypoint_horizon_and_certificate,
        true,
        "a coherent eight-group ledger must not pass the nine-group runtime contract",
    );
}

#[test]
fn runtime_root_cache_non_route_list_mismatch_misses_and_repairs() {
    assert_runtime_root_corruption_rebuilds(
        remove_cached_runtime_entrypoints,
        true,
        "a non-route list/horizon mismatch must fail semantic cache validation",
    );
}

#[test]
fn runtime_root_cache_zero_multiplicity_fact_misses_and_repairs() {
    assert_runtime_root_corruption_rebuilds(
        add_zero_multiplicity_entrypoint,
        true,
        "a rendered fact row cannot erase itself from certified visibility accounting",
    );
}

#[test]
fn runtime_root_cache_legacy_hidden_group_misses_and_repairs() {
    assert_runtime_root_corruption_rebuilds(
        add_legacy_runtime_hidden_group,
        true,
        "duplicate legacy visibility accounting must fail semantic cache validation",
    );
}

#[test]
fn runtime_root_cache_missing_recursive_hidden_boundary_misses_and_repairs() {
    let repo = TempDir::new().expect("inert nested root cache repo");
    let cache = TempDir::new().expect("inert nested root cache dir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("src/lib.ts"),
        "export const inert = true;\n",
    );
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &["commit", "-qm", "inert nested cache fixture"],
    );

    let expected_output = run_markdown(repo.path(), cache.path(), &["runtime", "."]);
    let expected_artifact = runtime_root_cache_json(cache.path());
    assert_eq!(
        expected_artifact["projection"]["recursive_hidden_count"], 1,
        "even a fact-inert nested file is hidden by the root projection"
    );
    let path = lens_artifact_path(cache.path(), "runtime-root.json");
    let corrupted = remove_recursive_hidden_boundary(
        fs::read_to_string(&path).expect("inert nested runtime cache"),
    );
    fs::write(&path, corrupted).expect("erase cached recursive boundary");

    let rebuilt = run_markdown(repo.path(), cache.path(), &["runtime", "."]);
    assert_lens_markdown_eq(
        &expected_output,
        &rebuilt,
        "a bounded root artifact cannot erase its committed recursive file boundary",
    );
    assert_eq!(
        runtime_root_cache_json(cache.path()),
        expected_artifact,
        "the forged full-looking report must be repaired to the canonical bounded projection"
    );
}

#[test]
fn runtime_root_cache_wrong_header_misses_and_repairs() {
    assert_runtime_root_corruption_rebuilds(
        corrupt_runtime_report_header,
        true,
        "the runtime cache must validate report kind, schema, and root scope",
    );
}

#[test]
fn runtime_root_cache_wrong_projection_misses_and_repairs() {
    assert_runtime_root_corruption_rebuilds(
        corrupt_runtime_projection,
        false,
        "a bounded current-level root cache must not accept another projection",
    );
}

#[test]
fn runtime_root_cache_nested_fact_in_current_level_projection_misses_and_repairs() {
    assert_runtime_root_corruption_rebuilds(
        reveal_nested_worker_in_cached_projection,
        true,
        "a full nested fact cannot be served under a current-level cache header",
    );
}

#[test]
fn runtime_root_cache_nested_entrypoint_in_current_level_projection_misses_and_repairs() {
    assert_runtime_root_corruption_rebuilds(
        reveal_nested_entrypoint_in_cached_projection,
        true,
        "a full nested entrypoint cannot be served under a current-level cache header",
    );
}

#[test]
fn runtime_root_cache_nested_script_in_root_catalog_misses_and_repairs() {
    assert_runtime_root_corruption_rebuilds(
        move_cached_root_script_to_nested_manifest,
        true,
        "a root-only script catalog cannot serve a nested manifest payload",
    );
}

#[test]
fn runtime_root_cache_wrong_snapshot_misses_and_repairs() {
    assert_runtime_root_corruption_rebuilds(
        corrupt_runtime_certificate_snapshot,
        false,
        "certificate snapshots must bind the cached report to its index fingerprint",
    );
}

#[test]
fn runtime_root_cache_wrong_schema_misses_and_repairs() {
    assert_runtime_root_corruption_rebuilds(
        corrupt_runtime_report_schema,
        false,
        "the runtime cache must reject a stale report schema independently",
    );
}

#[test]
fn runtime_root_cache_wrong_scope_misses_and_repairs() {
    assert_runtime_root_corruption_rebuilds(
        corrupt_runtime_report_scope,
        false,
        "the runtime root cache must reject a non-root report independently",
    );
}

fn remove_entrypoint_horizon_and_certificate(text: String) -> String {
    mutate_runtime_cache(text, |cache| {
        let horizons = cache["report"]["observations"]["horizons"]
            .as_array_mut()
            .expect("runtime horizons");
        let index = horizons
            .iter()
            .position(|horizon| horizon["group"] == "entrypoints")
            .expect("entrypoint horizon");
        let certificate_id = horizons[index]["count"]["certificate_id"]
            .as_str()
            .expect("entrypoint certificate")
            .to_string();
        horizons.remove(index);
        cache["report"]["observations"]["certificates"]
            .as_object_mut()
            .expect("runtime certificates")
            .remove(&certificate_id)
            .expect("entrypoint certificate body");
    })
}

fn remove_cached_runtime_entrypoints(text: String) -> String {
    mutate_runtime_cache(text, |cache| {
        let entrypoints = cache["report"]["entrypoints"]
            .as_array_mut()
            .expect("cached entrypoints");
        assert!(!entrypoints.is_empty(), "cache fixture needs an entrypoint");
        entrypoints.clear();
    })
}

fn add_zero_multiplicity_entrypoint(text: String) -> String {
    mutate_runtime_cache(text, |cache| {
        cache["report"]["entrypoints"]
            .as_array_mut()
            .expect("cached entrypoints")
            .push(serde_json::json!({
                "id": "surface:forged-zero",
                "kind": "runtime_container",
                "path": "FORGED_ZERO_FACT",
                "role": "runtime_entrypoint",
                "evidence": "forged_zero_multiplicity",
                "strength": "high",
                "count": 0,
                "examples": [],
                "hidden_count": 0
            }));
    })
}

fn add_legacy_runtime_hidden_group(text: String) -> String {
    mutate_runtime_cache(text, |cache| {
        cache["report"]["hidden"]
            .as_array_mut()
            .expect("runtime hidden groups")
            .push(serde_json::json!({
                "reason": "runtime entrypoints hidden by limit",
                "count": 1,
                "expand": "codemap runtime . --all --limit 1"
            }));
    })
}

fn remove_recursive_hidden_boundary(text: String) -> String {
    mutate_runtime_cache(text, |cache| {
        assert!(
            cache["projection"]["recursive_hidden_count"]
                .as_u64()
                .is_some_and(|count| count > 0),
            "cache fixture needs inert nested files"
        );
        let hidden = cache["report"]["hidden"]
            .as_array_mut()
            .expect("runtime hidden groups");
        assert_eq!(hidden.len(), 1, "fixture needs one canonical boundary");
        hidden.clear();
    })
}

fn corrupt_runtime_report_header(text: String) -> String {
    mutate_runtime_cache(text, |cache| {
        cache["report"]["kind"] = serde_json::json!("forged_runtime_report");
        cache["report"]["schema_version"] = serde_json::json!("4");
        cache["report"]["scope"] = serde_json::json!("src");
    })
}

fn corrupt_runtime_projection(text: String) -> String {
    mutate_runtime_cache(text, |cache| {
        cache["projection"]["kind"] = serde_json::json!("full");
        cache["projection"]["limit"] = serde_json::json!(999);
        cache["projection"]["current_level_root"] = serde_json::json!(false);
    })
}

fn reveal_nested_worker_in_cached_projection(text: String) -> String {
    mutate_runtime_cache(text, |cache| {
        let horizons = cache["report"]["observations"]["horizons"]
            .as_array_mut()
            .expect("runtime horizons");
        let horizon = horizons
            .iter_mut()
            .find(|horizon| horizon["group"] == "workers")
            .expect("worker horizon");
        let observed = horizon["count"]["observed"]
            .as_u64()
            .expect("worker observed count");
        let shown = horizon["shown"].as_u64().expect("worker shown count");
        assert!(shown < observed, "cache fixture needs a hidden worker fact");
        let next_shown = shown + 1;
        horizon["shown"] = serde_json::json!(next_shown);
        horizon["hidden"] = serde_json::json!(observed - next_shown);
        if next_shown == observed {
            horizon["expand"] = Value::Null;
        }
        cache["report"]["workers"]
            .as_array_mut()
            .expect("cached workers")
            .push(serde_json::json!({
                "id": "surface:worker_or_job:src/repo/runtime_paths.rs",
                "kind": "worker_or_job",
                "path": "src/repo/runtime_paths.rs",
                "role": null,
                "evidence": "worker_job_path_convention",
                "strength": "medium",
                "count": null,
                "examples": [],
                "hidden_count": 0
            }));
    })
}

fn reveal_nested_entrypoint_in_cached_projection(text: String) -> String {
    mutate_runtime_cache(text, |cache| {
        let entrypoints = cache["report"]["entrypoints"]
            .as_array_mut()
            .expect("cached entrypoints");
        assert!(!entrypoints.is_empty(), "cache fixture needs an entrypoint");
        entrypoints[0] = serde_json::json!({
            "id": "surface:cli_entrypoint:packages/app/package.json:app",
            "kind": "cli_entrypoint",
            "path": "packages/app/src/cli.js",
            "role": "runtime_entrypoint",
            "evidence": "package_json_bin",
            "strength": "hard",
            "count": 1,
            "examples": ["app -> packages/app/src/cli.js"],
            "hidden_count": 0
        });
    })
}

fn move_cached_root_script_to_nested_manifest(text: String) -> String {
    mutate_runtime_cache(text, |cache| {
        let scripts = cache["report"]["scripts"]
            .as_array_mut()
            .expect("cached scripts");
        assert!(!scripts.is_empty(), "cache fixture needs a root script");
        scripts[0]["path"] = serde_json::json!("packages/evil/package.json");
        scripts[0]["examples"] = serde_json::json!(["test: forged-command"]);
    })
}

fn corrupt_runtime_certificate_snapshot(text: String) -> String {
    mutate_runtime_cache(text, |cache| {
        let certificates = cache["report"]["observations"]["certificates"]
            .as_object_mut()
            .expect("runtime certificates");
        let certificate = certificates
            .values_mut()
            .next()
            .expect("runtime certificate");
        certificate["snapshot"] = serde_json::json!("forged-index-snapshot");
    })
}

fn corrupt_runtime_report_schema(text: String) -> String {
    mutate_runtime_cache(text, |cache| {
        cache["report"]["schema_version"] = serde_json::json!("4");
    })
}

fn corrupt_runtime_report_scope(text: String) -> String {
    mutate_runtime_cache(text, |cache| {
        cache["report"]["scope"] = serde_json::json!("src");
    })
}

fn mutate_runtime_cache(text: String, mutate: impl FnOnce(&mut Value)) -> String {
    let mut cache: Value = serde_json::from_str(&text).expect("runtime cache json");
    mutate(&mut cache);
    let body = format!(
        "{}\n",
        serde_json::to_string_pretty(&cache).expect("mutated runtime cache")
    );
    with_current_runtime_report_hash(body)
}
