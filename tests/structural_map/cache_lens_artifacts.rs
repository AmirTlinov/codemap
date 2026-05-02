#[test]
fn dirty_daily_lens_artifacts_roundtrip_without_output_drift() {
    let (repo, cache) = fixture();
    let rel = "packages/app/src/useReplay.ts";

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    write(
        &repo.path().join(rel),
        "import { seek } from '@fixture/replay';\n\nexport const changedFrame = seek(71).frame;\n",
    );

    let changed_first = run_lens_stdout(repo.path(), cache.path(), &["changed"]);
    let changed_second = run_lens_stdout(repo.path(), cache.path(), &["changed"]);
    assert_eq!(
        changed_first, changed_second,
        "cached changed artifact must preserve markdown output"
    );
    assert!(
        cached_lens_artifact_exists(cache.path(), "proof-changed.json"),
        "changed command should warm the proof changed artifact from already computed proof facts"
    );

    let direct_cache = TempDir::new().expect("direct proof cache");
    let proof_direct = run_lens_stdout(repo.path(), direct_cache.path(), &["proof", "changed"]);
    let proof_from_changed = run_lens_stdout(repo.path(), cache.path(), &["proof", "changed"]);
    assert_eq!(
        proof_direct, proof_from_changed,
        "proof changed warmed by changed must match a direct proof changed report"
    );
    let direct_json_cache = TempDir::new().expect("direct proof json cache");
    let proof_direct_json = run_json(
        repo.path(),
        direct_json_cache.path(),
        &["proof", "changed", "--format", "json"],
    );
    let proof_from_changed_json =
        run_json(repo.path(), cache.path(), &["proof", "changed", "--format", "json"]);
    assert_eq!(
        proof_direct_json, proof_from_changed_json,
        "proof changed warmed by changed must match direct proof changed JSON"
    );

    let sentinel = "__changed_warmed_proof_cache__";
    poison_lens_report_field(cache.path(), "proof-changed.json", "run_hint", sentinel);
    let proof_json = run_json(repo.path(), cache.path(), &["proof", "changed", "--format", "json"]);
    assert_eq!(
        proof_json["run_hint"], sentinel,
        "proof changed should read the artifact warmed by changed for the exact default key"
    );

    let proof_first = run_lens_stdout(repo.path(), cache.path(), &["proof", "changed"]);
    let proof_second = run_lens_stdout(repo.path(), cache.path(), &["proof", "changed"]);
    assert_eq!(
        proof_first, proof_second,
        "cached proof artifact must preserve markdown output"
    );
    let proof_changed_flag = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", "--changed"])
        .output()
        .expect("codemap should run");
    assert!(
        !proof_changed_flag.status.success(),
        "proof --changed should be a hard replacement error, not a cached alias"
    );
    let stderr = String::from_utf8_lossy(&proof_changed_flag.stderr);
    assert!(
        stderr.contains("`codemap proof --changed` was replaced by `codemap proof changed`"),
        "proof --changed should point agents at proof changed, stderr={stderr}"
    );

    assert!(
        cached_lens_artifact_exists(cache.path(), "changed-current.json"),
        "changed command should write an external lens artifact"
    );
    assert!(
        cached_lens_artifact_exists(cache.path(), "proof-changed.json"),
        "proof changed should write an external lens artifact"
    );
}

#[test]
fn changed_warmed_proof_cache_ignores_changed_display_limit() {
    let (repo, cache) = fixture();
    let direct_cache = TempDir::new().expect("direct proof cache");
    for rel in [
        "packages/replay/src/session.ts",
        "packages/replay/src/public-only.ts",
        "packages/replay/src/types.ts",
    ] {
        write(&repo.path().join(rel), &format!("// changed {rel}\n"));
    }

    let _ = run_lens_stdout(repo.path(), cache.path(), &["changed", "--limit", "1"]);
    let warmed = run_lens_stdout(repo.path(), cache.path(), &["proof", "changed"]);
    let direct = run_lens_stdout(repo.path(), direct_cache.path(), &["proof", "changed"]);
    assert_eq!(
        direct, warmed,
        "changed --limit must not truncate the warmed default proof changed artifact"
    );
}

#[test]
fn changed_warmed_proof_cache_preserves_direct_proof_order_and_hidden_limit() {
    let (repo, cache) = fixture();
    let direct_cache = TempDir::new().expect("direct proof cache");
    let rel = "packages/replay/src/session.ts";
    write(
        &repo.path().join(rel),
        "import { Timeline } from './timeline';\n\nexport function seek(cursor: number) {\n  return { frame: new Timeline().frameAt(cursor + 1) };\n}\n",
    );
    for index in 0..14 {
        write(
            &repo
                .path()
                .join(format!("packages/replay/tests/session-extra-{index}.test.ts")),
            "import { seek } from '../src/session';\n\ntest('extra seek proof', () => {\n  expect(seek(2).frame).toBeTruthy();\n});\n",
        );
    }

    let _ = run_lens_stdout(repo.path(), cache.path(), &["changed"]);
    let warmed = run_lens_stdout(repo.path(), cache.path(), &["proof", "changed"]);
    let direct = run_lens_stdout(repo.path(), direct_cache.path(), &["proof", "changed"]);
    assert_eq!(
        direct, warmed,
        "changed-warmed proof must preserve direct proof ordering and hidden truncation"
    );
}

#[test]
fn navigation_lens_artifacts_roundtrip_without_output_drift() {
    let (repo, cache) = fixture();
    let rel = "packages/app/src/useReplay.ts";

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);

    let ls_first = run_lens_stdout(repo.path(), cache.path(), &["ls", rel]);
    let ls_second = run_lens_stdout(repo.path(), cache.path(), &["ls", rel]);
    assert_eq!(
        ls_first, ls_second,
        "cached ls artifact must preserve markdown output"
    );

    let cone_first = run_lens_stdout(repo.path(), cache.path(), &["cone", rel]);
    let cone_second = run_lens_stdout(repo.path(), cache.path(), &["cone", rel]);
    assert_eq!(
        cone_first, cone_second,
        "cached cone artifact must preserve markdown output"
    );

    assert!(
        cached_lens_artifact_exists(cache.path(), "ls-current.json"),
        "ls command should write an external lens artifact"
    );
    assert!(
        cached_lens_artifact_exists(cache.path(), "cone-current.json"),
        "cone command should write an external lens artifact"
    );
}

#[test]
fn proof_map_lens_artifact_roundtrips_without_output_drift() {
    let (repo, cache) = fixture();

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);

    let proof_map_first = run_lens_stdout(repo.path(), cache.path(), &["proof-map", "."]);
    let proof_map_second = run_lens_stdout(repo.path(), cache.path(), &["proof-map", "."]);
    assert_eq!(
        proof_map_first, proof_map_second,
        "cached proof-map artifact must preserve markdown output"
    );

    assert!(
        cached_lens_artifact_exists(cache.path(), "proof-map-current.json"),
        "proof-map command should write an external lens artifact"
    );
}

#[test]
fn proof_map_staged_cache_rechecks_selector_changed_set() {
    let (repo, cache) = fixture();
    let rel = "packages/app/src/useReplay.ts";

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    write(
        &repo.path().join(rel),
        "import { seek } from '@fixture/replay';\n\nexport const stagedFrame = seek(9).frame;\n",
    );
    git(repo.path(), &["add", rel]);

    let staged_first = run_lens_stdout(
        repo.path(),
        cache.path(),
        &["proof-map", "--staged", "--format", "json"],
    );
    let staged_first: Value =
        serde_json::from_str(&staged_first).expect("first proof-map json");
    assert_eq!(
        staged_first["changed"].as_array().expect("changed files"),
        &[Value::String(rel.to_string())],
        "proof-map --staged should capture the staged file before caching"
    );

    git(repo.path(), &["reset", "-q", "HEAD", "--", rel]);
    let staged_second = run_lens_stdout(
        repo.path(),
        cache.path(),
        &["proof-map", "--staged", "--format", "json"],
    );
    let staged_second: Value =
        serde_json::from_str(&staged_second).expect("second proof-map json");
    assert!(
        staged_second["changed"]
            .as_array()
            .expect("changed files")
            .is_empty(),
        "proof-map cache must not reuse a staged artifact after the staged set changes: {staged_second:#}"
    );
}

#[test]
fn siblings_and_place_lens_artifacts_roundtrip_without_output_drift() {
    let (repo, cache) = fixture();
    let scope = "packages/app/src/features/studio/canvas";

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);

    let siblings_first = run_lens_stdout(repo.path(), cache.path(), &["siblings", scope]);
    let siblings_second = run_lens_stdout(repo.path(), cache.path(), &["siblings", scope]);
    assert_eq!(
        siblings_first, siblings_second,
        "cached siblings artifact must preserve markdown output"
    );

    let place_first = run_lens_stdout(
        repo.path(),
        cache.path(),
        &["place", scope, "--kind", "test"],
    );
    let place_second = run_lens_stdout(
        repo.path(),
        cache.path(),
        &["place", scope, "--kind", "test"],
    );
    assert_eq!(
        place_first, place_second,
        "cached place artifact must preserve markdown output"
    );

    assert!(
        cached_lens_artifact_exists(cache.path(), "siblings-current.json"),
        "siblings command should write an external lens artifact"
    );
    assert!(
        cached_lens_artifact_exists(cache.path(), "place-current.json"),
        "place command should write an external lens artifact"
    );
}

#[test]
fn siblings_lens_artifact_only_serves_exact_cache_key() {
    let (repo, cache) = fixture();
    let scope = "packages/app/src/features/studio/canvas";
    let other_scope = "packages/app/src/features/studio";
    let sentinel = "__cached_siblings_scope__";

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    let _ = run_json(
        repo.path(),
        cache.path(),
        &["siblings", scope, "--format", "json"],
    );
    poison_lens_report_field(cache.path(), "siblings-current.json", "scope", sentinel);
    let exact = run_json(
        repo.path(),
        cache.path(),
        &["siblings", scope, "--format", "json"],
    );
    assert_eq!(
        exact["scope"], sentinel,
        "siblings fast path should read the cached artifact for an exact key"
    );

    let include_hidden = run_json(
        repo.path(),
        cache.path(),
        &["siblings", scope, "--all", "--format", "json"],
    );
    assert_ne!(
        include_hidden["scope"], sentinel,
        "siblings cache must miss when include_hidden changes"
    );

    let _ = run_json(
        repo.path(),
        cache.path(),
        &["siblings", scope, "--format", "json"],
    );
    poison_lens_report_field(cache.path(), "siblings-current.json", "scope", sentinel);
    let limit = run_json(
        repo.path(),
        cache.path(),
        &["siblings", scope, "--limit", "7", "--format", "json"],
    );
    assert_ne!(limit["scope"], sentinel, "siblings cache must miss when limit changes");

    let _ = run_json(
        repo.path(),
        cache.path(),
        &["siblings", scope, "--format", "json"],
    );
    poison_lens_report_field(cache.path(), "siblings-current.json", "scope", sentinel);
    let different_scope = run_json(
        repo.path(),
        cache.path(),
        &["siblings", other_scope, "--format", "json"],
    );
    assert_ne!(
        different_scope["scope"], sentinel,
        "siblings cache must miss when scope changes"
    );
}

#[test]
fn place_lens_artifact_only_serves_exact_cache_key() {
    let (repo, cache) = fixture();
    let scope = "packages/app/src/features/studio/canvas";
    let other_scope = "packages/app/src/features/studio";
    let sentinel = "__cached_place_kind__";

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    let _ = run_json(
        repo.path(),
        cache.path(),
        &["place", scope, "--kind", "test", "--format", "json"],
    );
    poison_lens_report_field(
        cache.path(),
        "place-current.json",
        "requested_kind",
        sentinel,
    );
    let exact = run_json(
        repo.path(),
        cache.path(),
        &["place", scope, "--kind", "test", "--format", "json"],
    );
    assert_eq!(
        exact["requested_kind"], sentinel,
        "place fast path should read the cached artifact for an exact key"
    );

    let different_kind = run_json(
        repo.path(),
        cache.path(),
        &["place", scope, "--kind", "route", "--format", "json"],
    );
    assert_ne!(
        different_kind["requested_kind"], sentinel,
        "place cache must miss when kind changes"
    );

    let _ = run_json(
        repo.path(),
        cache.path(),
        &["place", scope, "--kind", "test", "--format", "json"],
    );
    poison_lens_report_field(
        cache.path(),
        "place-current.json",
        "requested_kind",
        sentinel,
    );
    let include_hidden = run_json(
        repo.path(),
        cache.path(),
        &[
            "place",
            scope,
            "--kind",
            "test",
            "--all",
            "--format",
            "json",
        ],
    );
    assert_ne!(
        include_hidden["requested_kind"], sentinel,
        "place cache must miss when include_hidden changes"
    );

    let _ = run_json(
        repo.path(),
        cache.path(),
        &["place", scope, "--kind", "test", "--format", "json"],
    );
    poison_lens_report_field(
        cache.path(),
        "place-current.json",
        "requested_kind",
        sentinel,
    );
    let limit = run_json(
        repo.path(),
        cache.path(),
        &[
            "place", scope, "--kind", "test", "--limit", "7", "--format", "json",
        ],
    );
    assert_ne!(
        limit["requested_kind"], sentinel,
        "place cache must miss when limit changes"
    );

    let _ = run_json(
        repo.path(),
        cache.path(),
        &["place", scope, "--kind", "test", "--format", "json"],
    );
    poison_lens_report_field(cache.path(), "place-current.json", "scope", sentinel);
    let different_scope = run_json(
        repo.path(),
        cache.path(),
        &["place", other_scope, "--kind", "test", "--format", "json"],
    );
    assert_ne!(
        different_scope["scope"], sentinel,
        "place cache must miss when scope changes"
    );
}

fn run_lens_stdout(repo: &Path, cache: &Path, args: &[&str]) -> String {
    let output = codemap()
        .current_dir(repo)
        .env("CODEMAP_CACHE_DIR", cache)
        .args(args)
        .output()
        .expect("codemap should run");
    assert!(
        output.status.success(),
        "codemap {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf8 stdout")
}

fn cached_lens_artifact_exists(cache_root: &Path, name: &str) -> bool {
    fs::read_dir(cache_root)
        .expect("cache dir")
        .filter_map(|entry| entry.ok())
        .any(|entry| entry.path().join(name).exists())
}

fn poison_lens_report_field(cache_root: &Path, name: &str, field: &str, value: &str) {
    let path = lens_artifact_path(cache_root, name);
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

fn lens_artifact_path(cache_root: &Path, name: &str) -> std::path::PathBuf {
    fs::read_dir(cache_root)
        .expect("cache dir")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path().join(name))
        .find(|path| path.exists())
        .unwrap_or_else(|| panic!("{name} should exist under {}", cache_root.display()))
}
