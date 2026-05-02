#[test]
fn changed_combines_delta_impact_and_proof_without_running_commands() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/session.ts"),
        "import { Timeline } from './timeline';\nimport type { FrameDto } from './types';\n\nexport function seek(cursor: number): FrameDto {\n  return { frame: new Timeline().frameAt(cursor + 1) };\n}\n\nexport function seekLabel() {\n  return 'seek';\n}\n",
    );

    let changed = run_json(repo.path(), cache.path(), &["changed", "--format", "json"]);
    assert_schema("schemas/changed.schema.json", &changed);
    assert_eq!(changed["kind"], "changed_report");
    assert_eq!(changed["schema_version"], "1");
    assert!(
        changed["changed"]
            .as_array()
            .expect("changed anchors")
            .iter()
            .any(|file| file["path"] == "packages/replay/src/session.ts"),
        "changed should include the touched anchor: {changed:#}"
    );
    assert!(
        changed["map_delta"]["changed_symbols"]
            .as_u64()
            .unwrap_or_default()
            >= 1,
        "changed should summarize map delta from diff-map facts: {changed:#}"
    );
    assert!(
        !changed["impact"].as_array().expect("impact").is_empty(),
        "changed should include structural impact clusters: {changed:#}"
    );
    assert!(
        !changed["proof"]["commands"]
            .as_array()
            .expect("proof commands")
            .is_empty()
            || !changed["proof"]["fallback"]
                .as_array()
                .expect("fallback")
                .is_empty(),
        "changed should include proof overview without running commands: {changed:#}"
    );
    assert!(
        changed["expand"]
            .as_array()
            .expect("expand")
            .iter()
            .any(|command| command == "codemap changed --section proof"
                || command == "codemap changed --files packages/replay/src/session.ts --section proof"),
        "changed should provide deterministic proof drill-down: {changed:#}"
    );
}

#[test]
fn changed_reports_clean_state_calmly() {
    let (repo, cache) = fixture();

    let changed = run_json(repo.path(), cache.path(), &["changed", "--format", "json"]);
    assert_schema("schemas/changed.schema.json", &changed);
    assert!(
        changed["changed"].as_array().expect("changed").is_empty(),
        "clean repo should have no changed anchors: {changed:#}"
    );
    assert!(
        changed["impact"].as_array().expect("impact").is_empty(),
        "clean repo should have no impact clusters: {changed:#}"
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .arg("changed")
        .output()
        .expect("changed markdown should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("No changed anchors detected."),
        "clean changed markdown should be calm: {markdown}"
    );
}

#[test]
fn changed_distinguishes_visible_and_total_changed_files() {
    let (repo, cache) = fixture();
    for index in 0..5 {
        write(
            &repo
                .path()
                .join(format!("packages/replay/src/extra-{index}.ts")),
            &format!("export const extra{index} = {index};\n"),
        );
    }

    let changed = run_json(
        repo.path(),
        cache.path(),
        &["changed", "--limit", "2", "--format", "json"],
    );
    assert_schema("schemas/changed.schema.json", &changed);
    assert_eq!(
        changed["total_changed_count"].as_u64(),
        Some(5),
        "changed should expose total changed anchors separately from visible rows: {changed:#}"
    );
    assert_eq!(
        changed["changed"].as_array().expect("visible changed").len(),
        2,
        "changed should keep markdown/json visible rows bounded: {changed:#}"
    );
    assert!(
        changed["git_state"]
            .as_array()
            .expect("git state")
            .iter()
            .all(|entry| entry["status"] == "untracked"),
        "untracked files should be typed git-state facts: {changed:#}"
    );
    assert!(
        changed["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|group| group["reason"]
                .as_str()
                .unwrap_or_default()
                .contains("changed file summaries hidden by limit")
                && group["count"].as_u64() == Some(3)),
        "hidden changed summaries should account for truncated rows: {changed:#}"
    );
    assert!(
        !changed["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|group| group["reason"] == "git state rows hidden by limit"),
        "JSON keeps complete git_state, so render-only Git State truncation should not appear as report hidden: {changed:#}"
    );
    assert_eq!(
        changed["git_state"].as_array().expect("git state").len(),
        5,
        "JSON should keep full git state even when markdown is bounded: {changed:#}"
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--limit", "2"])
        .output()
        .expect("changed markdown should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("Changed: `2` shown / `5` total files"),
        "markdown should not under-report truncated changed anchors: {markdown}"
    );
    assert_eq!(
        markdown.matches("[untracked; staged=false; unstaged=true]").count(),
        2,
        "markdown should bound Git State rows by the changed limit: {markdown}"
    );
    assert!(
        markdown.contains("- changed symbols: `5`") || markdown.contains("- added exports: `5`"),
        "markdown should render Map Delta as compact count bullets: {markdown}"
    );
    assert!(
        markdown.contains("[risk=") && !markdown.contains("| Cluster | Risk | Reasons | Edges |"),
        "changed markdown should render impact summary as compact cluster bullets: {markdown}"
    );
    assert!(
        !markdown.contains("| Status | Path | Old | Staged | Unstaged |")
            && !markdown.contains("| Surface | Count |"),
        "changed markdown should not return to Git State or Map Delta table spam: {markdown}"
    );
    assert!(
        markdown.contains("git state rows hidden by limit"),
        "markdown should expose hidden Git State rows with expand: {markdown}"
    );
    assert!(
        !markdown.contains("--files "),
        "changed markdown hidden expands should not dump long selected-file lists: {markdown}"
    );
}

#[test]
fn dogfood_script_refuses_cleanup_outside_target_or_temp() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let refused_out = repo_root.join("dogfood-refused-output");
    let _ = fs::remove_dir_all(&refused_out);

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/dogfood-codemap.sh"))
        .env("CODEMAP_DOGFOOD_OUT", &refused_out)
        .output()
        .expect("dogfood script should run");

    assert_eq!(
        output.status.code(),
        Some(2),
        "dogfood script should refuse unsafe output dirs; stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !refused_out.exists(),
        "dogfood script should not create refused output dir"
    );
}

#[test]
fn dogfood_script_refuses_traversal_outside_target() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let refused_out = repo_root.join("dogfood-refused-output");
    let traversal_out = repo_root.join("target/../../dogfood-refused-output");
    let _ = fs::remove_dir_all(&refused_out);

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/dogfood-codemap.sh"))
        .env("CODEMAP_DOGFOOD_OUT", &traversal_out)
        .output()
        .expect("dogfood script should run");

    assert_eq!(
        output.status.code(),
        Some(2),
        "dogfood script should refuse traversal outside target; stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !refused_out.exists(),
        "dogfood script should not create traversal-refused output dir"
    );
}
