fn snapshot_token(markdown: &str) -> String {
    let token = markdown
        .lines()
        .find_map(|line| {
            line.split("snapshot=`")
                .nth(1)
                .and_then(|rest| rest.split('`').next())
        })
        .map(str::to_string)
        .expect("changed output should carry a snapshot token");
    assert_eq!(token.len(), 16, "snapshot token should be 16 hex: {token}");
    assert!(
        token.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "snapshot token should be hex: {token}"
    );
    token
}

fn changed_markdown(repo: &std::path::Path, cache: &std::path::Path, args: &[&str]) -> String {
    let output = codemap()
        .current_dir(repo)
        .env("CODEMAP_CACHE_DIR", cache)
        .args(args)
        .output()
        .expect("codemap should run");
    assert!(
        output.status.success(),
        "codemap {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf8 stdout")
}

#[test]
fn snapshot_token_appears_in_changed_output() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/probe.ts"),
        "export const probe = 1;\n",
    );
    let token = snapshot_token(&changed_markdown(repo.path(), cache.path(), &["changed"]));
    assert_eq!(token.len(), 16);
}

#[test]
fn changed_since_snapshot_returns_only_delta() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/aaa.ts"),
        "export const aaa = 1;\n",
    );
    let token = snapshot_token(&changed_markdown(repo.path(), cache.path(), &["changed"]));
    write(
        &repo.path().join("packages/replay/src/bbb.ts"),
        "export const bbb = 1;\n",
    );

    let delta = changed_markdown(repo.path(), cache.path(), &["changed", "--since", &token]);
    assert!(
        delta.contains("packages/replay/src/bbb.ts"),
        "since-delta should show the file changed after the snapshot: {delta}"
    );
    assert!(
        !delta.contains("packages/replay/src/aaa.ts"),
        "since-delta should hide files unchanged since the snapshot: {delta}"
    );

    let full = changed_markdown(repo.path(), cache.path(), &["changed"]);
    assert!(
        full.contains("aaa.ts") && full.contains("bbb.ts"),
        "control: plain changed shows the full worktree set: {full}"
    );
}

#[test]
fn proof_changed_since_snapshot_scopes_proof() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/p-aaa.ts"),
        "export const pAaa = 1;\n",
    );
    let token = snapshot_token(&changed_markdown(repo.path(), cache.path(), &["changed"]));
    write(
        &repo.path().join("packages/replay/src/p-bbb.ts"),
        "export const pBbb = 1;\n",
    );

    let scoped = changed_markdown(
        repo.path(),
        cache.path(),
        &["proof", "changed", "--since", &token],
    );
    assert!(
        scoped.contains("p-bbb.ts") && !scoped.contains("p-aaa.ts"),
        "proof changed --since should scope to the snapshot delta: {scoped}"
    );
}

#[test]
fn snapshot_not_found_is_fail_open() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/probe.ts"),
        "export const probe = 1;\n",
    );
    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--since", "deadbeefdeadbeef"])
        .output()
        .expect("codemap should run");
    assert!(
        output.status.success(),
        "an unknown snapshot token must fail open, not error"
    );
    let markdown = String::from_utf8(output.stdout).expect("utf8");
    assert!(
        markdown.contains("snapshot_not_found")
            && markdown.contains("showing full git worktree changed set"),
        "missing snapshot should emit a typed fail-open unknown: {markdown}"
    );
}

#[test]
fn since_git_ref_still_resolves() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/ccc.ts"),
        "export const ccc = 1;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "second"]);

    let markdown = changed_markdown(repo.path(), cache.path(), &["changed", "--since", "HEAD~1"]);
    assert!(
        !markdown.contains("snapshot_not_found"),
        "a non-token --since like HEAD~1 must resolve as a git ref: {markdown}"
    );
}

#[test]
fn snapshot_store_is_bounded_and_keeps_zero_repo_footprint() {
    let (repo, cache) = fixture();
    for index in 0..40 {
        write(
            &repo
                .path()
                .join(format!("packages/replay/src/lru-{index}.ts")),
            &format!("export const lru{index} = {index};\n"),
        );
        let _ = changed_markdown(repo.path(), cache.path(), &["changed"]);
    }
    let snapshots_dir = std::fs::read_dir(cache.path())
        .expect("cache dir")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path().join("snapshots"))
        .find(|path| path.exists())
        .expect("snapshots dir should live under the external cache");
    let count = std::fs::read_dir(&snapshots_dir)
        .expect("snapshots dir")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
        .count();
    assert!(
        count <= 32,
        "snapshot store should be LRU-bounded to 32, got {count}"
    );

    // Snapshots live in the external cache, never in the target repo.
    let status = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo.path())
        .output()
        .expect("git status");
    let status = String::from_utf8(status.stdout).expect("utf8");
    assert!(
        !status.contains("snapshots"),
        "snapshots must not appear in the target repo: {status}"
    );
}

#[test]
fn changed_since_unknown_token_is_visible_on_clean_worktree() {
    // Regression A2: on a clean worktree the snapshot_not_found notice used to be
    // swallowed by the empty-changed early-return (visible only in --json). It must
    // show in default markdown so a missing --since snapshot is never silent.
    let (repo, cache) = fixture();
    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--since", "deadbeefdeadbeef"])
        .output()
        .expect("codemap should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("utf8");
    assert!(
        markdown.contains("snapshot_not_found"),
        "fail-open notice must be visible in default markdown on a clean worktree: {markdown}"
    );
}

#[test]
fn warm_snapshot_token_resolves_under_since() {
    // Regression A1: the token shown on a warm fast path must equal the snapshot
    // save-key (cached full fingerprint), so --since <that token> resolves instead of
    // failing open to the full worktree.
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/warm.ts"),
        "export const warm = 1;\n",
    );
    // First call saves the snapshot; the second is served from the warm path.
    let _ = changed_markdown(repo.path(), cache.path(), &["changed"]);
    let token = snapshot_token(&changed_markdown(repo.path(), cache.path(), &["changed"]));
    let delta = changed_markdown(repo.path(), cache.path(), &["changed", "--since", &token]);
    assert!(
        !delta.contains("snapshot_not_found"),
        "warm-path token must be backed by a saved snapshot: {delta}"
    );
}

#[test]
fn impact_and_proof_map_since_snapshot_scope_to_delta() {
    // Regression A3: impact/diff-map/proof-map used to treat a snapshot token as a git
    // ref and silently return empty. They must scope to the snapshot delta.
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/imp-a.ts"),
        "export const impA = 1;\n",
    );
    let token = snapshot_token(&changed_markdown(repo.path(), cache.path(), &["changed"]));
    write(
        &repo.path().join("packages/replay/src/imp-b.ts"),
        "export const impB = 1;\n",
    );
    for lens in [
        vec!["impact", "--since", &token],
        vec!["proof-map", "--since", &token],
        vec!["diff-map", "--since", &token],
    ] {
        let out = changed_markdown(repo.path(), cache.path(), &lens);
        assert!(
            out.contains("imp-b.ts") && !out.contains("imp-a.ts"),
            "`{lens:?}` --since should scope to the snapshot delta (imp-b, not imp-a): {out}"
        );
    }
}
