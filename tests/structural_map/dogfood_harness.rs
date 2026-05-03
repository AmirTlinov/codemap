#[test]
fn dogfood_script_runs_daily_and_focused_probes_read_only() {
    let repo = TempDir::new().expect("repo tempdir");
    let out = TempDir::new().expect("dogfood output tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"dogfood-harness-fixture","private":true,"scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join("src/session.ts"),
        "export function sessionValue() {\n  return 1;\n}\n",
    );
    write(
        &repo.path().join("tests/session.test.ts"),
        "import { sessionValue } from '../src/session';\n\ntest('session value', () => {\n  expect(sessionValue()).toBe(1);\n});\n",
    );
    write(
        &repo.path().join(".env.example"),
        "DATABASE_URL=\nSESSION_SECRET=\n",
    );
    write(
        &repo.path().join("prisma/schema.prisma"),
        "datasource db { provider = \"postgresql\" url = env(\"DATABASE_URL\") }\ngenerator client { provider = \"prisma-client-js\" }\nmodel Session { id String @id }\n",
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - run: npm test\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "dogfood fixture"]);

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new("bash")
        .arg(repo_root.join("scripts/dogfood-codemap.sh"))
        .env("CODEMAP_BIN", env!("CARGO_BIN_EXE_codemap"))
        .env("CODEMAP_DOGFOOD_OUT", out.path())
        .arg(repo.path())
        .output()
        .expect("dogfood script should run");
    assert!(
        output.status.success(),
        "dogfood script failed: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("[dogfood] repo-start")
            && stderr.contains("[dogfood] run repo=")
            && stderr.contains("[dogfood] done repo=")
            && stderr.contains("[dogfood] summary probes="),
        "dogfood script should expose live progress on stderr: {stderr}"
    );

    let summary_path = out.path().join("summary.jsonl");
    let summary = fs::read_to_string(&summary_path).expect("summary jsonl");
    let rows: Vec<Value> = summary
        .lines()
        .map(|line| serde_json::from_str(line).expect("summary line json"))
        .collect();
    assert_eq!(
        rows.first().and_then(|value| value["label"].as_str()),
        Some("ls_root"),
        "dogfood should start with a cache-writing agent map command, not read-only doctor: {summary}"
    );
    let first_four: Vec<_> = rows
        .iter()
        .take(4)
        .filter_map(|value| value["label"].as_str())
        .collect();
    assert_eq!(
        first_four,
        ["ls_root", "changed", "proof_changed", "doctor"],
        "dogfood daily probes should follow the agent preflight order before diagnostics: {summary}"
    );
    let ls_root_index = rows
        .iter()
        .position(|value| value["label"] == "ls_root")
        .expect("ls_root summary row");
    let doctor_index = rows
        .iter()
        .position(|value| value["label"] == "doctor")
        .expect("doctor summary row");
    assert!(
        ls_root_index < doctor_index,
        "dogfood should warm cache before read-only doctor diagnostics: {summary}"
    );
    for label in [
        "doctor",
        "ls_root",
        "ls_links",
        "changed",
        "proof_changed",
        "cone_anchor",
        "cone_owner",
        "proof_owner",
        "cone_owner_manifest",
        "proof_owner_manifest",
        "cone_owner_schema",
        "proof_owner_schema",
        "cone_owner_env",
        "proof_owner_env",
        "cone_owner_ci",
        "proof_owner_ci",
        "contract_anchor",
        "delete_anchor",
    ] {
        assert!(
            summary.contains(&format!(r#""label":"{label}""#)),
            "dogfood summary should include {label}: {summary}"
        );
    }
    for value in rows {
        if value.get("command").is_some() {
            assert_eq!(
                value["status"], 0,
                "dogfood probes should succeed in the controlled fixture: {value:#}"
            );
            assert!(
                value.get("elapsed_ms").is_some()
                    && value.get("lines").is_some()
                    && value.get("line_budget").is_some()
                    && value.get("hidden_lines").is_some()
                    && value.get("unknown_lines").is_some()
                    && value.get("map_quality_lines").is_some()
                    && value.get("budget_status").is_some(),
                "dogfood command summaries should include timing and line-budget fields: {value:#}"
            );
        }
    }

    let status = Command::new("git")
        .args(["status", "--short"])
        .current_dir(repo.path())
        .output()
        .expect("git status should run");
    assert!(
        status.stdout.is_empty(),
        "dogfood script must not write into target repos: {}",
        String::from_utf8_lossy(&status.stdout)
    );
}
