#[test]
fn exact_entry_stays_local_and_zero_links_are_explicit() {
    let repo = TempDir::new().expect("proportional fixture");
    let cache = TempDir::new().expect("proportional cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("pyproject.toml"),
        "[project]\nname = \"proportional-fixture\"\nversion = \"0.1.0\"\n",
    );
    write(
        &repo.path().join("src/pricing.py"),
        "def calculate(amount: float) -> float:\n    return amount\n",
    );
    write(
        &repo.path().join("apps/unrelated/package.json"),
        r#"{"name":"unrelated-admin","scripts":{"deploy":"echo unrelated"}}"#,
    );
    write(
        &repo.path().join(".github/workflows/unrelated.yml"),
        "name: unrelated-root-ci\non: push\njobs: {}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "proportional fixture"]);

    for args in [
        vec!["ls", "src", "--section", "links"],
        vec!["ls", "src/pricing.py", "--section", "links"],
        vec!["cone", "src/pricing.py#calculate", "--section", "links"],
        vec!["where", "calculate"],
    ] {
        let markdown = run_markdown(repo.path(), cache.path(), &args);
        assert!(
            markdown.contains("No indexed structural links observed in this scope."),
            "valid zero-link exact entry must say zero explicitly: {markdown}"
        );
        assert!(
            !markdown.contains("unrelated-admin") && !markdown.contains("unrelated-root-ci"),
            "exact entry must not leak unrelated root catalog: {markdown}"
        );
    }

    let exact = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/pricing.py#calculate", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &exact);
    assert_eq!(exact["anchor"]["path"], "src/pricing.py#calculate");
    assert_eq!(exact["build_identity"]["binary_sha256"], Value::Null);
    assert_eq!(
        exact["build_identity"]["binary_sha256_state"],
        "not_requested"
    );

    let missing = run_markdown(repo.path(), cache.path(), &["ls", "src/missing.py"]);
    assert!(missing.contains("No indexed file or directory anchor found."));
    assert!(!missing.contains("No indexed structural links observed"));
    assert!(
        String::from_utf8(
            Command::new("git")
                .args(["status", "--porcelain=v1"])
                .current_dir(repo.path())
                .output()
                .expect("git status")
                .stdout,
        )
        .expect("status utf8")
        .trim()
        .is_empty(),
        "daily exact entries must leave the target repository untouched"
    );
}

#[test]
fn published_protocol_is_exact_first_without_a_router() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let help = String::from_utf8(codemap().arg("--help").output().unwrap().stdout).unwrap();
    let bootstrap = String::from_utf8(
        codemap()
            .args(["bootstrap", "--global-instruction"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let texts = [
        ("help", help),
        ("bootstrap", bootstrap),
        ("README", fs::read_to_string(root.join("README.md")).unwrap()),
        (
            "product",
            fs::read_to_string(root.join("docs/PRODUCT.md")).unwrap(),
        ),
        ("AGENTS", fs::read_to_string(root.join("AGENTS.md")).unwrap()),
    ];
    for (owner, text) in texts {
        for daily in [
            "codemap where",
            "codemap cone",
            "codemap ls .",
            "codemap changed",
            "codemap proof changed",
        ] {
            assert!(text.contains(daily), "{owner} omitted daily entry {daily}: {text}");
        }
        let lower = text.to_ascii_lowercase();
        assert!(
            lower.contains("root orientation") && lower.contains("unknown"),
            "{owner} must reserve root orientation for unknown scope: {text}"
        );
    }
}

#[test]
fn rich_single_definition_where_is_bounded_without_hiding_real_links() {
    let repo = TempDir::new().expect("rich where fixture");
    let cache = TempDir::new().expect("rich where cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"rich-where","scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join("src/target.ts"),
        "export const first = (value: number) => value + 1;\nexport const second = (value: number) => value * 2;\nexport const third = (value: number) => value - 3;\nexport function target(value: number): number {\n  return first(value) + second(value) + third(value);\n}\n",
    );
    for index in 0..6 {
        write(
            &repo.path().join(format!("src/consumer-{index}.ts")),
            &format!(
                "import {{ target }} from './target';\nexport const use{index} = target({index});\n"
            ),
        );
        write(
            &repo.path().join(format!("tests/target-{index}.test.ts")),
            &format!(
                "import {{ target }} from '../src/target';\ntest('target {index}', () => expect(target({index})).toBeTypeOf('number'));\n"
            ),
        );
    }
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "rich where fixture"]);

    let markdown = run_markdown(repo.path(), cache.path(), &["where", "target"]);
    assert!(
        markdown.contains("src/consumer-0.ts")
            && markdown.contains("symbol_uses -> `src/target.ts#first`"),
        "where must retain consumer and outgoing symbol-use facts: {markdown}"
    );
    assert!(
        markdown.lines().count() <= 60,
        "rich exact where exceeded 60 lines: {markdown}"
    );
    let approximate_tokens = markdown.chars().count().div_ceil(4);
    assert!(
        approximate_tokens <= 700,
        "rich exact where exceeded ~700 tokens ({approximate_tokens}): {markdown}"
    );
}
