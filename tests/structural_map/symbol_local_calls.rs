#[test]
fn symbol_anchor_cone_follows_local_calls_in_primary_languages() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("src/session.rs"),
        r#"pub fn handle_request(input: &str) -> String {
    let parsed = parse_request(input);
    // comment_only(input) must not become xref.
    let _doc = "comment_only(input)";
    let _raw_doc = r"comment_only(input)";
    let _block_doc = /*
        comment_only(input)
    */ "";
    build_response(parsed)
}

fn parse_request(input: &str) -> String {
    input.to_string()
}

fn build_response(value: String) -> String {
    value
}

fn comment_only(_input: &str) -> String {
    String::new()
}
"#,
    );
    write(
        &repo.path().join("src/session.py"),
        r#"def handle_request(input: str) -> str:
    parsed = parse_request(input)
    # comment_only(input) must not become xref.
    doc = "comment_only(input)"
    raw_doc = r"""comment_only(input)"""
    formatted_doc = f'''
        comment_only(input)
    '''
    return build_response(parsed)


def parse_request(input: str) -> str:
    return input


def build_response(value: str) -> str:
    return value


def comment_only(input: str) -> str:
    return ""
"#,
    );
    write(
        &repo.path().join("go/session.go"),
        r#"package session

func HandleRequest(input string) string {
	parsed := parseRequest(input)
	// commentOnly(input) must not become xref.
	doc := "commentOnly(input)"
	rawDoc := `commentOnly(input)`
	blockDoc := /*
		commentOnly(input)
	*/ ""
	_ = doc
	_ = rawDoc
	_ = blockDoc
	return buildResponse(parsed)
}

func parseRequest(input string) string {
	return input
}

func buildResponse(value string) string {
	return value
}

func commentOnly(input string) string {
	return ""
}
"#,
    );
    write(
        &repo.path().join("Sources/App/Session.swift"),
        r#"public func handleRequest(_ input: String) -> String {
    let parsed = parseRequest(input)
    // commentOnly(input) must not become xref.
    let doc = "commentOnly(input)"
    let multilineDoc = """
        commentOnly(input)
    """
    _ = doc
    _ = multilineDoc
    return buildResponse(parsed)
}

func parseRequest(_ input: String) -> String {
    input
}

func buildResponse(_ value: String) -> String {
    value
}

func commentOnly(_ input: String) -> String {
    ""
}
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "local symbol xref fixture"]);

    for (anchor, expected_a, expected_b, rejected) in [
        (
            "src/session.rs#handle_request",
            "src/session.rs#parse_request",
            "src/session.rs#build_response",
            "src/session.rs#comment_only",
        ),
        (
            "src/session.py#handle_request",
            "src/session.py#parse_request",
            "src/session.py#build_response",
            "src/session.py#comment_only",
        ),
        (
            "go/session.go#HandleRequest",
            "go/session.go#parseRequest",
            "go/session.go#buildResponse",
            "go/session.go#commentOnly",
        ),
        (
            "Sources/App/Session.swift#handleRequest",
            "Sources/App/Session.swift#parseRequest",
            "Sources/App/Session.swift#buildResponse",
            "Sources/App/Session.swift#commentOnly",
        ),
    ] {
        let cone = run_json(
            repo.path(),
            cache.path(),
            &["cone", anchor, "--format", "json"],
        );
        assert_schema("schemas/cone.schema.json", &cone);
        let outgoing = cone["outgoing"].as_array().expect("outgoing");
        for expected in [expected_a, expected_b] {
            assert!(
                outgoing.iter().any(|edge| edge["to"] == expected
                    && edge["type"] == "symbol_uses"
                    && edge["evidence"] == "local_symbol_in_symbol_body"),
                "symbol cone for {anchor} should point at local call {expected}: {cone:#}"
            );
        }
        assert!(
            outgoing.iter().all(|edge| edge["to"] != rejected),
            "symbol cone for {anchor} must ignore comments/strings mentioning {rejected}: {cone:#}"
        );
    }
}

#[test]
fn python_multiline_signature_keeps_the_function_body_in_the_exact_cone() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("src/carrier.py"),
        "def deliver(value: str) -> str:\n    return value\n",
    );
    write(
        &repo.path().join("src/entry.py"),
        r#"from .carrier import deliver


def dispatch(
    value: str,
    *,
    enabled: bool = True,
) -> str:
    if not enabled:
        return value
    return deliver(value)
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "python multiline range fixture"]);

    let cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/entry.py#dispatch", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert_eq!(cone["anchor"]["symbols"][0]["line_end"], 11, "{cone:#}");
    assert!(
        cone["outgoing"]
            .as_array()
            .expect("outgoing")
            .iter()
            .any(|edge| {
                edge["to"] == "src/carrier.py#deliver"
                    && edge["evidence"] == "imported_symbol_in_symbol_body"
            }),
        "the multiline signature must not truncate its imported carrier call: {cone:#}"
    );
}


#[test]
fn symbol_anchor_cone_does_not_treat_receiver_methods_as_local_function_calls() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("src/session.go"),
        r#"package session

type Session struct{}

func Caller() {
	helper()
}

func (s Session) helper() {}
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "method target fixture"]);

    let caller_cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/session.go#Caller", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &caller_cone);
    assert!(
        caller_cone["outgoing"]
            .as_array()
            .expect("outgoing")
            .iter()
            .all(|edge| edge["to"] != "src/session.go#helper"),
        "unqualified call must not become a same-file function edge to receiver method: {caller_cone:#}"
    );

    let helper_cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/session.go#helper", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &helper_cone);
    assert!(
        helper_cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .all(|edge| edge["from"] != "src/session.go#Caller"),
        "incoming cone must not report local function consumers for receiver method targets: {helper_cone:#}"
    );
}
