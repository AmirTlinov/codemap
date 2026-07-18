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

fn snapshot_json(repo: &std::path::Path, cache: &std::path::Path, args: &[&str]) -> Value {
    run_json(repo, cache, args)
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
fn snapshot_scoped_daily_reports_reuse_the_exact_warm_artifact() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/warm-a.ts"),
        "export const warmA = 1;\n",
    );
    let token = snapshot_token(&changed_markdown(repo.path(), cache.path(), &["changed"]));
    write(
        &repo.path().join("packages/replay/src/warm-b.ts"),
        "export const warmB = 1;\n",
    );

    let first = changed_markdown(repo.path(), cache.path(), &["changed", "--since", &token]);
    assert!(first.contains("warm-b.ts") && !first.contains("warm-a.ts"));

    let snapshot = std::fs::read_dir(cache.path())
        .expect("cache root")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path().join("snapshots").join(format!("{token}.json")))
        .find(|path| path.is_file())
        .expect("saved snapshot");
    std::fs::remove_file(snapshot).expect("remove source snapshot after warming exact reports");

    for args in [
        vec!["changed", "--since", &token],
        vec!["proof", "changed", "--since", &token],
    ] {
        let warmed = changed_markdown(repo.path(), cache.path(), &args);
        assert!(
            warmed.contains("warm-b.ts") && !warmed.contains("snapshot_not_found"),
            "the exact warm report should not recompute its removed snapshot: {warmed}"
        );
    }
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
            && markdown.contains("showing full git worktree changed set (1 files)"),
        "missing snapshot should emit a typed fail-open unknown: {markdown}"
    );
    for lens in ["impact", "diff-map", "proof-map"] {
        let output = changed_markdown(
            repo.path(),
            cache.path(),
            &[lens, "--since", "deadbeefdeadbeef"],
        );
        assert!(
            output.contains("snapshot_not_found") && output.contains("(1 files)"),
            "{lens} must report the full fallback scale: {output}"
        );
    }
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

#[test]
fn snapshot_delta_is_phase_local_and_keeps_downstream_context() {
    let (repo, cache) = fixture();
    let producer = repo.path().join("packages/replay/src/session-producer.ts");
    write(&producer, "export const beforeSession = 1;\n");
    write(
        &repo.path().join("packages/replay/src/session-consumer.ts"),
        "import { beforeSession } from './session-producer';\nexport const consumed = beforeSession;\n",
    );
    for index in 0..100 {
        write(
            &repo
                .path()
                .join(format!("packages/replay/src/pre-session-{index:03}.ts")),
            &format!("export const preSession{index} = {index};\n"),
        );
    }
    let first = snapshot_json(repo.path(), cache.path(), &["changed", "--format", "json"]);
    assert!(first["total_changed_count"].as_u64().unwrap_or(0) >= 102);
    let token = first["session_snapshot"]["token"]
        .as_str()
        .expect("session token")
        .to_string();
    assert_eq!(first["session_snapshot"]["freshness"], "exact");
    assert_eq!(first["session_snapshot"]["storage"], "external_cache");
    assert!(first["session_snapshot"]["created_unix_seconds"].is_u64());

    write(
        &producer,
        "export const beforeSession = 1;\nexport const afterSession = 2;\n",
    );
    write(
        &repo.path().join("packages/replay/src/session-runtime.ts"),
        "export const sessionUrl = process.env.SESSION_URL;\n",
    );
    write(
        &repo.path().join("packages/replay/src/session-proof.spec.ts"),
        "import { afterSession } from './session-producer';\ntest('session delta', () => expect(afterSession).toBe(2));\n",
    );
    let delta = snapshot_json(
        repo.path(),
        cache.path(),
        &["changed", "--since", &token, "--format", "json"],
    );
    assert_eq!(delta["selection"]["kind"], "snapshot");
    assert_eq!(delta["selection"]["resolved"], true);
    assert_eq!(delta["selection"]["selected_files"], 3);
    assert_eq!(delta["total_changed_count"], 3);
    assert_eq!(delta["map_delta"]["added_exports"], 2);
    let changed = delta["changed"].as_array().expect("changed files");
    assert_eq!(changed.len(), 3);
    assert!(changed.iter().any(|file| file["path"] == "packages/replay/src/session-producer.ts"));
    let impact = delta["impact"].as_array().expect("impact clusters");
    assert!(
        impact.iter().any(|cluster| cluster["direct_consumers"]
            .as_array()
            .is_some_and(|edges| edges.iter().any(|edge| edge["from"] == "packages/replay/src/session-consumer.ts"))),
        "the unchanged downstream consumer must remain visible as context: {delta:#}"
    );

    let proof = snapshot_json(
        repo.path(),
        cache.path(),
        &["proof", "changed", "--since", &token, "--format", "json"],
    );
    let proof_changed = proof["changed"].as_array().expect("proof changed");
    assert_eq!(proof_changed.len(), 3);
    assert!(proof_changed.iter().any(|path| path == "packages/replay/src/session-producer.ts"));
}
