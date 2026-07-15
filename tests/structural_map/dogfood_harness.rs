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
            && stderr.contains("latency=")
            && stderr.contains("[dogfood] summary probes=")
            && stderr.contains("slow=")
            && stderr.contains("primary_slow="),
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
                    && value.get("latency_budget_ms").is_some()
                    && value.get("latency_status").is_some()
                    && value.get("lines").is_some()
                    && value.get("line_budget").is_some()
                    && value.get("hidden_lines").is_some()
                    && value.get("unknown_lines").is_some()
                    && value.get("map_quality_lines").is_some()
                    && value.get("trust_violations").is_some()
                    && value.get("budget_status").is_some(),
                "dogfood command summaries should include timing and line-budget fields: {value:#}"
            );
            assert_eq!(
                value["trust_violations"], 0,
                "dogfood fixture should not emit legacy role/verdict wording in agent-facing output: {value:#}"
            );
            assert!(
                matches!(value["latency_status"].as_str(), Some("ok" | "slow")),
                "dogfood latency status should be explicit and bounded: {value:#}"
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

#[test]
fn dogfood_strict_mode_fails_closed_on_summary_violations() {
    let parent = TempDir::new().expect("dogfood target parent");
    let out = TempDir::new().expect("dogfood output tempdir");
    let missing = parent.path().join("missing-repo");
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/dogfood-codemap.sh"))
        .env("CODEMAP_BIN", env!("CARGO_BIN_EXE_codemap"))
        .env("CODEMAP_DOGFOOD_OUT", out.path())
        .env("CODEMAP_DOGFOOD_STRICT", "1")
        .arg(&missing)
        .output()
        .expect("dogfood script should run");

    assert!(
        !output.status.success(),
        "strict dogfood should fail closed when the summary has violations"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("strict_fail") && stderr.contains("failures=1"),
        "strict dogfood should name the summary violation: {stderr}"
    );
}

#[test]
fn every_printed_bounded_expand_executes_and_reveals_the_promised_group() {
    let repo = TempDir::new().expect("bounded expand fixture");
    let cache = TempDir::new().expect("bounded expand cache");
    git(repo.path(), &["init", "-q"]);
    let mut owner = String::new();
    for index in 0..8 {
        owner.push_str(&format!("import {{ dep{index} }} from './dep-{index}';\n"));
        write(
            &repo.path().join(format!("src/dep-{index}.ts")),
            &format!("export const dep{index} = {index};\n"),
        );
        write(
            &repo.path().join(format!("tests/owner-{index}.test.ts")),
            "import { owner } from '../src/owner';\ntest('owner', () => expect(owner).toBeDefined());\n",
        );
    }
    owner.push_str("export const owner = dep0;\n");
    write(&repo.path().join("src/owner.ts"), &owner);

    let bounded = run_markdown(
        repo.path(),
        cache.path(),
        &["cone", "src/owner.ts", "--depth", "1", "--limit", "1"],
    );
    assert!(!bounded.contains("tests/owner-7.test.ts"), "{bounded}");
    let expands = printed_expands(&bounded);
    assert!(!expands.is_empty(), "bounded cone did not print expands: {bounded}");
    for expand in expands {
        let output = execute_printed_expand(repo.path(), cache.path(), &expand);
        assert!(
            output.contains("tests/owner-7.test.ts") && output.contains("src/dep-7.ts"),
            "expand did not reveal the hidden verification and outgoing groups: command={expand}\n{output}"
        );
    }

    let self_cache = TempDir::new().expect("self dogfood expand cache");
    let self_repo = Path::new(env!("CARGO_MANIFEST_DIR"));
    let self_map = run_markdown(
        self_repo,
        self_cache.path(),
        &[
            "cone",
            "src/model/lens_reports.rs",
            "--depth",
            "1",
            "--limit",
            "1",
        ],
    );
    let self_expands = printed_expands(&self_map);
    assert!(!self_expands.is_empty(), "live pilot did not print expands: {self_map}");
    for expand in self_expands {
        let output = execute_printed_expand(self_repo, self_cache.path(), &expand);
        assert!(!output.trim().is_empty(), "live expand was empty: {expand}");
    }
}

fn printed_expands(markdown: &str) -> Vec<String> {
    let mut expands = markdown
        .lines()
        .filter_map(|line| line.split_once("expand: `").map(|(_, tail)| tail))
        .filter_map(|tail| tail.split_once('`').map(|(command, _)| command.to_string()))
        .filter(|command| command.starts_with("codemap "))
        .collect::<Vec<_>>();
    expands.sort();
    expands.dedup();
    expands
}

fn execute_printed_expand(repo: &Path, cache: &Path, command: &str) -> String {
    let command = format!(
        "{}{}",
        env!("CARGO_BIN_EXE_codemap"),
        &command["codemap".len()..]
    );
    let output = Command::new("bash")
        .args(["-c", &command])
        .current_dir(repo)
        .env("CODEMAP_CACHE_DIR", cache)
        .output()
        .expect("printed expand should execute");
    assert!(
        output.status.success(),
        "printed expand failed: {command}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("printed expand utf8")
}
