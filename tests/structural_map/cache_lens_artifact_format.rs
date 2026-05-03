#[test]
fn stale_lens_artifact_format_is_not_served_and_is_visible_in_doctor() {
    let (repo, cache) = fixture();
    let rel = "packages/app/src/useReplay.ts";

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    let _ = run_json(repo.path(), cache.path(), &["cone", rel, "--format", "json"]);
    poison_format_lens_report_field(cache.path(), "cone-current.json", "kind", "__stale_cone__");
    poison_format_lens_top_level_field(cache.path(), "cone-current.json", "format_version", 1);

    let cone = run_json(repo.path(), cache.path(), &["cone", rel, "--format", "json"]);
    assert_eq!(
        cone["kind"], "cone_report",
        "stale lens artifact format must miss the fast path instead of serving stale output: {cone:#}"
    );

    poison_format_lens_top_level_field(cache.path(), "cone-current.json", "format_version", 1);
    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert!(
        doctor["map_quality"]
            .as_array()
            .expect("map_quality")
            .iter()
            .any(stale_lens_warning_mentions_cone),
        "doctor should expose stale lens artifacts as map-quality diagnostics: {doctor:#}"
    );
}

#[test]
fn lens_artifact_fingerprint_mismatch_is_normal_invalidation_not_doctor_noise() {
    let (repo, cache) = fixture();
    let rel = "packages/app/src/useReplay.ts";

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    write(
        &repo.path().join(rel),
        "import { seek } from '@fixture/replay';\n\nexport const changedFrame = seek(71).frame;\n",
    );

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert!(
        doctor["map_quality"]
            .as_array()
            .expect("map_quality")
            .iter()
            .all(|warning| warning["kind"] != "stale_lens_artifact"),
        "ordinary lens artifact fingerprint mismatch should be ignored, not surfaced as quality noise: {doctor:#}"
    );
}

fn stale_lens_warning_mentions_cone(warning: &Value) -> bool {
    warning["kind"] == "stale_lens_artifact"
        && warning["examples"]
            .as_array()
            .expect("examples")
            .iter()
            .any(|example| {
                example
                    .as_str()
                    .is_some_and(|text| text.contains("cone-current.json"))
            })
}

fn poison_format_lens_report_field(cache_root: &Path, name: &str, field: &str, value: &str) {
    let path = format_lens_artifact_path(cache_root, name);
    let text = fs::read_to_string(&path).expect("lens artifact should be readable");
    let mut json: Value = serde_json::from_str(&text).expect("lens artifact json");
    json["report"][field] = Value::String(value.to_string());
    fs::write(
        path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&json).expect("lens artifact json")
        ),
    )
    .expect("poison lens artifact");
}

fn poison_format_lens_top_level_field(cache_root: &Path, name: &str, field: &str, value: u64) {
    let path = format_lens_artifact_path(cache_root, name);
    let text = fs::read_to_string(&path).expect("lens artifact should be readable");
    let mut json: Value = serde_json::from_str(&text).expect("lens artifact json");
    json[field] = Value::Number(value.into());
    fs::write(
        path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&json).expect("lens artifact json")
        ),
    )
    .expect("poison lens artifact top-level field");
}

fn format_lens_artifact_path(cache_root: &Path, name: &str) -> std::path::PathBuf {
    fs::read_dir(cache_root)
        .expect("cache dir")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path().join(name))
        .find(|path| path.exists())
        .unwrap_or_else(|| panic!("{name} should exist under {}", cache_root.display()))
}
