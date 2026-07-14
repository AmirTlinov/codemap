fn assert_lens_markdown_eq(left: &str, right: &str, message: &str) {
    assert_map_snapshot_has_provenance(left);
    assert_map_snapshot_has_provenance(right);
    assert_eq!(
        normalize_invocation_snapshot(left),
        normalize_invocation_snapshot(right),
        "{message}"
    );
}

fn assert_map_snapshot_has_provenance(text: &str) {
    if text.lines().any(|line| line.starts_with("Repo:")) {
        assert!(
            text.contains("worktree=`") || text.contains("  worktree: `"),
            "repo prelude should carry worktree truth: {text}"
        );
        assert!(
            text.contains("remote_refs=`") || text.contains("  remote_refs: `"),
            "repo prelude should carry remote currentness boundary: {text}"
        );
        assert!(
            text.contains("no network used"),
            "repo prelude must state that remote currentness is local-only: {text}"
        );
        return;
    }
    let snapshot = text
        .lines()
        .find(|line| line.starts_with("Map Snapshot:"))
        .expect("markdown output should include a map snapshot or repo prelude line");
    assert!(
        snapshot.contains("branch=`")
            && snapshot.contains("dirty=`")
            && snapshot.contains("snapshot=`")
            && snapshot.contains("schema=`")
            && snapshot.contains("repo_footprint=`zero`"),
        "snapshot should carry provenance fields: {snapshot}"
    );
}

fn normalize_invocation_snapshot(text: &str) -> String {
    text.lines()
        .map(|line| {
            if line.starts_with("Map Snapshot:") {
                "Map Snapshot: <invocation snapshot>"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
