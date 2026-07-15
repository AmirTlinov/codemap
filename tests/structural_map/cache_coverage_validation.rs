// Responsibility: corrupted-coverage-cache-behavior
type CoverageCacheMutation = fn(&mut Value);
type ConeReportCacheMutation = fn(&mut Value);

#[test]
fn corrupted_certificate_registry_misses_and_rebuilds_cone_cache() {
    assert_coverage_cache_corruptions_rebuild(&[
        ("dangling certificate", corrupt_dangling_certificate),
        ("certificate key/id mismatch", corrupt_certificate_key),
        ("non-canonical certificate hash", corrupt_certificate_body),
    ]);
}

#[test]
fn corrupted_horizon_semantics_miss_and_rebuild_cone_cache() {
    assert_coverage_cache_corruptions_rebuild(&[
        ("scope mismatch", corrupt_horizon_scope),
        ("closure mismatch", corrupt_horizon_closure),
        ("reasons mismatch", corrupt_horizon_reasons),
        ("shown/hidden mismatch", corrupt_horizon_visibility),
        ("unsupported mismatch", corrupt_horizon_unsupported),
        ("dynamic mismatch", corrupt_horizon_dynamic),
        ("duplicate horizon", corrupt_duplicate_horizon),
        ("expand without hidden", corrupt_horizon_expand),
    ]);
}

#[test]
fn corrupted_cone_fact_list_misses_instead_of_serving_a_false_horizon() {
    let (repo, cache) = fixture();
    let args = [
        "cone",
        "packages/replay/src/session.ts#seek",
        "--format",
        "json",
    ];
    let expected = run_json(repo.path(), cache.path(), &args);
    assert!(
        !expected["incoming"]
            .as_array()
            .expect("incoming facts")
            .is_empty(),
        "fixture must exercise count-to-list alignment"
    );
    let path = lens_artifact_path(cache.path(), "cone-current.json");
    let mut artifact = cached_lens_artifact_json(cache.path(), "cone-current.json");
    artifact["report"]["incoming"] = Value::Array(Vec::new());
    fs::write(
        &path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&artifact).expect("artifact json")
        ),
    )
    .expect("write corrupted cone facts");

    let rebuilt = run_json(repo.path(), cache.path(), &args);
    assert_eq!(rebuilt["incoming"], expected["incoming"]);
    let repaired = cached_lens_artifact_json(cache.path(), "cone-current.json");
    assert_eq!(repaired["report"]["incoming"], expected["incoming"]);
}

#[test]
fn coherent_count_and_fact_identity_corruption_miss_the_cone_cache() {
    assert_cone_report_corruptions_rebuild(&[
        ("paired observed/hidden mutation", corrupt_coherent_observed_count),
        ("forged edge identity", corrupt_incoming_edge_identity),
    ]);
}

fn assert_cone_report_corruptions_rebuild(
    corruptions: &[(&str, ConeReportCacheMutation)],
) {
    let (repo, cache) = fixture();
    let args = [
        "cone",
        "packages/replay/src/session.ts#seek",
        "--format",
        "json",
    ];
    let expected = run_json(repo.path(), cache.path(), &args);
    for (name, corrupt) in corruptions {
        let path = lens_artifact_path(cache.path(), "cone-current.json");
        let mut artifact = cached_lens_artifact_json(cache.path(), "cone-current.json");
        corrupt(&mut artifact["report"]);
        fs::write(
            &path,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&artifact).expect("artifact json")
            ),
        )
        .expect("write corrupted cone report");

        let rebuilt = run_json(repo.path(), cache.path(), &args);
        assert_eq!(rebuilt, expected, "{name} must miss instead of being served");
        let repaired = cached_lens_artifact_json(cache.path(), "cone-current.json");
        assert_eq!(repaired["report"]["incoming"], expected["incoming"]);
    }
}

fn assert_coverage_cache_corruptions_rebuild(
    corruptions: &[(&str, CoverageCacheMutation)],
) {
    let (repo, cache) = fixture();
    let args = [
        "cone",
        "packages/replay/src/session.ts#seek",
        "--format",
        "json",
    ];
    let expected = run_json(repo.path(), cache.path(), &args);

    for (name, corrupt) in corruptions {
        let path = lens_artifact_path(cache.path(), "cone-current.json");
        let mut artifact = cached_lens_artifact_json(cache.path(), "cone-current.json");
        corrupt(&mut artifact["report"]["observations"]);
        assert_ne!(
            artifact["report"]["observations"], expected["observations"],
            "{name} mutation must change the cached ledger"
        );
        fs::write(
            &path,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&artifact).expect("artifact json")
            ),
        )
        .expect("write corrupted cone artifact");

        let rebuilt = run_json(repo.path(), cache.path(), &args);
        assert_eq!(
            rebuilt["observations"], expected["observations"],
            "{name} must miss instead of being served"
        );
        let repaired = cached_lens_artifact_json(cache.path(), "cone-current.json");
        assert_eq!(
            repaired["report"]["observations"], expected["observations"],
            "{name} miss must rewrite a valid artifact"
        );
    }
}

fn corrupt_dangling_certificate(ledger: &mut Value) {
    ledger["certificates"] = Value::Object(serde_json::Map::new());
}

fn corrupt_certificate_key(ledger: &mut Value) {
    let certificates = ledger["certificates"]
        .as_object_mut()
        .expect("certificate registry");
    let key = certificates.keys().next().expect("certificate").clone();
    let certificate = certificates.remove(&key).expect("certificate value");
    certificates.insert(format!("corrupt-{key}"), certificate);
}

fn corrupt_certificate_body(ledger: &mut Value) {
    first_certificate_mut(ledger)["snapshot"] = Value::String("corrupt-snapshot".to_string());
}

fn corrupt_horizon_scope(ledger: &mut Value) {
    first_horizon_mut(ledger)["scope"] = Value::String("corrupt-scope".to_string());
}

fn corrupt_horizon_closure(ledger: &mut Value) {
    let closure = &mut first_horizon_mut(ledger)["count"]["closure"];
    *closure = Value::String(if closure == "closed" { "open" } else { "closed" }.to_string());
}

fn corrupt_horizon_reasons(ledger: &mut Value) {
    first_horizon_mut(ledger)["count"]["reasons"]
        .as_array_mut()
        .expect("horizon reasons")
        .push(Value::String("anchor_not_indexed".to_string()));
}

fn corrupt_horizon_visibility(ledger: &mut Value) {
    let hidden = &mut first_horizon_mut(ledger)["hidden"];
    let value = hidden.as_u64().expect("hidden count") + 1;
    *hidden = Value::Number(value.into());
}

fn corrupt_horizon_unsupported(ledger: &mut Value) {
    first_horizon_mut(ledger)["unsupported"]
        .as_array_mut()
        .expect("unsupported observations")
        .push(serde_json::json!({
            "file": "src/corrupt.ts",
            "construct": "corrupt construct"
        }));
}

fn corrupt_horizon_dynamic(ledger: &mut Value) {
    first_horizon_mut(ledger)["dynamic"]
        .as_array_mut()
        .expect("dynamic stops")
        .push(serde_json::json!({"kind": "dynamic_import_flow"}));
}

fn corrupt_duplicate_horizon(ledger: &mut Value) {
    let duplicate = ledger["horizons"][0].clone();
    ledger["horizons"]
        .as_array_mut()
        .expect("horizons")
        .push(duplicate);
}

fn corrupt_horizon_expand(ledger: &mut Value) {
    first_horizon_mut(ledger)["expand"] = Value::String("codemap cone bogus --all".to_string());
}

fn corrupt_coherent_observed_count(report: &mut Value) {
    let incoming = report["observations"]["horizons"]
        .as_array_mut()
        .expect("horizons")
        .iter_mut()
        .find(|horizon| horizon["group"] == "incoming")
        .expect("incoming horizon");
    let observed = incoming["count"]["observed"]
        .as_u64()
        .expect("observed")
        + 100;
    let hidden = incoming["hidden"].as_u64().expect("hidden") + 100;
    incoming["count"]["observed"] = Value::Number(observed.into());
    incoming["hidden"] = Value::Number(hidden.into());
}

fn corrupt_incoming_edge_identity(report: &mut Value) {
    let edge = report["incoming"]
        .as_array_mut()
        .expect("incoming")
        .first_mut()
        .expect("incoming edge");
    edge["from"] = Value::String("src/FORGED-CONSUMER.ts".to_string());
    if let Some(location) = edge["locations"].as_array_mut().and_then(|items| items.first_mut()) {
        location["path"] = Value::String("src/FORGED-CONSUMER.ts".to_string());
    }
}

fn first_horizon_mut(ledger: &mut Value) -> &mut Value {
    ledger["horizons"]
        .as_array_mut()
        .expect("horizons")
        .first_mut()
        .expect("horizon")
}

fn first_certificate_mut(ledger: &mut Value) -> &mut Value {
    let id = ledger["horizons"][0]["count"]["certificate_id"]
        .as_str()
        .expect("certificate id")
        .to_string();
    ledger["certificates"]
        .get_mut(&id)
        .expect("referenced certificate")
}
