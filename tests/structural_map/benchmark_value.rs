#[test]
fn value_benchmark_reports_context_compression_without_agent_lift_claim() {
    let repo = TempDir::new().expect("benchmark fixture repo");
    let out = TempDir::new().expect("benchmark output dir");
    let cache = TempDir::new().expect("benchmark cache dir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"benchmark-fixture","private":true,"scripts":{"test":"vitest run","typecheck":"tsc --noEmit"}}"#,
    );
    write(
        &repo.path().join("src/session.ts"),
        "export function sessionValue(input: number) {\n  return input + 1;\n}\n",
    );
    write(
        &repo.path().join("tests/session.test.ts"),
        "import { sessionValue } from '../src/session';\n\ntest('session value', () => {\n  expect(sessionValue(1)).toBe(2);\n});\n",
    );
    let mut large_module = String::new();
    for index in 0..700 {
        large_module.push_str(&format!(
            "export function generatedHelper{index}(input: number) {{\n  return input + {index};\n}}\n\n"
        ));
    }
    write(&repo.path().join("src/generated-context.ts"), &large_module);
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "benchmark fixture"]);

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let codemap_bin = Path::new(env!("CARGO_BIN_EXE_codemap"));
    let codemap_arg = codemap_bin.strip_prefix(repo_root).unwrap_or(codemap_bin);
    let output = Command::new("python3")
        .arg(repo_root.join("scripts/benchmark-codemap-value.py"))
        .arg(repo.path())
        .arg("--codemap-bin")
        .arg(codemap_arg)
        .arg("--out-dir")
        .arg(out.path())
        .current_dir(repo_root)
        .env("CODEMAP_CACHE_DIR", cache.path())
        .output()
        .expect("benchmark script should run");
    assert!(
        output.status.success(),
        "benchmark failed: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let summary_md = fs::read_to_string(out.path().join("summary.md")).expect("summary markdown");
    assert!(
        summary_md.contains("deterministic context compression")
            && summary_md.contains("does not prove behavioral model lift")
            && summary_md.contains("Saved")
            && summary_md.contains("Compression"),
        "benchmark summary should state the honest claim boundary and value metrics: {summary_md}"
    );
    assert!(
        summary_md.contains("Signals / 1k map tokens"),
        "benchmark summary should expose navigation signal density: {summary_md}"
    );

    let summary_jsonl = fs::read_to_string(out.path().join("summary.jsonl")).expect("summary jsonl");
    let row: Value = serde_json::from_str(summary_jsonl.lines().next().expect("summary row"))
        .expect("summary row json");
    let baseline_tokens = row["baseline"]["approx_tokens"]
        .as_i64()
        .expect("baseline tokens");
    let codemap_tokens = row["codemap_daily_map"]["approx_tokens"]
        .as_i64()
        .expect("codemap tokens");
    assert!(
        baseline_tokens > codemap_tokens,
        "fixture should prove token compression: {row:#}"
    );
    assert_eq!(
        row["claim_boundary"].as_str(),
        Some(
            "This benchmark proves deterministic context compression and navigation signal density. It does not prove behavioral model lift; use a paired model A/B task benchmark for that."
        ),
        "benchmark should not overclaim agent intelligence: {row:#}"
    );
    assert!(
        row["result"]["navigation_signal_density"]["unique_path_mentions"]
            .as_i64()
            .unwrap_or_default()
            > 0,
        "benchmark should count path navigation signal: {row:#}"
    );
    assert!(
        row["result"]["navigation_signal_density"]["navigation_signals_per_1k_tokens"]
            .as_f64()
            .unwrap_or_default()
            > 0.0,
        "benchmark should count total navigation signal density: {row:#}"
    );
    assert!(
        row["codemap_daily_map"]["all_commands_succeeded"]
            .as_bool()
            .unwrap_or(false),
        "daily codemap benchmark probes should succeed: {row:#}"
    );

    let label = repo
        .path()
        .file_name()
        .and_then(|name| name.to_str())
        .expect("repo tempdir label");
    assert!(
        out.path().join(label).join("ls_root.md").exists()
            && out.path().join(label).join("changed.md").exists()
            && out.path().join(label).join("proof_changed.md").exists()
            && out.path().join(label).join("cone_anchor.md").exists(),
        "benchmark should persist per-command readable artifacts"
    );
    let status = Command::new("git")
        .args(["status", "--short"])
        .current_dir(repo.path())
        .output()
        .expect("git status should run");
    assert!(
        status.stdout.is_empty(),
        "benchmark script must not write into target repos: {}",
        String::from_utf8_lossy(&status.stdout)
    );
}

#[test]
fn value_benchmark_fails_closed_when_codemap_probe_fails() {
    let repo = TempDir::new().expect("benchmark fixture repo");
    let out = TempDir::new().expect("benchmark output dir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("src/session.ts"),
        "export function sessionValue() {\n  return 1;\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "benchmark fixture"]);

    let failing_bin = out.path().join("failing-codemap");
    write(&failing_bin, "#!/usr/bin/env sh\nexit 23\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&failing_bin)
            .expect("failing binary metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&failing_bin, permissions).expect("chmod failing binary");
    }

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new("python3")
        .arg(repo_root.join("scripts/benchmark-codemap-value.py"))
        .arg(repo.path())
        .arg("--codemap-bin")
        .arg(&failing_bin)
        .arg("--out-dir")
        .arg(out.path().join("result"))
        .output()
        .expect("benchmark script should run");
    assert!(
        !output.status.success(),
        "benchmark should fail closed when codemap probes fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("benchmark failed: codemap probes failed"),
        "benchmark should name failed probes: {stderr}"
    );
    let summary_md =
        fs::read_to_string(out.path().join("result/summary.md")).expect("summary markdown");
    assert!(
        summary_md.contains("| failed |"),
        "failed benchmark should show failed status in readable summary: {summary_md}"
    );
    assert!(
        summary_md.contains("| - | - |"),
        "failed benchmark should not render fake savings/compression: {summary_md}"
    );
    let summary_jsonl =
        fs::read_to_string(out.path().join("result/summary.jsonl")).expect("summary jsonl");
    let row: Value = serde_json::from_str(summary_jsonl.lines().next().expect("summary row"))
        .expect("summary row json");
    assert_eq!(row["result"]["status"].as_str(), Some("failed"));
    assert!(
        row["result"]["compression_ratio_vs_visible_text"].is_null()
            && row["result"]["token_savings_percent_vs_visible_text"].is_null(),
        "failed benchmark should not serialize fake win metrics: {row:#}"
    );
}
