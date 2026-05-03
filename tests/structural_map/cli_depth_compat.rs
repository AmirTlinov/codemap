#[test]
fn daily_commands_accept_depth_one_without_cli_friction() {
    let (repo, cache) = fixture();
    for args in [
        &["ls", ".", "--depth", "1", "--format", "json"][..],
        &["changed", "--depth", "1", "--format", "json"][..],
    ] {
        let output = codemap()
            .current_dir(repo.path())
            .env("CODEMAP_CACHE_DIR", cache.path())
            .args(args)
            .output()
            .expect("codemap should run");
        assert!(
            output.status.success(),
            "{args:?} should accept --depth 1 as a no-friction daily flag: stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "{args:?} should emit JSON after accepting --depth 1: {error}; stdout={}; stderr={}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
        });
        assert!(
            matches!(
                report["kind"].as_str(),
                Some("ls_report" | "changed_report")
            ),
            "{args:?} should still return a normal structural report: {report:#}"
        );
    }
}

#[test]
fn daily_commands_fail_closed_for_expanded_depth() {
    let (repo, cache) = fixture();
    for args in [
        &["ls", ".", "--depth", "2"][..],
        &["changed", "--depth", "2"][..],
    ] {
        let output = codemap()
            .current_dir(repo.path())
            .env("CODEMAP_CACHE_DIR", cache.path())
            .args(args)
            .output()
            .expect("codemap should run");
        assert!(
            !output.status.success(),
            "{args:?} should not silently accept expanded depth for fixed-depth daily maps"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("currently keeps depth fixed at 1")
                && stderr.contains("codemap cone <anchor> --depth 2")
                && stderr.contains("codemap proof <anchor|changed> --depth 2"),
            "{args:?} should fail closed with the focused expansion path: {stderr}"
        );
    }
}
