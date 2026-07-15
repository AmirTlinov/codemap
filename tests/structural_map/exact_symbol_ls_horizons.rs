// Responsibility: exact-symbol-ls-horizon-contract
const EXACT_SYMBOL_LS_ANCHOR: &str = "src/token.ts#refreshToken";

#[test]
fn exact_symbol_ls_readable_and_json_share_two_certified_horizons() {
    let repo = exact_symbol_ls_fixture();
    let readable_cache = TempDir::new().expect("symbol ls readable cache");
    let json_cache = TempDir::new().expect("symbol ls json cache");

    let readable = run_markdown(
        repo.path(),
        readable_cache.path(),
        &["ls", EXACT_SYMBOL_LS_ANCHOR, "--limit", "1"],
    );
    let json = run_json(
        repo.path(),
        json_cache.path(),
        &[
            "ls",
            EXACT_SYMBOL_LS_ANCHOR,
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/ls.schema.json", &json);
    assert_eq!(json["schema_version"], "14", "{json:#}");
    assert_eq!(json["edges"].as_array().expect("symbol edges").len(), 3);
    assert!(
        json["hidden"].as_array().expect("hidden").is_empty(),
        "full JSON must not repeat detached symbol-edge accounting: {json:#}"
    );

    let ledger = &json["observations"];
    assert_eq!(ledger["horizons"].as_array().expect("horizons").len(), 2);
    for (group, observed, closure) in [
        ("consumers", 2, "closed"),
        ("verification", 1, "open"),
    ] {
        let item = horizon(ledger, group);
        assert_eq!(
            item["count"]["observed"], observed,
            "{group}: {json:#}"
        );
        assert_eq!(item["count"]["closure"], closure, "{group}: {json:#}");
        assert_eq!(item["shown"], observed, "{group}: {json:#}");
        assert_eq!(item["hidden"], 0, "{group}: {json:#}");
        assert_horizon_certificate_resolves(ledger, item);

        let certificate = item["count"]["certificate_id"]
            .as_str()
            .expect("certificate")
            .strip_prefix("coverage-v1:")
            .expect("coverage certificate");
        let preview = format!("cert=`v1:{}`", &certificate[..12]);
        assert!(
            readable
                .lines()
                .any(|line| line.starts_with(&format!("- {group}:")) && line.contains(&preview)),
            "readable and JSON must expose the same {group} certificate: {readable}"
        );
    }
    let readable_visibility = readable
        .lines()
        .filter(|line| line.starts_with("- consumers:") || line.starts_with("- verification:"))
        .collect::<Vec<_>>();
    assert_eq!(readable_visibility.len(), 2, "{readable}");
    assert!(
        readable_visibility
            .iter()
            .all(|line| line.contains("shown=1"))
            && readable_visibility
                .iter()
                .any(|line| line.starts_with("- consumers:") && line.contains("hidden=1"))
            && readable_visibility
                .iter()
                .any(|line| line.starts_with("- verification:") && line.contains("hidden=0")),
        "the limit-one projection must represent both populated relationship groups: {readable}"
    );
    assert!(
        !readable.contains("symbol edges hidden by limit"),
        "the horizons own symbol visibility accounting: {readable}"
    );
}

#[test]
fn missing_exact_symbol_keeps_both_ls_groups_unavailable() {
    let repo = exact_symbol_ls_fixture();
    for anchor in ["src/token.ts#missing", "src/absent.ts#refreshToken"] {
        let cache = TempDir::new().expect("missing symbol ls cache");
        let json = run_json(
            repo.path(),
            cache.path(),
            &["ls", anchor, "--format", "json"],
        );
        assert_eq!(json["mode"], "missing", "{anchor}: {json:#}");
        assert_eq!(json["path"], anchor, "{anchor}: {json:#}");
        for group in ["consumers", "verification"] {
            let item = horizon(&json["observations"], group);
            assert_eq!(item["count"]["observed"], 0, "{anchor}: {json:#}");
            assert_eq!(
                item["count"]["closure"], "unavailable",
                "{anchor}: {json:#}"
            );
            assert_eq!(
                item["count"]["reasons"],
                serde_json::json!(["anchor_not_indexed"]),
                "{anchor}: {json:#}"
            );
            assert_horizon_certificate_resolves(&json["observations"], item);
        }
    }
}

#[test]
fn exact_symbol_ls_cache_preserves_the_full_machine_projection() {
    let repo = exact_symbol_ls_fixture();
    let cache = TempDir::new().expect("symbol ls warm cache");
    let args = [
        "ls",
        EXACT_SYMBOL_LS_ANCHOR,
        "--limit",
        "1",
        "--format",
        "json",
    ];
    let cold = run_json(repo.path(), cache.path(), &args);
    let artifact: Value = serde_json::from_str(
        &fs::read_to_string(lens_artifact_path(cache.path(), "ls-current.json"))
            .expect("cached exact-symbol ls artifact"),
    )
    .expect("exact-symbol ls artifact json");
    assert_eq!(
        artifact["report"]["observations"]["horizons"]
            .as_array()
            .expect("cached horizons")
            .len(),
        2,
        "{artifact:#}"
    );
    let warm = run_json(repo.path(), cache.path(), &args);
    assert_eq!(warm, cold, "warm exact-symbol LS must be byte-model identical");
}

fn exact_symbol_ls_fixture() -> TempDir {
    let repo = TempDir::new().expect("exact symbol ls repo");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"exact-symbol-ls","private":true}"#,
    );
    write(
        &repo.path().join("src/token.ts"),
        "export function refreshToken(value: string) { return value; }\n",
    );
    write(
        &repo.path().join("src/session.ts"),
        "import { refreshToken } from './token';\nexport const session = refreshToken('live');\n",
    );
    write(
        &repo.path().join("src/namespace.ts"),
        "import * as token from './token';\nexport const mediated = token.refreshToken('open');\n",
    );
    write(
        &repo.path().join("tests/token.test.ts"),
        "import { refreshToken } from '../src/token';\ntest('refresh', () => refreshToken('proof'));\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "exact symbol ls fixture"]);
    repo
}
