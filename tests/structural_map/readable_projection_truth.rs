// Responsibility: readable where/cone projection truth and compactness

#[test]
fn unique_where_prints_consumers_once_and_uses_remaining_incoming_in_xray() {
    let (repo, cache) = readable_projection_repo("where-disjoint");
    write(
        &repo.path().join("src/a-target.ts"),
        "export function target() {\n  return 1;\n}\n\nexport function localA() {\n  return target();\n}\n\nexport function localB() {\n  return target();\n}\n\nexport function localC() {\n  return target();\n}\n",
    );
    for index in 0..4 {
        write(
            &repo.path().join(format!("src/z-consumer-{index}.ts")),
            &format!(
                "import {{ target }} from './a-target';\nexport function use{index}() {{ return target(); }}\n"
            ),
        );
    }
    commit_projection_fixture(repo.path(), "where disjoint fixture");

    let markdown = run_markdown(
        repo.path(),
        cache.path(),
        &["where", "target", "--limit", "2"],
    );
    assert!(
        markdown.contains("consumers: counted(4); shown=2 hidden=2"),
        "consumer horizon must describe the rendered preview: {markdown}"
    );
    assert!(
        !markdown.contains("aliases: @anchor"),
        "short symbol paths must retain the established readable shape: {markdown}"
    );
    assert!(
        markdown.contains("incoming: counted-at-least(7,")
            && markdown.contains("shown=4 hidden=3"),
        "incoming horizon must count consumer facts plus disjoint x-ray facts: {markdown}"
    );
    assert!(
        markdown.contains("Consumers:")
            && markdown.contains("z-consumer-0.ts")
            && markdown.contains("z-consumer-1.ts")
            && markdown.contains("a-target.ts#localA")
            && markdown.contains("a-target.ts#localB"),
        "single where must print selected consumers and remaining incoming facts: {markdown}"
    );
    let fact_lines = markdown
        .lines()
        .filter(|line| line.contains("--") && line.contains("-->"))
        .collect::<Vec<_>>();
    let unique = fact_lines
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        fact_lines.len(),
        unique.len(),
        "where must not print the same structural edge twice: {markdown}"
    );
}

#[test]
fn cone_section_visibility_names_only_groups_rendered_by_that_section() {
    let (repo, cache) = readable_projection_repo("section-visibility");
    write(
        &repo.path().join("src/target.ts"),
        "export function target() { return 1; }\n",
    );
    write(
        &repo.path().join("src/use.ts"),
        "import { target } from './target';\nexport const value = target();\n",
    );
    write(
        &repo.path().join("tests/target.test.ts"),
        "import { target } from '../src/target';\ntest('target', () => target());\n",
    );
    commit_projection_fixture(repo.path(), "section visibility fixture");
    let anchor = "src/target.ts#target";

    for section in [None, Some("observed")] {
        let mut args = vec!["cone", anchor];
        if let Some(section) = section {
            args.extend(["--section", section]);
        }
        let markdown = run_markdown(repo.path(), cache.path(), &args);
        assert!(
            markdown.contains("- incoming:") && markdown.contains("- verification:"),
            "default/observed renders both groups: {markdown}"
        );
    }
    let links = run_markdown(
        repo.path(),
        cache.path(),
        &["cone", anchor, "--section", "links"],
    );
    assert!(links.contains("- incoming:"), "{links}");
    assert!(!links.contains("- verification:"), "{links}");
    let proof = run_markdown(
        repo.path(),
        cache.path(),
        &["cone", anchor, "--section", "proof"],
    );
    assert!(proof.contains("- verification:"), "{proof}");
    assert!(!proof.contains("- incoming:"), "{proof}");

    for section in ["roles", "hidden", "unknown"] {
        let markdown = run_markdown(
            repo.path(),
            cache.path(),
            &["cone", anchor, "--section", section],
        );
        assert!(
            !markdown.contains("## Visibility")
                && !markdown.contains("- incoming:")
                && !markdown.contains("- verification:"),
            "{section} must not claim unrelated visible groups: {markdown}"
        );
    }
}

#[test]
fn cone_and_where_reject_zero_limit_without_panicking() {
    let (repo, cache) = readable_projection_repo("positive-limit");
    write(
        &repo.path().join("src/target.ts"),
        "export function target() { return 1; }\n",
    );
    commit_projection_fixture(repo.path(), "positive limit fixture");
    for args in [
        ["where", "target", "--limit", "0"],
        ["cone", "src/target.ts#target", "--limit", "0"],
    ] {
        let output = codemap()
            .current_dir(repo.path())
            .env("CODEMAP_CACHE_DIR", cache.path())
            .args(args)
            .output()
            .expect("zero-limit command should return");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "zero limit must be rejected");
        assert!(stderr.contains("at least 1"), "{stderr}");
        assert!(!stderr.contains("panicked"), "{stderr}");
    }
}

#[test]
fn multi_where_is_model_bounded_and_keeps_truthful_horizons() {
    let (repo, cache) = readable_projection_repo("multi-budget");
    for index in 0..12 {
        write(
            &repo.path().join(format!("src/def-{index:02}.ts")),
            "export function Shared() { return 1; }\n",
        );
        write(
            &repo.path().join(format!("src/use-{index:02}.ts")),
            &format!(
                "import {{ Shared }} from './def-{index:02}';\nexport const value{index} = Shared();\n"
            ),
        );
    }
    commit_projection_fixture(repo.path(), "multi budget fixture");

    let markdown = run_markdown(repo.path(), cache.path(), &["where", "Shared"]);
    assert!(
        markdown.contains("definition_matches: counted(12); shown=4 hidden=8"),
        "definition projection must be explicit: {markdown}"
    );
    assert_eq!(
        markdown
            .lines()
            .filter(|line| {
                line.contains("consumers: counted-at-least(1,")
                    && line.contains("shown=0 hidden=1")
            })
            .count(),
        4,
        "every rendered definition needs its truthful consumer horizon: {markdown}"
    );
    assert!(markdown.contains("def-03.ts#Shared"), "{markdown}");
    assert!(!markdown.contains("def-04.ts#Shared"), "{markdown}");
    assert!(markdown.lines().count() <= 60, "{markdown}");
    let approximate_tokens = markdown.chars().count().div_ceil(4);
    assert!(
        approximate_tokens <= 700,
        "multi where exceeded ~700 tokens ({approximate_tokens}): {markdown}"
    );
}

#[test]
fn saturated_symbol_cone_caps_readable_outgoing_but_all_and_json_are_complete() {
    let (repo, cache) = readable_projection_repo("saturated-cone");
    let mut imports = String::new();
    let mut calls = Vec::new();
    for index in 0..14 {
        write(
            &repo.path().join(format!("src/d{index:02}.ts")),
            &format!("export function d{index:02}() {{ return {index}; }}\n"),
        );
        imports.push_str(&format!("import {{ d{index:02} }} from './d{index:02}';\n"));
        calls.push(format!("d{index:02}()"));
    }
    write(
        &repo.path().join("src/target.ts"),
        &format!(
            "{imports}\nexport function target() {{ return {}; }}\n",
            calls.join(" + ")
        ),
    );
    for index in 0..8 {
        write(
            &repo.path().join(format!("src/c{index:02}.ts")),
            &format!(
                "import {{ target }} from './target';\nexport const c{index:02} = target();\n"
            ),
        );
    }
    commit_projection_fixture(repo.path(), "saturated cone fixture");
    let anchor = "src/target.ts#target";

    let bounded = run_markdown(repo.path(), cache.path(), &["cone", anchor]);
    assert!(bounded.lines().count() <= 90, "{bounded}");
    let approximate_tokens = bounded.chars().count().div_ceil(4);
    assert!(
        approximate_tokens <= 1100,
        "saturated cone exceeded ~1100 tokens ({approximate_tokens}): {bounded}"
    );
    assert!(
        bounded.contains("symbol outgoing edges hidden by limit"),
        "readable model must own the outgoing cap: {bounded}"
    );

    let json = run_json(
        repo.path(),
        cache.path(),
        &["cone", anchor, "--limit", "2", "--format", "json"],
    );
    assert_eq!(json["outgoing"].as_array().expect("outgoing").len(), 14);
    let all = run_markdown(
        repo.path(),
        cache.path(),
        &["cone", anchor, "--all", "--limit", "2"],
    );
    for index in 0..14 {
        assert!(all.contains(&format!("src/d{index:02}.ts")), "{all}");
    }
}

#[test]
fn long_symbol_paths_compact_readable_facts_without_touching_json_or_budgets() {
    let (repo, cache) = readable_projection_repo("long-symbol-paths");
    let base = "packages/application-shell/src/features/financial-reporting/quarterly-statements";
    let target_file = format!("{base}/target-with-descriptive-business-name.ts");
    let target_symbol = "targetWithDescriptiveBusinessName";
    let anchor = format!("{target_file}#{target_symbol}");
    let mut imports = String::new();
    let mut calls = Vec::new();
    for index in 0..14 {
        let side = if index % 2 == 0 { "a" } else { "b" };
        let region = format!("region-{index:02}-{side}");
        let dependency = format!("{base}/dependencies/{region}/shared-business-rule.ts");
        write(
            &repo.path().join(&dependency),
            &format!("export function businessRule{index:02}() {{ return {index}; }}\n"),
        );
        imports.push_str(&format!(
            "import {{ businessRule{index:02} }} from './dependencies/{region}/shared-business-rule';\n"
        ));
        calls.push(format!("businessRule{index:02}()"));
    }
    write(
        &repo.path().join(&target_file),
        &format!(
            "{imports}\nexport function {target_symbol}() {{ return {}; }}\n",
            calls.join(" + ")
        ),
    );
    for index in 0..8 {
        write(
            &repo
                .path()
                .join(format!("{base}/consumers/descriptive-consumer-{index}.ts")),
            &format!(
                "import {{ {target_symbol} }} from '../target-with-descriptive-business-name';\nexport const consumer{index} = {target_symbol}();\n"
            ),
        );
    }
    for index in 0..5 {
        write(
            &repo
                .path()
                .join(format!("{base}/tests/descriptive-target-{index}.test.ts")),
            &format!(
                "import {{ {target_symbol} }} from '../target-with-descriptive-business-name';\ntest('target {index}', () => {target_symbol}());\n"
            ),
        );
    }
    commit_projection_fixture(repo.path(), "long path projection fixture");

    let cone = run_markdown(repo.path(), cache.path(), &["cone", &anchor]);
    let where_output = run_markdown(repo.path(), cache.path(), &["where", target_symbol]);
    for (name, markdown, max_lines, max_tokens) in [
        ("cone", &cone, 90, 1_100),
        ("where", &where_output, 60, 700),
    ] {
        assert!(markdown.lines().count() <= max_lines, "{name}: {markdown}");
        let tokens = markdown.chars().count().div_ceil(4);
        assert!(
            tokens <= max_tokens,
            "{name} used ~{tokens} tokens: {markdown}"
        );
        assert!(markdown.contains("aliases: @anchor"), "{name}: {markdown}");
        assert!(markdown.contains("`@anchor`"), "{name}: {markdown}");
        assert!(markdown.contains("`@from:1`"), "{name}: {markdown}");
    }
    let fact_anchor_mentions = cone
        .lines()
        .filter(|line| !line.contains("codemap "))
        .map(|line| line.matches(&anchor).count())
        .sum::<usize>();
    assert_eq!(
        fact_anchor_mentions, 1,
        "full anchor must live in the header: {cone}"
    );
    assert!(cone.contains("./dependencies/region-00-a/shared-business-rule.ts"));
    assert!(cone.contains("./dependencies/region-01-b/shared-business-rule.ts"));

    let cone_json = run_json(
        repo.path(),
        cache.path(),
        &["cone", &anchor, "--format", "json"],
    );
    let serialized = serde_json::to_string(&cone_json).expect("serialize cone JSON");
    assert!(serialized.contains(&target_file), "{cone_json:#}");
    assert!(
        !serialized.contains("@anchor"),
        "aliases must stay readable-only: {cone_json:#}"
    );
}

fn readable_projection_repo(name: &str) -> (TempDir, TempDir) {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        &format!(r#"{{"name":"{name}","private":true}}"#),
    );
    (repo, cache)
}

fn commit_projection_fixture(repo: &Path, message: &str) {
    git(repo, &["add", "."]);
    git(repo, &["commit", "-qm", message]);
}
