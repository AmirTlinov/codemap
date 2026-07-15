// Responsibility: cross-owner-ci-execution-path-regressions
#[test]
fn workflow_cone_carries_job_script_deploy_smoke_and_receipt_path() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join(".github/workflows/release.yml"),
        r#"name: release
on: [workflow_dispatch]
jobs:
  promote:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@0123456789abcdef
      - name: Deploy exact workload
        run: node scripts/deploy.mjs --out reports/releases/deploy-receipt.json
      - name: Runtime smoke
        run: ./scripts/smoke.sh
      - name: Write release receipt
        run: |
          node scripts/write-receipt.mjs \
            --out reports/releases/release-receipt.json
"#,
    );
    write(
        &repo.path().join("scripts/deploy.mjs"),
        "spawnSync(\"kubectl\", [\"apply\", \"-f\", \"deploy/prod.yml\"]);\n",
    );
    write(
        &repo.path().join("scripts/smoke.sh"),
        "#!/bin/sh\ncurl -fsS https://example.test/health\n",
    );
    write(
        &repo.path().join("scripts/write-receipt.mjs"),
        "fs.writeFileSync(\"reports/releases/release-receipt.json\", \"{}\");\n",
    );
    write(&repo.path().join("deploy/prod.yml"), "kind: Deployment\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "release path"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            ".github/workflows/release.yml",
            "--depth",
            "2",
            "--all",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &json);
    let edges = json["outgoing"].as_array().expect("outgoing edges");
    assert!(
        edges.iter().all(|edge| edge["from"] != edge["to"]),
        "execution path must not emit self-edges: {json:#}"
    );
    let has = |edge_type: &str, from: &str, to: &str| {
        edges.iter().any(|edge| {
            edge["type"] == edge_type
                && edge["from"].as_str().is_some_and(|value| value.contains(from))
                && edge["to"].as_str().is_some_and(|value| value.contains(to))
        })
    };
    for (edge_type, from, to) in [
        ("declares_job", ".github/workflows/release.yml", "#promote"),
        ("contains_step", "#promote", "Deploy exact workload"),
        ("invokes_script", "Deploy exact workload", "scripts/deploy.mjs"),
        ("invokes_process", "scripts/deploy.mjs", "process:kubectl"),
        ("deploys", "scripts/deploy.mjs", "deployment:kubernetes"),
        ("smoke_checks", "scripts/smoke.sh", "smoke:https://example.test/health"),
        ("produces_receipt", "Write release receipt", "release-receipt.json"),
    ] {
        assert!(
            has(edge_type, from, to),
            "missing {edge_type} {from} -> {to}: {json:#}"
        );
    }
    assert!(
        json["unknowns"].as_array().expect("unknowns").iter().any(|item| {
            item["kind"] == "external_action_execution"
                && item["line_start"] == 7
        }),
        "external action stop must retain its exact location: {json:#}"
    );

    let readable = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cone", ".github/workflows/release.yml", "--depth", "2"])
        .output()
        .expect("readable release cone");
    assert!(readable.status.success());
    let readable = String::from_utf8(readable.stdout).expect("readable utf8");
    for expected in [
        "declares_job ->",
        "contains_step ->",
        "invokes_script -> `scripts/deploy.mjs`",
        "invokes_process -> `process:kubectl`",
        "deploys -> `deployment:kubernetes`",
        "smoke_checks ->",
        "produces_receipt ->",
    ] {
        assert!(
            readable.contains(expected),
            "bounded readable cone lost `{expected}`:\n{readable}"
        );
    }

    let flow = run_json(
        repo.path(),
        cache.path(),
        &[
            "flow",
            ".github/workflows/release.yml",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/flow.schema.json", &flow);
    let flow_kinds = flow["steps"]
        .as_array()
        .expect("flow steps")
        .iter()
        .filter_map(|step| step["kind"].as_str())
        .collect::<BTreeSet<_>>();
    for kind in [
        "declares_job",
        "contains_step",
        "invokes_script",
        "invokes_process",
        "deploys",
        "smoke_checks",
        "produces_receipt",
    ] {
        assert!(
            flow_kinds.contains(kind),
            "flow and cone must project the same workflow facts; missing {kind}: {flow:#}"
        );
    }
}

#[test]
fn heredoc_and_computed_shell_stay_typed_stops_not_fragment_actions() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join(".github/workflows/release.yml"),
        r#"jobs:
  release:
    steps:
      - name: Embedded generator
        run: |
          node <<'NODE'
          console.log('kubectl apply -f invented.yml')
          NODE
          target="$(cat target.txt)"
          "$target"
"#,
    );
    write(&repo.path().join("target.txt"), "./dynamic.sh\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "dynamic boundaries"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            ".github/workflows/release.yml",
            "--all",
            "--format",
            "json",
        ],
    );
    let unknowns = json["unknowns"].as_array().expect("unknowns");
    assert!(
        unknowns
            .iter()
            .any(|item| item["kind"] == "heredoc_execution_boundary")
            && unknowns
                .iter()
                .any(|item| item["kind"] == "computed_shell_execution"),
        "dynamic boundaries must stay explicit: {json:#}"
    );
    assert!(
        !json["outgoing"].as_array().expect("outgoing").iter().any(|edge| {
            edge["to"].as_str().is_some_and(|to| to.contains("invented.yml"))
        }),
        "heredoc text cannot become an execution fact: {json:#}"
    );
}

#[test]
fn workflow_commands_resolve_package_make_and_cargo_targets() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join(".github/workflows/release.yml"),
        r#"jobs:
  release:
    steps:
      - run: npm run deploy
      - run: make smoke
      - run: cargo run --bin ship
"#,
    );
    write(
        &repo.path().join("package.json"),
        r#"{"name":"targets","scripts":{"deploy":"node scripts/deploy.mjs"}}"#,
    );
    write(&repo.path().join("scripts/deploy.mjs"), "console.log('deploy');\n");
    write(&repo.path().join("Makefile"), "smoke:\n\t./scripts/smoke.sh\n");
    write(&repo.path().join("scripts/smoke.sh"), "#!/bin/sh\nexit 0\n");
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"targets\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"ship\"\npath = \"src/bin/ship.rs\"\n",
    );
    write(&repo.path().join("src/bin/ship.rs"), "fn main() {}\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "command targets"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            ".github/workflows/release.yml",
            "--all",
            "--format",
            "json",
        ],
    );
    let edges = json["outgoing"].as_array().expect("outgoing");
    for target in ["script:deploy", "script:smoke", "src/bin/ship.rs"] {
        assert!(
            edges.iter().any(|edge| {
                edge["type"] == "invokes_script"
                    && edge["to"].as_str().is_some_and(|to| to.contains(target))
            }),
            "workflow target `{target}` was not resolved: {json:#}"
        );
    }
    for process in ["process:node", "process:make", "process:cargo"] {
        assert!(
            edges.iter().any(|edge| edge["type"] == "invokes_process" && edge["to"] == process),
            "resolved target did not carry `{process}`: {json:#}"
        );
    }
}
