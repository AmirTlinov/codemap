#[test]
fn daily_workflow_markdown_stays_compact_and_non_ritualistic() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/session.ts"),
        "import { Timeline } from './timeline';\nimport type { FrameDto } from './types';\n\nexport function seek(cursor: number): FrameDto {\n  return { frame: new Timeline().frameAt(cursor + 2) };\n}\n",
    );

    let cases = [
        ("ls root", vec!["ls", "."], 120),
        (
            "cone anchor",
            vec!["cone", "packages/replay/src/session.ts"],
            120,
        ),
        ("changed", vec!["changed"], 120),
        ("proof changed", vec!["proof", "changed"], 120),
    ];
    for (name, args, max_lines) in cases {
        let markdown = run_markdown(repo.path(), cache.path(), &args);
        assert!(
            markdown.lines().count() <= max_lines,
            "{name} exceeded the daily markdown line budget of {max_lines}: {markdown}"
        );
        assert_no_daily_table_spam(name, &markdown);
        assert_no_forbidden_product_language(name, &markdown);
        assert_hidden_sections_have_expand(name, &markdown);
    }

    let proof = run_markdown(repo.path(), cache.path(), &["proof", "changed"]);
    assert!(
        proof.matches("vitest run").count() <= 1,
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
        &repo.path().join("scripts/dogfood-codemap.sh"),
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
        markdown.contains("scripts/dogfood-codemap.sh") && !markdown.contains("[missing; unknown"),
        "script changes should stay first-class in compact changed output: {markdown}"
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
        siblings.contains("## Proof Sensors"),
        "siblings markdown should use neutral source-backed proof sensor wording: {siblings}"
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
        markdown.contains("## Soft Evidence")
            && markdown.contains("script_surface_match")
            && !markdown.contains("## Proof Sensors"),
        "script/path overlap should render as soft evidence, not as deterministic proof: {markdown}"
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
        output.status.success(),
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
