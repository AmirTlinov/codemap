#[test]
fn where_single_definition_is_cone_equivalent() {
    let (repo, cache) = fixture();
    let json = run_json(repo.path(), cache.path(), &["where", "seek", "--format", "json"]);
    assert_schema("schemas/where.schema.json", &json);
    assert_eq!(json["kind"], "where_report");
    assert_eq!(json["total_matches"], 1);
    assert_eq!(
        json["definitions"][0]["anchor"]["path"],
        "packages/replay/src/session.ts#seek"
    );
    let consumers = json["definitions"][0]["consumers"]
        .as_array()
        .expect("consumers array");
    assert!(
        consumers.iter().any(|edge| edge["from"]
            .as_str()
            .is_some_and(|from| from.contains("useReplay"))),
        "seek should list useReplay as a consumer: {json:#}"
    );

    // Single-def `where` renders the full cone X-Ray and must not regress the
    // `#sym#sym` double-hash bug.
    let markdown = run_markdown(repo.path(), cache.path(), &["where", "seek"]);
    assert!(
        markdown.contains("# Structural Where") && markdown.contains("## X-Ray Card"),
        "single-def where should render the cone-equivalent card: {markdown}"
    );
    assert!(
        markdown.contains("packages/replay/src/session.ts#seek")
            && !markdown.contains("#seek#seek"),
        "symbol anchor must not double the `#sym` suffix: {markdown}"
    );
}

#[test]
fn where_multi_definition_is_path_ordered_not_ranked() {
    let (repo, cache) = fixture();
    let json = run_json(
        repo.path(),
        cache.path(),
        &["where", "ShellHint", "--format", "json"],
    );
    assert_schema("schemas/where.schema.json", &json);
    // ShellHint is defined in several files (exports plus shadow copies); where lists
    // every definition honestly.
    assert!(
        json["total_matches"].as_u64().expect("total") >= 2,
        "ShellHint has multiple definitions: {json:#}"
    );
    let paths: Vec<String> = json["definitions"]
        .as_array()
        .expect("definitions array")
        .iter()
        .map(|def| def["anchor"]["path"].as_str().expect("anchor path").to_string())
        .collect();
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(
        paths, sorted,
        "definitions must be deterministically path-ordered, never ranked: {json:#}"
    );
    assert!(
        paths.contains(
            &"packages/app/src/features/studio/canvas/other-shell-hint.tsx#ShellHint".to_string()
        ) && paths.contains(
            &"packages/app/src/features/studio/canvas/shell-hint.tsx#ShellHint".to_string()
        ),
        "both real ShellHint exports must appear: {paths:?}"
    );
    for def in json["definitions"].as_array().expect("definitions") {
        assert!(
            def["expand"]
                .as_array()
                .expect("expand")
                .iter()
                .any(|cmd| cmd.as_str().expect("cmd").starts_with("codemap cone ")),
            "each definition should expose an exact cone expand: {def:#}"
        );
    }
}

#[test]
fn where_kind_filter_narrows_by_symbol_kind() {
    let (repo, cache) = fixture();
    let as_class = run_json(
        repo.path(),
        cache.path(),
        &["where", "seek", "--kind", "class", "--format", "json"],
    );
    assert_eq!(
        as_class["total_matches"], 0,
        "seek is a function, not a class: {as_class:#}"
    );
    let as_function = run_json(
        repo.path(),
        cache.path(),
        &["where", "seek", "--kind", "function", "--format", "json"],
    );
    assert_eq!(as_function["total_matches"], 1);
}

#[test]
fn where_not_found_emits_unknown_and_soft_matches() {
    let (repo, cache) = fixture();
    let missing = run_json(
        repo.path(),
        cache.path(),
        &["where", "doesNotExistXyz", "--format", "json"],
    );
    assert_schema("schemas/where.schema.json", &missing);
    assert_eq!(missing["total_matches"], 0);
    assert!(
        missing["definitions"]
            .as_array()
            .expect("definitions")
            .is_empty()
    );
    assert!(
        missing["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "symbol_not_found"),
        "not-found should emit a typed symbol_not_found unknown: {missing:#}"
    );

    // A substring with no exact match yields soft, explicitly-not-ranked suggestions.
    let soft = run_json(
        repo.path(),
        cache.path(),
        &["where", "Shell", "--format", "json"],
    );
    let suggestions = soft["soft_suggestions"]
        .as_array()
        .expect("soft_suggestions");
    assert!(
        !suggestions.is_empty(),
        "`Shell` should yield soft substring suggestions: {soft:#}"
    );
    assert!(
        suggestions.iter().all(|suggestion| suggestion["expand"]
            .as_str()
            .expect("expand")
            .starts_with("codemap where ")),
        "soft suggestions must point back at exact where lookups: {soft:#}"
    );
    let markdown = run_markdown(repo.path(), cache.path(), &["where", "Shell"]);
    assert!(
        markdown.contains("[Soft]") && markdown.contains("not ranked, not an answer"),
        "soft matches must be labeled and disclaimed as not an answer: {markdown}"
    );
}

#[test]
fn where_kind_filter_accepts_displayed_symbol_prefix() {
    // Regression W1: where output shows `symbol:function`; the --kind filter must
    // accept that displayed form as well as the bare `function`.
    let (repo, cache) = fixture();
    let bare = run_json(
        repo.path(),
        cache.path(),
        &["where", "seek", "--kind", "function", "--format", "json"],
    );
    let prefixed = run_json(
        repo.path(),
        cache.path(),
        &["where", "seek", "--kind", "symbol:function", "--format", "json"],
    );
    assert_eq!(prefixed["total_matches"], 1);
    assert_eq!(bare["total_matches"], prefixed["total_matches"]);
}
