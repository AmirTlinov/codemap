#[test]
fn proof_map_changed_header_and_raw_expands_stay_bounded() {
    let (repo, cache) = fixture();
    for index in 0..8 {
        write(
            &repo
                .path()
                .join(format!("packages/replay/src/mass-{index}.ts")),
            &format!("export const mass{index} = {index};\n"),
        );
    }

    let markdown = run_markdown(
        repo.path(),
        cache.path(),
        &["proof-map", "--changed", "--limit", "1"],
    );
    let max_line = markdown.lines().map(str::len).max().unwrap_or_default();
    assert!(
        max_line <= 420,
        "proof-map changed header should stay bounded instead of listing every path: {markdown}"
    );
    let changed_line = markdown
        .lines()
        .find(|line| line.starts_with("Changed:"))
        .expect("changed summary line");
    assert!(
        changed_line.contains("sample:") && changed_line.contains("hidden:"),
        "proof-map should summarize changed anchors with sample/hidden counts: {markdown}"
    );

    let raw = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof-map",
            "--changed",
            "--raw-sensors",
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof-map.schema.json", &raw);
    assert_eq!(raw["selector"], "--changed");
    assert!(
        raw["hidden"].as_array().expect("hidden").iter().all(|group| group["expand"]
            .as_str()
            .is_some_and(|expand| expand
                .starts_with("codemap proof-map --changed --raw-sensors --limit "))),
        "raw-sensors hidden expands must keep the raw-sensors phase: {raw:#}"
    );
}

#[test]
fn proof_map_schema_unknowns_match_exact_proof_unknowns() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("schemas/proof-map.schema.json"),
        r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object"}"#,
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "schemas/proof-map.schema.json", "--format", "json"],
    );
    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof-map",
            "schemas/proof-map.schema.json",
            "--raw-sensors",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert_schema("schemas/proof-map.schema.json", &proof_map);

    let proof_kinds = sorted_unknown_kinds(&proof);
    let proof_map_kinds = sorted_unknown_kinds(&proof_map);
    for kind in [
        "schema_check_not_found",
        "schema_client_consumer_not_found",
        "schema_env_link_not_found",
        "schema_migration_not_found",
    ] {
        assert!(
            proof_kinds.iter().any(|value| value == kind)
                && proof_map_kinds.iter().any(|value| value == kind),
            "schema unknown `{kind}` should be visible in proof and proof-map: proof={proof:#} proof_map={proof_map:#}"
        );
    }
}

#[test]
fn changed_writes_reusable_compact_proof_map_cache() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/cache-hop.ts"),
        "export const cacheHop = true;\n",
    );

    let _ = run_lens_stdout(repo.path(), cache.path(), &["changed", "--limit", "1"]);
    let proof_map = run_lens_stdout(
        repo.path(),
        cache.path(),
        &["proof-map", "--changed", "--limit", "1"],
    );

    assert!(
        proof_map.contains("strategy=`cached_lens`"),
        "changed should pass its compact proof-map material into the next proof-map contact: {proof_map}"
    );
}

#[test]
fn changed_proof_section_writes_reusable_compact_proof_map_cache() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/cache-proof-section.ts"),
        "export const cacheProofSection = true;\n",
    );

    let _ = run_lens_stdout(
        repo.path(),
        cache.path(),
        &["changed", "--section", "proof", "--limit", "1"],
    );
    let proof_map = run_lens_stdout(
        repo.path(),
        cache.path(),
        &["proof-map", "--changed", "--limit", "1"],
    );

    assert!(
        proof_map.contains("strategy=`cached_lens`"),
        "changed --section proof should pass its compact proof-map material forward: {proof_map}"
    );
}

#[test]
fn changed_roles_section_does_not_birth_proof_map_cache() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/cache-roles-section.ts"),
        "export const cacheRolesSection = true;\n",
    );

    let _ = run_lens_stdout(
        repo.path(),
        cache.path(),
        &["changed", "--section", "roles", "--limit", "1"],
    );

    assert_eq!(
        proof_map_keyed_artifact_count(cache.path()),
        0,
        "changed --section roles should not materialize proof-map cache mass"
    );
}

#[test]
fn proof_map_cache_keeps_compact_and_raw_phases_separate() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/cache-phase.ts"),
        "export const cachePhase = true;\n",
    );

    let compact_first = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof-map",
            "--changed",
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );
    let raw = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof-map",
            "--changed",
            "--raw-sensors",
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );
    let compact_second = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof-map",
            "--changed",
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );

    assert_schema("schemas/proof-map.schema.json", &compact_first);
    assert_schema("schemas/proof-map.schema.json", &raw);
    assert_schema("schemas/proof-map.schema.json", &compact_second);
    assert_eq!(
        compact_first, compact_second,
        "raw-sensors proof-map cache must not erase the compact proof-map phase"
    );
    assert!(
        proof_map_keyed_artifact_count(cache.path()) >= 2,
        "compact and raw proof-map phases should each leave a keyed cache artifact"
    );
}

#[test]
fn changed_section_bypasses_full_changed_cache_mass() {
    let (repo, cache) = fixture();
    write_root_manifest_lock_pair_change(repo.path());

    let cold = run_lens_stdout(
        repo.path(),
        cache.path(),
        &["changed", "--section", "observed", "--limit", "1"],
    );
    let _ = run_lens_stdout(repo.path(), cache.path(), &["changed", "--limit", "1"]);
    let after_full = run_lens_stdout(
        repo.path(),
        cache.path(),
        &["changed", "--section", "observed", "--limit", "1"],
    );

    for markdown in [&cold, &after_full] {
        assert!(
            markdown.contains("- `package.json` [manifest")
                && !markdown.contains("- `pnpm-lock.yaml` ["),
            "section observed should keep its own bounded seed instead of replaying full changed cache: {markdown}"
        );
        assert!(
            !markdown.contains("strategy=`cached_lens`"),
            "section changed should not be served from full changed-current cache: {markdown}"
        );
    }
}

#[test]
fn changed_json_section_observed_does_not_birth_proof_map_mass() {
    let (repo, cache) = fixture();
    write_root_manifest_lock_pair_change(repo.path());

    let json = run_json(
        repo.path(),
        cache.path(),
        &[
            "changed",
            "--section",
            "observed",
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );

    assert_schema("schemas/changed.schema.json", &json);
    assert_eq!(json["changed"].as_array().expect("changed").len(), 1);
    assert!(json["impact"].as_array().expect("impact").is_empty());
    assert!(json["proof"]["hard"].as_array().expect("proof hard").is_empty());
    assert!(
        !cached_lens_artifact_exists(cache.path(), "changed-current.json"),
        "section JSON should not write the full changed lens cache"
    );
    assert_eq!(
        proof_map_keyed_artifact_count(cache.path()),
        0,
        "observed JSON section should not materialize proof-map cache mass"
    );
}

#[test]
fn compact_changed_proof_map_prioritizes_proof_bearing_seed() {
    let (repo, cache) = fixture();
    write_root_manifest_lock_pair_change(repo.path());

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof-map",
            "--changed",
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );

    assert_schema("schemas/proof-map.schema.json", &proof_map);
    assert!(
        proof_map["hard"]
            .as_array()
            .expect("hard")
            .iter()
            .any(|surface| surface["path"] == "package.json"),
        "compact proof-map should spend its first changed seed on proof-bearing manifest material, not lockfile order: {proof_map:#}"
    );
}

#[test]
fn changed_sections_keep_pair_context_and_hidden_horizon() {
    let (repo, cache) = fixture();
    write_root_manifest_lock_pair_change(repo.path());

    let links = run_lens_stdout(
        repo.path(),
        cache.path(),
        &["changed", "--section", "links", "--limit", "1"],
    );
    assert!(
        links.contains("`lockfile_manifest_pair` [yes]"),
        "bounded links section should read manifest/lockfile context from the full selected changed set: {links}"
    );

    let observed = run_lens_stdout(
        repo.path(),
        cache.path(),
        &["changed", "--section", "observed", "--limit", "1"],
    );
    assert!(
        !observed.contains("manifest_without_lockfile_change"),
        "bounded observed section must not report a missing paired lockfile when it is hidden only by limit: {observed}"
    );

    let proof = run_lens_stdout(
        repo.path(),
        cache.path(),
        &["changed", "--section", "proof", "--limit", "1"],
    );
    assert!(
        proof.contains("\n## Hidden\n")
            && proof.contains("changed proof seeds hidden by compact limit"),
        "section proof should carry the hidden horizon beside the section output: {proof}"
    );
}

fn sorted_unknown_kinds(report: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(unknowns) = report["unknowns"].as_array() {
        out.extend(
            unknowns
                .iter()
                .filter_map(|unknown| unknown["kind"].as_str().map(str::to_string)),
        );
    }
    out.sort();
    out.dedup();
    out
}

fn write_root_manifest_lock_pair_change(repo: &Path) {
    write(repo.join("pnpm-lock.yaml").as_path(), "lockfileVersion: '9.0'\n");
    git(repo, &["add", "pnpm-lock.yaml"]);
    git(repo, &["commit", "-qm", "lock baseline"]);
    write(
        repo.join("package.json").as_path(),
        r#"{"name":"map-fixture","private":true,"workspaces":["packages/*"],"scripts":{"test":"pnpm test","typecheck":"tsc -b","verify":"node scripts/verify.js"}}"#,
    );
    write(
        repo.join("pnpm-lock.yaml").as_path(),
        "lockfileVersion: '9.0'\npackages:\n  /map-fixture: {}\n",
    );
}

fn proof_map_keyed_artifact_count(cache_root: &Path) -> usize {
    std::fs::read_dir(cache_root)
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .flat_map(|entry| {
            std::fs::read_dir(entry.path())
                .into_iter()
                .flat_map(|entries| entries.filter_map(Result::ok))
                .collect::<Vec<_>>()
        })
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with("proof-map-") && name.ends_with(".json"))
        })
        .count()
}
