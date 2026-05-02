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
        ("proof changed", vec!["proof", "--changed"], 120),
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

    let proof = run_markdown(repo.path(), cache.path(), &["proof", "--changed"]);
    assert!(
        proof.matches("vitest run").count() <= 1,
        "proof should group sensors by command instead of repeating the same command: {proof}"
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
