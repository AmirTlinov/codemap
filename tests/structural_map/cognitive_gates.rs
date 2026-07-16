#[test]
fn daily_workflow_markdown_stays_compact_and_non_ritualistic() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/session.ts"),
        "import { Timeline } from './timeline';\nimport type { FrameDto } from './types';\n\nexport function seek(cursor: number): FrameDto {\n  return { frame: new Timeline().frameAt(cursor + 2) };\n}\n",
    );

    let cases = [
        ("ls root", vec!["ls", "."], 120, 1500),
        (
            "ls exact file",
            vec!["ls", "packages/replay/src/session.ts"],
            90,
            1100,
        ),
        (
            "cone anchor",
            vec!["cone", "packages/replay/src/session.ts"],
            90,
            1100,
        ),
        ("where symbol", vec!["where", "seek"], 60, 700),
        ("changed", vec!["changed"], 120, 2000),
        ("proof changed", vec!["proof", "changed"], 90, 1200),
    ];
    for (name, args, max_lines, max_tokens) in cases {
        let markdown = run_markdown(repo.path(), cache.path(), &args);
        assert!(
            markdown.lines().count() <= max_lines,
            "{name} exceeded the daily markdown line budget of {max_lines}: {markdown}"
        );
        let approximate_tokens = markdown.chars().count().div_ceil(4);
        assert!(
            approximate_tokens <= max_tokens,
            "{name} exceeded the approximate token budget of {max_tokens}: {approximate_tokens}\n{markdown}"
        );
        assert_no_daily_table_spam(name, &markdown);
        assert_no_forbidden_product_language(name, &markdown);
        assert_hidden_sections_have_expand(name, &markdown);
    }

    let proof = run_markdown(repo.path(), cache.path(), &["proof", "changed"]);
    // `Most-Direct Commands` deliberately echoes direct commands as a top-of-output
    // summary; the detailed Runnable section must still group sensors by command
    // instead of repeating one.
    let runnable_detail = proof
        .split_once("## Runnable Command Surfaces")
        .map(|(_, rest)| rest)
        .unwrap_or(proof.as_str());
    assert!(
        runnable_detail.matches("vitest run").count() <= 1,
        "proof should group sensors by command instead of repeating the same command: {proof}"
    );
}

#[test]
fn changed_large_schema_and_script_slice_stays_compact() {
    let (repo, cache) = fixture();
    let schema_body = (0..36)
        .map(|index| format!(r#""field{index}": {{"type": "string"}}"#))
        .collect::<Vec<_>>()
        .join(",\n");
    write(
        &repo.path().join("schemas/large.schema.json"),
        &format!(
            "{{\n  \"$schema\": \"https://json-schema.org/draft/2020-12/schema\",\n  \"type\": \"object\",\n  \"properties\": {{\n{schema_body}\n  }}\n}}\n"
        ),
    );
    write(
        &repo.path().join("scripts/check-changed.sh"),
        "#!/usr/bin/env bash\nset -euo pipefail\ncodemap changed\n",
    );
    for index in 0..24 {
        write(
            &repo
                .path()
                .join(format!("packages/replay/src/generated-{index}.ts")),
            &format!("export const generated{index} = {index};\n"),
        );
    }

    let markdown = run_markdown(repo.path(), cache.path(), &["changed"]);
    assert!(
        markdown.lines().count() <= 120,
        "large changed overview should stay compact and push detail into expand: {markdown}"
    );
    assert!(
        markdown.contains("hidden changed anchors") && markdown.contains("codemap changed --section observed"),
        "large changed overview should expose exact detail expansion: {markdown}"
    );
    assert!(
        markdown.contains("scripts/check-changed.sh") && !markdown.contains("[missing; unknown"),
        "script changes should stay first-class in compact changed output: {markdown}"
    );
}

#[test]
fn changed_self_dogfood_shape_compacts_roles_and_schema_events() {
    let (repo, cache) = fixture();
    for index in 0..30 {
        write(
            &repo.path().join(format!("schemas/public-{index}.schema.json")),
            r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "name": { "type": "string" }
  }
}
"#,
        );
    }
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "schema baseline"]);

    for index in 0..30 {
        write(
            &repo.path().join(format!("schemas/public-{index}.schema.json")),
            r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "name": { "type": "string" },
    "target_anchor": { "type": "string" }
  }
}
"#,
        );
    }
    for (path, body) in [
        (
            "Cargo.toml",
            "[package]\nname = \"wide-shape\"\nversion = \"0.1.0\"\n",
        ),
        ("Cargo.lock", "# generated lock\n"),
        ("src/map/proof_surface.rs", "pub fn proof_surface() {}\n"),
        ("src/repo/roles.rs", "pub fn classify_roles() {}\n"),
        ("src/render/changed.rs", "pub fn render_changed() {}\n"),
        ("src/cli/args.rs", "pub fn cli_args() {}\n"),
        ("src/map/cone_env_surfaces.rs", "pub fn extract_env() {}\n"),
        ("src/repo/constants.rs", "pub const CONFIG: &str = \"x\";\n"),
        ("src/map/proof_owner_ci_script_body.rs", "pub fn script_catalog() {}\n"),
        ("tests/structural_map/wide_shape.rs", "#[test]\nfn wide_shape() {}\n"),
        ("scripts/check-version-bump.py", "#!/usr/bin/env python3\nraise SystemExit(0)\n"),
        (".github/workflows/ci.yml", "name: ci\n"),
        ("docs/guide.md", "# Guide\n"),
        ("runtime/receipts/proof.json", "{\"receipt\":true}\n"),
        ("fixtures/archive/sample.txt", "fixture\n"),
    ] {
        write(&repo.path().join(path), body);
    }

    let markdown = run_markdown(repo.path(), cache.path(), &["changed"]);
    let line_count = markdown.lines().count();
    assert!(
        line_count <= 120,
        "wide changed overview should stay inside dogfood budget; lines={line_count}\n{markdown}"
    );
    assert!(
        markdown.contains("hidden surface hint groups")
            && markdown.contains("codemap changed --section roles"),
        "wide changed overview should compact role groups with an exact expand: {markdown}"
    );
    assert!(
        markdown.contains("`added_schema_field`")
            && markdown.contains("count=30")
            && markdown.contains("codemap changed --section observed"),
        "wide changed overview should group repeated schema-field events without losing expand: {markdown}"
    );
    assert!(
        markdown.contains("mechanical groups:")
            && markdown.contains("deterministic groups:")
            && markdown.contains("changed --section observed")
            && markdown.contains("changed --section links"),
        "45-file default view should summarize mechanical facts behind exact section expands: {markdown}"
    );
}

#[test]
fn public_markdown_does_not_leak_internal_role_evidence_labels() {
    let (repo, cache) = fixture();
    let contract = run_markdown(repo.path(), cache.path(), &["contract", "package.json"]);
    assert!(
        !contract.contains("role:") && contract.contains("surface_hint:"),
        "contract markdown should render deterministic surface hints without internal role labels: {contract}"
    );

    let siblings = run_markdown(
        repo.path(),
        cache.path(),
        &["siblings", "packages/replay/src", "--limit", "40"],
    );
    for forbidden in ["Proof Pattern", "Paired Proof Pattern", "role_script_target", "role:"] {
        assert!(
            !siblings.contains(forbidden),
            "siblings markdown leaked old trust-boundary vocabulary `{forbidden}`: {siblings}"
        );
    }
    assert!(
        siblings.contains("## Verification Sensors"),
        "siblings markdown should use neutral source-backed verification sensor wording: {siblings}"
    );
}

#[test]
fn soft_script_matches_render_under_soft_evidence() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"soft-script-siblings\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(
        &repo.path().join(".storybook/main.ts"),
        "export default { stories: [] };\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "soft script fixture"]);

    let markdown = run_markdown(repo.path(), cache.path(), &["siblings", ".storybook"]);
    assert!(
        markdown.contains("## Soft Surface Matches")
            && markdown.contains("script_surface_match")
            && !markdown.contains("## Verification Sensors"),
        "script/path overlap should render as soft surface matches, not as runnable verification: {markdown}"
    );
}

#[test]
fn changed_existing_unindexed_files_are_not_rendered_as_missing() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"unindexed-output","private":true}"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "baseline"]);
    write(
        &repo.path().join("banner_semantics_py.out"),
        "existing witness output\n",
    );

    let markdown = run_markdown(
        repo.path(),
        cache.path(),
        &["changed", "--files", "banner_semantics_py.out"],
    );
    assert!(
        markdown.contains("`banner_semantics_py.out` [unindexed; unknown")
            && !markdown.contains("`banner_semantics_py.out` [missing; unknown"),
        "existing-but-unindexed anchors should not be called missing: {markdown}"
    );
}

fn run_markdown(repo: &Path, cache: &Path, args: &[&str]) -> String {
    let output = codemap()
        .current_dir(repo)
        .env("CODEMAP_CACHE_DIR", cache)
        .args(args)
        .output()
        .expect("codemap markdown should run");
    assert!(
        matches!(output.status.code(), Some(0 | 10 | 20)) && !output.stdout.is_empty(),
        "codemap {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("markdown utf8")
}

fn assert_no_daily_table_spam(name: &str, markdown: &str) {
    for forbidden in [
        "| Field | Value |",
        "| From | Type | To | Evidence | Strength | Where |",
        "| Status | Path | Old | Staged | Unstaged |",
        "| Surface | Count |",
        "| Kind | Count |",
        "| Command | Sensor | Evidence |",
    ] {
        assert!(
            !markdown.contains(forbidden),
            "{name} returned table spam `{forbidden}`: {markdown}"
        );
    }
}

fn assert_no_forbidden_product_language(name: &str, markdown: &str) {
    let lower = markdown.to_ascii_lowercase();
    for forbidden in [
        "safe_to_delete",
        "safe to delete",
        "probably unused",
        "recommended",
        "best file",
        "source_of_truth",
        "read_first",
        "confidence",
    ] {
        assert!(
            !lower.contains(forbidden),
            "{name} leaked forbidden product language `{forbidden}`: {markdown}"
        );
    }
}

fn assert_hidden_sections_have_expand(name: &str, markdown: &str) {
    if let Some((_, hidden_section)) = markdown.split_once("\n## Hidden\n") {
        let hidden_before_next_section = hidden_section
            .split("\n## ")
            .next()
            .unwrap_or(hidden_section);
        assert!(
            hidden_before_next_section.contains("expand: `codemap "),
            "{name} has a hidden section without an executable expand command: {markdown}"
        );
    }
}
