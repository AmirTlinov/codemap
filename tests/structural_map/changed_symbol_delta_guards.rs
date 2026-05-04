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
