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
            && !markdown.contains("| Surface | Count |")
            && !markdown.contains("| Kind | Count |"),
        "changed markdown should not return to Git State, Map Delta, or proof sensor table spam: {markdown}"
    );
    assert!(
        markdown.contains("\n### Sensor Counts\n") && markdown.contains("- direct: `"),
        "changed proof summary should render sensor counts as compact bullets: {markdown}"
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
fn changed_does_not_turn_comment_only_edits_into_symbol_delta() {
    let (repo, cache) = fixture();
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/comment-only.tsx"),
        "/*\n  Updated prose only: no route, import, export, or symbol body change.\n*/\nexport function CommentOnly() {\n  return <div />;\n}\n",
    );

    let diff = run_json(repo.path(), cache.path(), &["diff-map", "--changed", "--format", "json"]);
    assert_schema("schemas/diff-map.schema.json", &diff);
    assert!(
        diff["changed_symbols"]
            .as_array()
            .expect("changed symbols")
            .is_empty(),
        "comment-only edits should not create changed symbol surfaces: {diff:#}"
    );
    assert!(
        diff["added_edges"].as_array().expect("added edges").is_empty()
            && diff["removed_edges"]
                .as_array()
                .expect("removed edges")
                .is_empty()
            && diff["added_runtime_routes"]
                .as_array()
                .expect("added routes")
                .is_empty()
            && diff["added_proof_surfaces"]
                .as_array()
                .expect("added proof")
                .is_empty(),
        "comments/docs are not hard structural proof or runtime evidence: {diff:#}"
    );

    let changed = run_json(repo.path(), cache.path(), &["changed", "--format", "json"]);
    assert_schema("schemas/changed.schema.json", &changed);
    assert_eq!(
        changed["map_delta"]["changed_symbols"].as_u64(),
        Some(0),
        "changed overview should not overclaim comment-only symbol deltas: {changed:#}"
    );
}

#[test]
fn changed_does_not_turn_inline_comment_edits_into_symbol_delta() {
    let (repo, cache) = fixture();
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/comment-only.tsx"),
        "/*\n  <button aria-label=\"Open settings panel\" data-testid=\"submit-order-button\">Settings</button>\n  await page.goto('/orders/new');\n*/\nexport function CommentOnly() {\n  return <div />; // updated prose only\n}\n",
    );

    let diff = run_json(repo.path(), cache.path(), &["diff-map", "--changed", "--format", "json"]);
    assert_schema("schemas/diff-map.schema.json", &diff);
    assert!(
        diff["changed_symbols"]
            .as_array()
            .expect("changed symbols")
            .is_empty(),
        "inline comment-only edits inside a symbol body should not create changed symbol surfaces: {diff:#}"
    );

    let changed = run_json(repo.path(), cache.path(), &["changed", "--format", "json"]);
    assert_schema("schemas/changed.schema.json", &changed);
    assert_eq!(
        changed["map_delta"]["changed_symbols"].as_u64(),
        Some(0),
        "changed overview should not overclaim inline comment-only symbol deltas: {changed:#}"
    );
}

#[test]
fn changed_keeps_url_string_literal_edits_as_symbol_delta() {
    let (repo, cache) = fixture();
    let path = repo
        .path()
        .join("packages/app/src/features/studio/comment-only.tsx");
    write(
        &path,
        "export function CommentOnly() {\n  return \"http://old.example.test\";\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "url baseline"]);
    write(
        &path,
        "export function CommentOnly() {\n  return \"http://new.example.test\";\n}\n",
    );

    let diff = run_json(repo.path(), cache.path(), &["diff-map", "--changed", "--format", "json"]);
    assert_schema("schemas/diff-map.schema.json", &diff);
    assert!(
        diff["changed_symbols"]
            .as_array()
            .expect("changed symbols")
            .iter()
            .any(|symbol| symbol["path"] == "packages/app/src/features/studio/comment-only.tsx"
                && symbol["name"] == "CommentOnly"
                && symbol["change"] == "symbol_body_changed"),
        "real string literal edits containing // should remain structural symbol deltas: {diff:#}"
    );

    let changed = run_json(repo.path(), cache.path(), &["changed", "--format", "json"]);
    assert_schema("schemas/changed.schema.json", &changed);
    assert_eq!(
        changed["map_delta"]["changed_symbols"].as_u64(),
        Some(1),
        "changed overview should keep real URL string edits as symbol deltas: {changed:#}"
    );
}

#[test]
fn changed_does_not_mark_symbol_body_when_import_above_symbol_is_removed() {
    let (repo, cache) = fixture();
    let path = repo
        .path()
        .join("packages/app/src/features/studio/comment-only.tsx");
    write(
        &path,
        "import { SettingsButton } from './settings-button';\n\nexport function CommentOnly() {\n  return <SettingsButton />;\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "import baseline"]);
    write(
        &path,
        "export function CommentOnly() {\n  return <SettingsButton />;\n}\n",
    );

    let diff = run_json(repo.path(), cache.path(), &["diff-map", "--changed", "--format", "json"]);
    assert_schema("schemas/diff-map.schema.json", &diff);
    assert!(
        diff["changed_symbols"]
            .as_array()
            .expect("changed symbols")
            .is_empty(),
        "removing an import above a symbol should be a removed edge, not a false symbol body delta: {diff:#}"
    );
    assert!(
        !diff["removed_edges"]
            .as_array()
            .expect("removed edges")
            .is_empty(),
        "the removed import should remain visible as a structural edge delta: {diff:#}"
    );

    let changed = run_json(repo.path(), cache.path(), &["changed", "--format", "json"]);
    assert_schema("schemas/changed.schema.json", &changed);
    assert_eq!(
        changed["map_delta"]["changed_symbols"].as_u64(),
        Some(0),
        "changed overview should not overclaim symbol deltas from removed old-coordinate lines: {changed:#}"
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
