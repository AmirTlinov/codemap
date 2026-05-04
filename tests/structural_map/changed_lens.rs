#[test]
fn changed_combines_delta_impact_and_proof_without_running_commands() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/session.ts"),
        "import { Timeline } from './timeline';\nimport type { FrameDto } from './types';\n\nexport function seek(cursor: number): FrameDto {\n  return { frame: new Timeline().frameAt(cursor + 1) };\n}\n\nexport function seekLabel() {\n  return 'seek';\n}\n",
    );

    let changed = run_json(repo.path(), cache.path(), &["changed", "--format", "json"]);
    assert_schema("schemas/changed.schema.json", &changed);
    assert_eq!(changed["kind"], "changed_report");
    assert_eq!(changed["schema_version"], "6");
    assert!(
        changed["changed"]
            .as_array()
            .expect("changed anchors")
            .iter()
            .any(|file| file["path"] == "packages/replay/src/session.ts"),
        "changed should include the touched anchor: {changed:#}"
    );
    assert!(
        changed["map_delta"]["changed_symbols"]
            .as_u64()
            .unwrap_or_default()
            >= 1,
        "changed should summarize map delta from diff-map facts: {changed:#}"
    );
    assert!(
        !changed["impact"].as_array().expect("impact").is_empty(),
        "changed should include structural impact clusters: {changed:#}"
    );
    assert!(
        !changed["proof"]["commands"]
            .as_array()
            .expect("proof commands")
            .is_empty()
            || !changed["proof"]["fallback"]
                .as_array()
                .expect("fallback")
                .is_empty(),
        "changed should include proof overview without running commands: {changed:#}"
    );
    assert!(
        changed["expand"]
            .as_array()
            .expect("expand")
            .iter()
            .any(|command| command == "codemap changed --section proof"
                || command == "codemap changed --files packages/replay/src/session.ts --section proof"),
        "changed should provide deterministic proof drill-down: {changed:#}"
    );
    for expected in [
        "codemap changed --section observed",
        "codemap changed --section links",
        "codemap changed --section roles",
        "codemap changed --section unknown",
    ] {
        assert!(
            changed["expand"]
                .as_array()
                .expect("expand")
                .iter()
                .any(|command| command == expected),
            "changed expand should expose RFC section `{expected}`: {changed:#}"
        );
    }
}

#[test]
fn changed_section_filters_use_stable_rfc_names() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/session.ts"),
        "import { Timeline } from './timeline';\n\nexport function seek(cursor: number) {\n  return new Timeline().frameAt(cursor + 3);\n}\n",
    );

    for (section, heading) in [
        ("observed", "## Observed"),
        ("links", "## Links"),
        ("roles", "## Surface Hints"),
        ("proof", "## Proof"),
        ("unknown", "## Unknown"),
        ("hidden", "## Hidden"),
    ] {
        let output = codemap()
            .current_dir(repo.path())
            .env("CODEMAP_CACHE_DIR", cache.path())
            .args(["changed", "--section", section])
            .output()
            .expect("changed section should run");
        assert!(
            output.status.success(),
            "changed --section {section} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
        assert!(
            markdown.contains(heading),
            "changed --section {section} should render stable section heading `{heading}`: {markdown}"
        );
    }

    let legacy_alias = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--section", "diff"])
        .output()
        .expect("changed legacy section alias should run");
    assert!(
        legacy_alias.status.success(),
        "legacy diff alias should remain accepted but hidden: {}",
        String::from_utf8_lossy(&legacy_alias.stderr)
    );
    let markdown = String::from_utf8(legacy_alias.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("## Observed"),
        "legacy diff alias should map to observed facts: {markdown}"
    );
}

#[test]
fn changed_staged_selector_keeps_full_worktree_summary() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/staged.ts"),
        "export const staged = true;\n",
    );
    git(repo.path(), &["add", "packages/replay/src/staged.ts"]);
    write(
        &repo.path().join("packages/replay/src/session.ts"),
        "import { Timeline } from './timeline';\n\nexport function seek(cursor: number) {\n  return new Timeline().frameAt(cursor + 9);\n}\n",
    );
    write(
        &repo.path().join("packages/replay/src/untracked.ts"),
        "export const untracked = true;\n",
    );

    let changed = run_json(repo.path(), cache.path(), &["changed", "--staged", "--format", "json"]);
    assert_schema("schemas/changed.schema.json", &changed);
    assert_eq!(changed["total_changed_count"].as_u64(), Some(1));
    assert_eq!(changed["prelude"]["worktree"]["staged"].as_u64(), Some(1));
    assert_eq!(changed["prelude"]["worktree"]["unstaged"].as_u64(), Some(1));
    assert_eq!(changed["prelude"]["worktree"]["untracked"].as_u64(), Some(1));

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--staged"])
        .output()
        .expect("changed staged markdown should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("\n## Worktree\n")
            && markdown.contains("- selector: `--staged`")
            && markdown.contains("- selected files: `1`")
            && markdown.contains("- staged: `1`")
            && markdown.contains("- unstaged: `1`")
            && markdown.contains("- untracked: `1`")
            && markdown.contains("Worktree counts show current repo state; selected files show `--staged`."),
        "staged changed output should keep full live worktree summary: {markdown}"
    );
}

#[test]
fn changed_risks_and_coupling_are_mechanical_not_recommendations() {
    let (repo, cache) = fixture();
    write(&repo.path().join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "lock baseline"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"map-fixture","private":true,"workspaces":["packages/*"],"scripts":{"test":"pnpm test","typecheck":"tsc -b","verify":"node scripts/verify.js"}}"#,
    );
    write(&repo.path().join("AGENTS.md"), "# local instructions\n");
    write(&repo.path().join("models/tiny.gguf"), "not a real model\n");
    write(
        &repo.path().join("packages/replay/src/no-proof.ts"),
        "export const noProof = true;\n",
    );

    let changed = run_json(repo.path(), cache.path(), &["changed", "--format", "json"]);
    assert_schema("schemas/changed.schema.json", &changed);
    let risk_kinds = changed["risks"]
        .as_array()
        .expect("risks")
        .iter()
        .map(|risk| risk["kind"].as_str().unwrap_or_default())
        .collect::<BTreeSet<_>>();
    for expected in [
        "instruction_file_changed",
        "manifest_without_lockfile_change",
        "model_weight_like_changed",
        "protected_looking_path_changed",
        "unknown_direct_proof",
    ] {
        assert!(
            risk_kinds.contains(expected),
            "changed risks should include `{expected}` as mechanical facts: {changed:#}"
        );
    }
    assert!(
        changed["coupling"]
            .as_array()
            .expect("coupling")
            .iter()
            .any(|fact| fact["kind"] == "source_has_direct_or_declared_proof_surface"
                && fact["status"] == "no"),
        "changed coupling should keep missing direct proof as a relationship fact: {changed:#}"
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .arg("changed")
        .output()
        .expect("changed markdown should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("\n## Risks\n")
            && markdown.contains("Mechanical facts only. Not an edit verdict.")
            && markdown.contains("\n## Coupling\n")
            && markdown.contains("Deterministic relationship facts only.")
            && !markdown.contains("recommended")
            && !markdown.contains("recommendation")
            && !markdown.contains("advice")
            && !markdown.contains("best file")
            && !markdown.contains("trusting edits")
            && !markdown.contains("require source provenance"),
        "changed markdown should expose risks/coupling without advice wording: {markdown}"
    );
}

#[test]
fn changed_inherits_package_export_contract_impact() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/package.json"),
        r#"{
  "name": "@fixture/replay",
  "version": "1.0.1",
  "main": "src/index.ts",
  "exports": { ".": "./src/index.ts" },
  "scripts": { "test": "vitest run" }
}
"#,
    );

    let changed = run_json(repo.path(), cache.path(), &["changed", "--format", "json"]);
    assert_schema("schemas/changed.schema.json", &changed);
    assert!(
        changed["impact"]
            .as_array()
            .expect("impact")
            .iter()
            .any(|cluster| cluster["changed"]
                .as_array()
                .expect("cluster changed")
                .iter()
                .any(|path| path == "packages/replay/package.json")
                && cluster["contract_links"]
                    .as_array()
                    .expect("contract links")
                    .iter()
                    .any(|edge| edge["from"] == "packages/replay/package.json"
                        && edge["to"] == "packages/replay/package.json"
                        && edge["type"] == "package_export"
                        && edge["evidence"] == "package_manifest")),
        "daily changed overview should inherit package export contract impact: {changed:#}"
    );
}

#[test]
fn changed_reports_clean_state_calmly() {
    let (repo, cache) = fixture();

    let changed = run_json(repo.path(), cache.path(), &["changed", "--format", "json"]);
    assert_schema("schemas/changed.schema.json", &changed);
    assert!(
        changed["changed"].as_array().expect("changed").is_empty(),
        "clean repo should have no changed anchors: {changed:#}"
    );
    assert!(
        changed["impact"].as_array().expect("impact").is_empty(),
        "clean repo should have no impact clusters: {changed:#}"
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .arg("changed")
        .output()
        .expect("changed markdown should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("No changed anchors detected."),
        "clean changed markdown should be calm: {markdown}"
    );
}

#[test]
fn changed_distinguishes_visible_and_total_changed_files() {
    let (repo, cache) = fixture();
    for index in 0..5 {
        write(
            &repo
                .path()
                .join(format!("packages/replay/src/extra-{index}.ts")),
            &format!("export const extra{index} = {index};\n"),
        );
    }

    let changed = run_json(
        repo.path(),
        cache.path(),
        &["changed", "--limit", "2", "--format", "json"],
    );
    assert_schema("schemas/changed.schema.json", &changed);
    assert_eq!(
        changed["total_changed_count"].as_u64(),
        Some(5),
        "changed should expose total changed anchors separately from visible rows: {changed:#}"
    );
    assert_eq!(
        changed["changed"].as_array().expect("visible changed").len(),
        2,
        "changed should keep markdown/json visible rows bounded: {changed:#}"
    );
    assert!(
        changed["git_state"]
            .as_array()
            .expect("git state")
            .iter()
            .all(|entry| entry["status"] == "untracked"),
        "untracked files should be typed git-state facts: {changed:#}"
    );
    assert!(
        changed["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|group| group["reason"]
                .as_str()
                .unwrap_or_default()
                .contains("changed file summaries hidden by limit")
                && group["count"].as_u64() == Some(3)),
        "hidden changed summaries should account for truncated rows: {changed:#}"
    );
    assert!(
        !changed["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|group| group["reason"] == "git state rows hidden by limit"),
        "JSON keeps complete git_state, so render-only Git State truncation should not appear as report hidden: {changed:#}"
    );
    assert_eq!(
        changed["git_state"].as_array().expect("git state").len(),
        5,
        "JSON should keep full git state even when markdown is bounded: {changed:#}"
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--limit", "2"])
        .output()
        .expect("changed markdown should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("Changed: `2` shown / `5` total files"),
        "markdown should not under-report truncated changed anchors: {markdown}"
    );
    assert_eq!(
        markdown.matches("[untracked; staged=false; unstaged=true]").count(),
        2,
        "markdown should bound Git State rows by the changed limit: {markdown}"
    );
    assert!(
        markdown.contains("- changed symbols: `5`") || markdown.contains("- added exports: `5`"),
        "markdown should render Map Delta as compact count bullets: {markdown}"
    );
    assert!(
        markdown.contains("[direct=") && !markdown.contains("| Cluster | Risk | Reasons | Edges |"),
        "changed markdown should render impact summary as compact cluster bullets: {markdown}"
    );
    assert!(
        !markdown.contains("| Status | Path | Old | Staged | Unstaged |")
            && !markdown.contains("| Surface | Count |")
            && !markdown.contains("| Kind | Count |"),
        "changed markdown should not return to Git State, Map Delta, or proof sensor table spam: {markdown}"
    );
    assert!(
        markdown.contains("\n### Sensor Counts\n") && markdown.contains("- runnable_direct: `"),
        "changed proof summary should render public sensor counts as compact bullets: {markdown}"
    );
    assert!(
        markdown.contains("git state rows hidden by limit"),
        "markdown should expose hidden Git State rows with expand: {markdown}"
    );
    assert!(
        !markdown.contains("--files "),
        "changed markdown hidden expands should not dump long selected-file lists: {markdown}"
    );
}
