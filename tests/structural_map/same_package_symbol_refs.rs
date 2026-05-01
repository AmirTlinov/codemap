#[test]
fn proof_and_impact_link_same_package_symbol_references_without_imports() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("go.mod"),
        "module example.com/replay\n\ngo 1.22\n",
    );
    write(
        &repo.path().join("session/session.go"),
        "package session\n\nfunc FrameLabel(frame int) string {\n\treturn \"frame\"\n}\n",
    );
    write(
        &repo.path().join("session/consumer.go"),
        "package session\n\nfunc RenderLabel() string {\n\treturn FrameLabel(1)\n}\n",
    );
    write(
        &repo.path().join("session/raw_fixture.go"),
        "package session\n\nconst fixture = `\nFrameLabel\n`\n",
    );
    write(
        &repo.path().join("session/foreign_consumer.go"),
        "package session\n\nimport other \"example.com/replay/other\"\n\nfunc RenderForeignLabel() string {\n\treturn other.FrameLabel(1)\n}\n",
    );
    write(
        &repo.path().join("other/label.go"),
        "package other\n\nfunc FrameLabel(frame int) string {\n\treturn \"foreign\"\n}\n",
    );
    write(
        &repo.path().join("session/method_session.go"),
        "package session\n\ntype Session struct{}\n\nfunc (s Session) Reset() {}\n",
    );
    write(
        &repo.path().join("session/cache.go"),
        "package session\n\ntype Cache struct{}\n\nfunc (c Cache) Reset() {}\n",
    );
    write(
        &repo.path().join("session/surface_test.go"),
        "package session\n\nimport \"testing\"\n\nfunc TestSurfaceUsesFrameLabel(t *testing.T) {\n\tif FrameLabel(2) == \"\" {\n\t\tt.Fatal(\"missing label\")\n\t}\n}\n",
    );
    write(
        &repo.path().join("session/raw_string_test.go"),
        "package session\n\nimport \"testing\"\n\nfunc TestRawStringOnly(t *testing.T) {\n\tfixture := `\nFrameLabel\n`\n\tif fixture == \"\" {\n\t\tt.Fatal(\"missing fixture\")\n\t}\n}\n",
    );
    write(
        &repo.path().join("session/foreign_test.go"),
        "package session\n\nimport (\n\t\"testing\"\n\tother \"example.com/replay/other\"\n)\n\nfunc TestForeignSelector(t *testing.T) {\n\tif other.FrameLabel(3) == \"\" {\n\t\tt.Fatal(\"missing foreign label\")\n\t}\n}\n",
    );
    write(
        &repo.path().join("session/cache_test.go"),
        "package session\n\nimport \"testing\"\n\nfunc TestCacheReset(t *testing.T) {\n\tvar cache Cache\n\tcache.Reset()\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "session/session.go", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .any(|surface| surface["path"] == "session/surface_test.go"
                && surface["evidence"] == "test_symbol_reference"
                && surface["strength"] == "high"),
        "same-package test symbol references should become structural proof, not fallback: {proof:#}"
    );
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .all(|surface| surface["path"] != "session/raw_string_test.go"),
        "symbols inside multiline raw strings must not become proof: {proof:#}"
    );
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .all(|surface| surface["path"] != "session/foreign_test.go"),
        "selector tails from imported packages must not become local symbol proof: {proof:#}"
    );
    assert!(
        proof["fallback"].as_array().expect("fallback").is_empty(),
        "symbol reference proof should suppress broad fallback: {proof:#}"
    );

    let cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "session/session.go", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert!(
        cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .any(|edge| edge["from"] == "session/consumer.go"
                && edge["to"] == "session/session.go"
                && edge["evidence"] == "same_package_symbol_reference"),
        "same-package source references should appear as incoming xref edges: {cone:#}"
    );
    assert!(
        cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .all(|edge| edge["from"] != "session/raw_fixture.go"),
        "symbols inside multiline raw strings must not become incoming xref edges: {cone:#}"
    );
    assert!(
        cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .all(|edge| edge["from"] != "session/foreign_consumer.go"),
        "selector tails from imported packages must not become local incoming xref edges: {cone:#}"
    );

    let impact = run_json(
        repo.path(),
        cache.path(),
        &[
            "impact",
            "--files",
            "session/session.go",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/impact.schema.json", &impact);
    let clusters = impact["clusters"].as_array().expect("clusters");
    assert!(
        clusters.iter().any(|cluster| cluster["direct_consumers"]
            .as_array()
            .expect("direct consumers")
            .iter()
            .any(|edge| edge["from"] == "session/consumer.go"
                && edge["evidence"] == "same_package_symbol_reference")),
        "impact should carry same-package symbol xref consumers: {impact:#}"
    );
    assert!(
        clusters.iter().all(|cluster| cluster["direct_consumers"]
            .as_array()
            .expect("direct consumers")
            .iter()
            .all(|edge| edge["from"] != "session/raw_fixture.go")),
        "raw-string-only files must not inflate impact consumers: {impact:#}"
    );
    assert!(
        clusters.iter().all(|cluster| cluster["direct_consumers"]
            .as_array()
            .expect("direct consumers")
            .iter()
            .all(|edge| edge["from"] != "session/foreign_consumer.go")),
        "selector-tail references must not inflate local impact consumers: {impact:#}"
    );

    let method_proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "session/method_session.go", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &method_proof);
    assert!(
        method_proof["proofs"]
            .as_array()
            .expect("method proofs")
            .iter()
            .all(|surface| surface["path"] != "session/cache_test.go"),
        "same-name methods on different receivers need type-aware xref and must not become proof: {method_proof:#}"
    );

    let method_cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "session/method_session.go", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &method_cone);
    assert!(
        method_cone["incoming"]
            .as_array()
            .expect("method incoming")
            .iter()
            .all(|edge| edge["from"] != "session/cache.go"),
        "same-name method declarations on different receivers must not become incoming xref edges: {method_cone:#}"
    );

    let method_impact = run_json(
        repo.path(),
        cache.path(),
        &[
            "impact",
            "--files",
            "session/method_session.go",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/impact.schema.json", &method_impact);
    assert!(
        method_impact["clusters"]
            .as_array()
            .expect("method clusters")
            .iter()
            .all(|cluster| cluster["direct_consumers"]
                .as_array()
                .expect("method direct consumers")
                .iter()
                .all(|edge| edge["from"] != "session/cache.go")),
        "same-name methods on unrelated receivers must not inflate impact: {method_impact:#}"
    );
}

