#[test]
fn clean_changed_fast_path_has_snapshot_fingerprint() {
    let (repo, cache) = fixture();
    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed"])
        .output()
        .expect("clean changed should run");
    assert!(
        output.status.success(),
        "clean changed failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("Map Snapshot: root=`")
            && markdown.contains("snapshot=`")
            && !markdown.contains("snapshot=`unknown`"),
        "clean changed fast path should still expose a bounded snapshot token: {markdown}"
    );
}
