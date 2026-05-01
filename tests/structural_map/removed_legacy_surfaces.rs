#[test]
fn removed_graph_lens_aliases_fail_closed() {
    let (repo, cache) = fixture();
    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["graph", "--lens", "verify", "--format", "json"])
        .output()
        .expect("codemap should run");
    assert!(
        !output.status.success(),
        "removed verify lens alias must not silently fall back"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown graph lens"));
}


#[test]
fn removed_router_commands_and_flags_fail_closed() {
    let (repo, cache) = fixture();
    let removed_command = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["explain", "packages/replay/src/session.ts"])
        .output()
        .expect("codemap should run");
    assert!(
        !removed_command.status.success(),
        "removed explain command must not be accepted"
    );

    let removed_flag = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--files",
            "packages/replay/src/session.ts",
            "--structural",
        ])
        .output()
        .expect("codemap should run");
    assert!(
        !removed_flag.status.success(),
        "removed --structural flag must not be accepted"
    );
}
