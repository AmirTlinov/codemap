#[test]
fn partial_rescan_repairs_missing_cached_inventory_file_without_full_scan() {
    let (repo, cache) = fixture();
    let rel = "packages/app/src/useReplay.ts";

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    remove_cached_inventory_file(cache.path(), rel);

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(doctor["cache_strategy"], "partial_rescan");
    assert!(
        doctor["files_reused"].as_u64().unwrap_or_default() > 0,
        "unchanged cached facts should still be reused when one inventory entry is missing: {doctor:#}"
    );
    assert_eq!(
        doctor["scanner"]["files_scanned"], 1,
        "missing inventory facts should rescan only the mismatched file, not the repo: {doctor:#}"
    );

    let ls = run_json(repo.path(), cache.path(), &["ls", rel, "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &ls);
    assert_eq!(ls["mode"], "file");
    assert!(
        ls["anchor"]["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .any(|symbol| symbol["name"] == "frame"),
        "repaired inventory should restore file facts: {ls:#}"
    );
}

#[test]
fn partial_rescan_repairs_stale_cached_inventory_file_without_full_scan() {
    let (repo, cache) = fixture();
    let rel = "packages/app/src/useReplay.ts";

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    corrupt_cached_inventory_file(cache.path(), rel);

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(doctor["cache_strategy"], "partial_rescan");
    assert!(
        doctor["files_reused"].as_u64().unwrap_or_default() > 0,
        "unchanged matching facts should still be reused around stale inventory: {doctor:#}"
    );
    assert_eq!(
        doctor["scanner"]["files_scanned"], 1,
        "stale cached facts should rescan only the mismatched file, not the repo: {doctor:#}"
    );

    let ls = run_json(repo.path(), cache.path(), &["ls", rel, "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &ls);
    assert!(
        ls["anchor"]["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .any(|symbol| symbol["name"] == "frame"),
        "partial rescan should restore current file facts: {ls:#}"
    );
}

fn remove_cached_inventory_file(cache_root: &Path, rel: &str) {
    edit_cached_inventory(cache_root, |inventory| {
        let files = inventory["files"]
            .as_array_mut()
            .expect("inventory files array");
        let before = files.len();
        files.retain(|file| file["path"] != rel);
        assert_eq!(files.len() + 1, before, "fixture should remove one file");
    });
}

fn corrupt_cached_inventory_file(cache_root: &Path, rel: &str) {
    edit_cached_inventory(cache_root, |inventory| {
        let files = inventory["files"].as_array_mut().expect("inventory files");
        let file = files
            .iter_mut()
            .find(|file| file["path"] == rel)
            .expect("fixture file");
        file["content_hash"] = Value::String("stale-content-hash".to_string());
        file["symbols"] = Value::Array(Vec::new());
    });
}

fn edit_cached_inventory(cache_root: &Path, edit: impl FnOnce(&mut Value)) {
    let inventory_path = fs::read_dir(cache_root)
        .expect("cache dir")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path().join("inventory.json"))
        .find(|path| path.exists())
        .expect("inventory json path");
    let mut inventory: Value =
        serde_json::from_str(&fs::read_to_string(&inventory_path).expect("inventory json"))
            .expect("inventory value");
    edit(&mut inventory);
    fs::write(
        &inventory_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&inventory).expect("serialize inventory")
        ),
    )
    .expect("write edited inventory");
}
