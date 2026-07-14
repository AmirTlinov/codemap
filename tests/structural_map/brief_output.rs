fn brief_markdown(repo: &std::path::Path, cache: &std::path::Path, args: &[&str]) -> String {
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

#[test]
fn default_output_drops_cache_telemetry_and_keeps_snapshot_token() {
    let (repo, cache) = fixture();
    let markdown = brief_markdown(repo.path(), cache.path(), &["ls", "."]);
    let snapshot = markdown
        .lines()
        .find(|line| line.starts_with("Map Snapshot:"))
        .expect("ls should carry a map snapshot line");
    assert!(
        snapshot.contains("snapshot=`"),
        "default snapshot line should carry the functional snapshot token: {snapshot}"
    );
    for telemetry in ["location=`", "cache=`", "strategy=`", "external_cache=`"] {
        assert!(
            !markdown.contains(telemetry),
            "default agent output must not leak cache telemetry `{telemetry}`: {markdown}"
        );
    }
    assert!(
        !markdown.contains(&repo.path().join("..").to_string_lossy().to_string())
            || !markdown.contains("Caches/codemap"),
        "default output must not print the external cache path: {markdown}"
    );
}

#[test]
fn brief_collapses_prelude_and_suppresses_disclaimers() {
    let (repo, cache) = fixture();
    // A real change gives `changed` a Surface Hints section (with disclaimer) and
    // a populated repo prelude block.
    write(
        &repo.path().join("packages/replay/src/brief-probe.ts"),
        "export const briefProbe = true;\n",
    );
    let default_changed = brief_markdown(repo.path(), cache.path(), &["changed"]);
    let brief_changed = brief_markdown(repo.path(), cache.path(), &["--brief", "changed"]);

    // The default `changed` map prints a multi-line `Repo:` block; brief collapses it.
    assert!(
        default_changed.contains("  root: `"),
        "default changed should print the expanded repo block: {default_changed}"
    );
    assert!(
        !brief_changed.contains("  root: `"),
        "brief changed should collapse the repo block to a single line: {brief_changed}"
    );
    assert!(
        brief_changed.lines().count() < default_changed.lines().count(),
        "brief changed should be shorter than default: brief={} default={}",
        brief_changed.lines().count(),
        default_changed.lines().count()
    );

    assert!(
        default_changed.contains("Derived from deterministic"),
        "default changed should keep the surface-hint disclaimer: {default_changed}"
    );
    assert!(
        !brief_changed.contains("Derived from deterministic"),
        "brief changed should suppress repeated disclaimers: {brief_changed}"
    );
}

#[test]
fn brief_env_matches_flag() {
    let (repo, cache) = fixture();
    let flag = brief_markdown(repo.path(), cache.path(), &["--brief", "ls", "."]);
    let env = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .env("CODEMAP_BRIEF", "1")
        .args(["ls", "."])
        .output()
        .expect("codemap should run");
    let env = String::from_utf8(env.stdout).expect("utf8 stdout");
    assert_eq!(
        flag.lines().count(),
        env.lines().count(),
        "CODEMAP_BRIEF=1 should match --brief output shape: flag={flag} env={env}"
    );
    assert!(
        !env.contains("Derived from deterministic"),
        "CODEMAP_BRIEF=1 should suppress disclaimers like --brief: {env}"
    );
}

#[test]
fn cache_telemetry_env_restores_debug_fields() {
    let (repo, cache) = fixture();
    let telemetry = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .env("CODEMAP_CACHE_TELEMETRY", "1")
        .args(["ls", "."])
        .output()
        .expect("codemap should run");
    let telemetry = String::from_utf8(telemetry.stdout).expect("utf8 stdout");
    assert!(
        telemetry.contains("cache=`")
            && telemetry.contains("strategy=`")
            && telemetry.contains("location=`"),
        "CODEMAP_CACHE_TELEMETRY=1 should restore debug cache fields: {telemetry}"
    );
}
