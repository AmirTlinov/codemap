fn agent_output(repo: &Path, cache: &Path, args: &[&str]) -> std::process::Output {
    codemap()
        .current_dir(repo)
        .env("CODEMAP_CACHE_DIR", cache)
        .args(args)
        .output()
        .expect("codemap agent contract command")
}

fn agent_json(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "agent stdout is not JSON: {error}; stderr={}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[test]
fn agent_contract_executes_argv_expand_and_distinguishes_empty_from_invalid() {
    let (repo, cache) = fixture();
    let empty = agent_output(
        repo.path(),
        cache.path(),
        &["where", "__missing_agent_symbol__", "--format", "json"],
    );
    assert_eq!(empty.status.code(), Some(10));
    assert!(empty.stderr.is_empty());
    let empty = agent_json(&empty);
    assert_eq!(empty["agent"]["result"], "valid_empty_map");
    assert_schema("schemas/where.schema.json", &empty);

    let invalid = agent_output(
        repo.path(),
        cache.path(),
        &["ls", "missing/path.ts", "--format", "json"],
    );
    assert_eq!(invalid.status.code(), Some(20));
    assert!(invalid.stderr.is_empty());
    let invalid = agent_json(&invalid);
    assert_eq!(invalid["agent"]["result"], "invalid_anchor");
    assert_schema("schemas/ls.schema.json", &invalid);

    let first = agent_output(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/replay/src/session.ts",
            "--format",
            "json",
        ],
    );
    assert_eq!(first.status.code(), Some(0));
    let first = agent_json(&first);
    let argv = first["agent"]["expands"][0]
        .as_array()
        .expect("machine argv expand");
    assert_eq!(argv[0], "codemap");
    assert!(argv.iter().any(|word| word == "--format"));
    let expanded = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(argv.iter().skip(1).map(|word| word.as_str().unwrap()))
        .output()
        .expect("execute argv expand");
    assert!(matches!(expanded.status.code(), Some(0 | 10 | 20)));
    assert_eq!(agent_json(&expanded)["agent"]["envelope_version"], "1");
}

#[test]
fn agent_contract_exit_taxonomy_keeps_reports_on_stdout_and_diagnostics_on_stderr() {
    let (repo, cache) = fixture();
    let unsupported = agent_output(
        repo.path(),
        cache.path(),
        &["graph", "--lens", "not-a-lens", "--format", "json"],
    );
    assert_eq!(unsupported.status.code(), Some(21));
    assert!(unsupported.stdout.is_empty());
    assert!(String::from_utf8_lossy(&unsupported.stderr).contains("unsupported_request"));

    let diagnostic = agent_output(
        repo.path(),
        cache.path(),
        &["boundaries", "--format", "json"],
    );
    assert_eq!(diagnostic.status.code(), Some(22));
    assert_schema("schemas/boundaries.schema.json", &agent_json(&diagnostic));
    assert!(String::from_utf8_lossy(&diagnostic.stderr).contains("diagnostic_failure"));

    let clean_check = agent_output(
        repo.path(),
        cache.path(),
        &["boundaries", "--changed", "--format", "json"],
    );
    assert_eq!(clean_check.status.code(), Some(0));
    assert_eq!(agent_json(&clean_check)["agent"]["result"], "success");

    let unsafe_cache = repo.path().join("unsafe-cache");
    let unsafe_output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", &unsafe_cache)
        .args(["cache", "clear", "--yes", "--format", "json"])
        .output()
        .expect("unsafe cache refusal");
    assert_eq!(unsafe_output.status.code(), Some(23));
    assert!(unsafe_output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&unsafe_output.stderr).contains("unsafe_execution_refused"));

    let invalid = agent_output(repo.path(), cache.path(), &["proof", "--format", "json"]);
    assert_eq!(invalid.status.code(), Some(20));
    assert!(invalid.stdout.is_empty());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("invalid_input"));
}

#[test]
fn agent_contract_paths_cover_spaces_unicode_subdirectories_symlinks_and_windows_separators() {
    let base = TempDir::new().expect("base tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    let repo = base.path().join("repo with spaces Карта");
    fs::create_dir_all(repo.join("src/nested")).expect("repo dirs");
    git(&repo, &["init", "-q"]);
    write(&repo.join("package.json"), r#"{"name":"portable-agent-path"}"#);
    write(
        &repo.join("src/данные файл.ts"),
        "export const portable = true;\n",
    );
    git(&repo, &["add", "."]);
    git(&repo, &["-c", "user.email=a@example.com", "-c", "user.name=a", "commit", "-qm", "fixture"]);

    let report = codemap()
        .current_dir(repo.join("src/nested"))
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args([
            "--root",
            repo.to_str().unwrap(),
            "ls",
            r"src\данные файл.ts",
            "--format",
            "json",
        ])
        .output()
        .expect("portable path report");
    assert_eq!(report.status.code(), Some(0));
    let report = agent_json(&report);
    assert_eq!(report["anchor"]["path"], "src/данные файл.ts");

    #[cfg(unix)]
    {
        let link = base.path().join("repo-link");
        std::os::unix::fs::symlink(&repo, &link).expect("repo symlink");
        let linked = codemap()
            .current_dir(base.path())
            .env("CODEMAP_CACHE_DIR", cache.path())
            .args(["--root", link.to_str().unwrap(), "ls", ".", "--format", "json"])
            .output()
            .expect("symlink root report");
        assert_eq!(linked.status.code(), Some(0));
        assert_schema("schemas/ls.schema.json", &agent_json(&linked));
    }
    assert!(!repo.join(".codemap-cache").exists());
}

#[test]
fn agent_contract_completions_are_repo_independent_and_cover_supported_shells() {
    let cwd = TempDir::new().expect("completion cwd");
    for shell in ["bash", "zsh", "fish", "powershell", "elvish"] {
        let output = codemap()
            .current_dir(cwd.path())
            .args(["completions", shell])
            .output()
            .expect("completion output");
        assert_eq!(output.status.code(), Some(0), "{shell}");
        assert!(!output.stdout.is_empty(), "{shell}");
        assert!(output.stderr.is_empty(), "{shell}");
    }
    assert_eq!(fs::read_dir(cwd.path()).unwrap().count(), 0);
}

#[test]
fn agent_contract_manifest_owns_protocol_and_report_envelopes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest: Value = serde_json::from_slice(
        &fs::read(root.join("schemas/manifest.json")).expect("schema manifest"),
    )
    .expect("manifest JSON");
    assert_eq!(manifest["agent_protocol"]["version"], 1);
    assert_eq!(manifest["agent_protocol"]["exit_codes"]["valid_empty_map"], 10);
    assert_eq!(manifest["agent_protocol"]["exit_codes"]["internal_error"], 70);
    for entry in manifest["schemas"].as_array().expect("schema entries") {
        if entry["contract"] == "semantic_anchor_config" {
            continue;
        }
        let schema: Value = serde_json::from_slice(
            &fs::read(root.join(entry["file"].as_str().unwrap())).expect("report schema"),
        )
        .expect("report schema JSON");
        assert!(
            schema["required"]
                .as_array()
                .is_some_and(|fields| fields.iter().any(|field| field == "agent")),
            "{} must require agent envelope",
            entry["kind"]
        );
        assert_eq!(schema["properties"]["agent"]["$ref"], "#/$defs/agent_envelope");
        assert_eq!(
            schema["$defs"]["agent_envelope"]["properties"]["envelope_version"]["const"],
            "1"
        );
    }
}

#[test]
fn agent_contract_python_harness_independently_validates_and_executes_expand() {
    let (repo, cache) = fixture();
    let report = agent_output(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/replay/src/session.ts",
            "--format",
            "json",
        ],
    );
    assert_eq!(report.status.code(), Some(0));
    let report_path = cache.path().join("agent-report.json");
    fs::write(&report_path, &report.stdout).expect("agent report fixture");
    let empty = agent_output(
        repo.path(),
        cache.path(),
        &["where", "__python_missing__", "--format", "json"],
    );
    let invalid = agent_output(
        repo.path(),
        cache.path(),
        &["ls", "missing/python.ts", "--format", "json"],
    );
    let empty_path = cache.path().join("empty-report.json");
    let invalid_path = cache.path().join("invalid-report.json");
    fs::write(&empty_path, &empty.stdout).expect("empty report fixture");
    fs::write(&invalid_path, &invalid.stdout).expect("invalid report fixture");
    let protocol = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/codemap_protocol.py");
    let probe = r#"import json, os, pathlib, runpy, subprocess, sys
m = runpy.run_path(sys.argv[1])
report = json.loads(pathlib.Path(sys.argv[2]).read_text())
argv = m["next_expand_argv"](report)
argv[0] = sys.argv[3]
env = dict(os.environ, CODEMAP_CACHE_DIR=sys.argv[5])
out = subprocess.run(argv, cwd=sys.argv[4], env=env, capture_output=True)
expanded = json.loads(out.stdout)
m["validate_agent_report"](expanded, out.returncode)
m["validate_agent_report"](json.loads(pathlib.Path(sys.argv[6]).read_text()), 10)
m["validate_agent_report"](json.loads(pathlib.Path(sys.argv[7]).read_text()), 20)
print(expanded["agent"]["report_kind"])
"#;
    let python = if cfg!(windows) { "python" } else { "python3" };
    let output = Command::new(python)
        .args([
            "-c",
            probe,
            protocol.to_str().unwrap(),
            report_path.to_str().unwrap(),
            env!("CARGO_BIN_EXE_codemap"),
            repo.path().to_str().unwrap(),
            cache.path().to_str().unwrap(),
            empty_path.to_str().unwrap(),
            invalid_path.to_str().unwrap(),
        ])
        .output()
        .expect("python agent harness");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.stdout.is_empty());
}
