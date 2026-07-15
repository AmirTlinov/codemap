#[test]
fn symbol_anchor_cone_filters_javascript_import_bindings() {
    let (repo, cache) = symbol_import_fixture();

    let ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", "src/card.tsx#GroupCard", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &ls);
    assert_eq!(ls["anchor"]["path"], "src/card.tsx#GroupCard");
    assert_eq!(ls["anchor"]["kind"], "symbol:component");
    assert_eq!(
        ls["anchor"]["symbols"].as_array().expect("symbols").len(),
        1
    );

    let cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/card.tsx#GroupCard", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    let incoming = cone["incoming"].as_array().expect("incoming");
    assert!(
        incoming.iter().any(|edge| {
            edge["from"] == "src/home.tsx"
                && edge["to"] == "src/card.tsx#GroupCard"
                && edge["evidence"] == "imported_symbol_reference"
        }),
        "symbol cone should show the aliased component consumer: {cone:#}"
    );
    assert!(
        incoming
            .iter()
            .any(|edge| edge["from"] == "src/side-effect.tsx"
                && edge["to"] == "src/card.tsx#GroupCard"
                && edge["evidence"] == "imported_symbol_reference"),
        "semicolonless side-effect imports must not hide later symbol references: {cone:#}"
    );
    assert!(
        incoming.iter().all(|edge| edge["from"] != "src/admin.tsx"
            && edge["from"] != "src/unused.tsx"
            && edge["from"] != "src/string-only.tsx"
            && edge["from"] != "src/await-regex-consumer.ts"
            && edge["from"] != "src/if-regex-consumer.tsx"
            && edge["from"] != "src/else-regex-consumer.tsx"
            && edge["from"] != "src/type-generic-consumer.tsx"
            && edge["from"] != "src/template-consumer.tsx"
            && edge["from"] != "src/generic-arrow.tsx"
            && edge["from"] != "src/local-shadow.tsx"
            && edge["from"] != "src/for-shadow.tsx"
            && edge["from"] != "src/for-await-shadow.tsx"
            && edge["from"] != "src/catch-shadow.tsx"),
        "symbol cone must not include other exports, unused imports, string-only mentions, or local/loop/catch shadows: {cone:#}"
    );
    assert!(
        cone["proof"]
            .as_array()
            .expect("proof")
            .iter()
            .any(|edge| edge["from"] == "src/card.test.tsx"
                && edge["to"] == "src/card.tsx#GroupCard"
                && edge["evidence"] == "test_imported_symbol_reference"),
        "symbol cone should expose exact symbol proof: {cone:#}"
    );

    let home_cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/home.tsx#HomePage", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &home_cone);
    assert!(
        home_cone["outgoing"]
            .as_array()
            .expect("home outgoing")
            .iter()
            .any(|edge| edge["from"] == "src/home.tsx#HomePage"
                && edge["to"] == "src/card.tsx#GroupCard"
                && edge["type"] == "symbol_uses"
                && edge["evidence"] == "imported_symbol_in_symbol_body"),
        "symbol cone should show imported symbols used inside the selected symbol body: {home_cone:#}"
    );
    assert!(
        home_cone["unknowns"]
            .as_array()
            .expect("home unknowns")
            .is_empty(),
        "symbol cone should not claim outgoing unknown when it found structural symbol uses: {home_cone:#}"
    );

    let panel_cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/panel-view.tsx#PanelView", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &panel_cone);
    let panel_outgoing = panel_cone["outgoing"].as_array().expect("panel outgoing");
    assert!(
        panel_outgoing
            .iter()
            .any(|edge| edge["to"] == "src/panel-parts.tsx#PanelHeader"
                && edge["type"] == "symbol_uses"
                && edge["evidence"] == "imported_symbol_in_symbol_body"),
        "symbol cone should include imported JSX symbols after multiline destructured params: {panel_cone:#}"
    );
    assert!(
        panel_outgoing
            .iter()
            .any(|edge| edge["to"] == "src/panel-parts.tsx#PanelBody"
                && edge["type"] == "symbol_uses"
                && edge["evidence"] == "imported_symbol_in_symbol_body"),
        "symbol cone should not stop the symbol body at the destructured parameter close: {panel_cone:#}"
    );
    assert!(
        panel_cone["unknowns"]
            .as_array()
            .expect("panel unknowns")
            .is_empty(),
        "symbol cone should not claim unknown outgoing once multiline-param symbol uses are found: {panel_cone:#}"
    );

    for false_anchor in [
        "src/unused.tsx#unused",
        "src/if-regex-consumer.tsx#regexConsumer",
        "src/generic-arrow.tsx#make",
        "src/angle-assertion.ts#cast",
        "src/local-shadow.tsx#ShadowPage",
    ] {
        let false_cone = run_json(
            repo.path(),
            cache.path(),
            &["cone", false_anchor, "--format", "json"],
        );
        assert_schema("schemas/cone.schema.json", &false_cone);
        assert!(
            false_cone["outgoing"]
                .as_array()
                .expect("false outgoing")
                .iter()
                .all(|edge| edge["to"] != "src/card.tsx#GroupCard"),
            "symbol outgoing must not link unused imports, regex-only mentions, or local shadows for {false_anchor}: {false_cone:#}"
        );
    }
    let lowercase_cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "src/lowercase-jsx.tsx#LowercaseView",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &lowercase_cone);
    assert!(
        lowercase_cone["outgoing"]
            .as_array()
            .expect("lowercase outgoing")
            .iter()
            .all(|edge| edge["to"] != "src/helpers.tsx#custom"),
        "lowercase JSX tags must not become imported symbol edges: {lowercase_cone:#}"
    );

    let limited_cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "src/two-cards.tsx#TwoCards",
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &limited_cone);
    assert_eq!(
        limited_cone["outgoing"]
            .as_array()
            .expect("limited outgoing")
            .len(),
        2,
        "JSON must keep every observed symbol outgoing edge: {limited_cone:#}"
    );
    assert!(
        limited_cone["hidden"]
            .as_array()
            .expect("limited hidden")
            .iter()
            .all(|group| group["reason"] != "symbol outgoing edges hidden by limit"),
        "full JSON must not report serialized outgoing edges as hidden: {limited_cone:#}"
    );
    let limited_markdown = run_markdown(
        repo.path(),
        cache.path(),
        &["cone", "src/two-cards.tsx#TwoCards", "--limit", "1"],
    );
    assert!(
        limited_markdown.contains("symbol outgoing edges hidden by limit: 1"),
        "readable symbol cone should keep its bounded outgoing projection: {limited_markdown}"
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "src/card.tsx#GroupCard", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let proofs = proof["proofs"].as_array().expect("proofs");
    assert!(
        proofs
            .iter()
            .any(|surface| surface["path"] == "src/card.test.tsx"
                && surface["evidence"] == "test_imported_symbol_reference"
                && surface["command"] == "npx vitest run src/card.test.tsx"),
        "symbol proof should prefer the exact importing test file: {proof:#}"
    );
    assert!(
        proofs
            .iter()
            .all(|surface| surface["path"] != "src/admin.test.tsx"),
        "symbol proof must not inherit tests for sibling exports: {proof:#}"
    );
    assert!(
        proofs
            .iter()
            .all(|surface| surface["path"] != "src/type-only-consumer.test.tsx"),
        "type-only symbol mentions must not become runtime proof: {proof:#}"
    );
    assert!(
        proofs.iter().all(
            |surface| surface["path"] != "src/type-annotation-consumer.test.tsx"
                && surface["path"] != "src/type-assertion-consumer.test.tsx"
                && surface["path"] != "src/implements-only.test.tsx"
                && surface["path"] != "src/object-key.test.tsx"
                && surface["path"] != "src/regex-only.test.tsx"
                && surface["path"] != "src/regex-angle.test.tsx"
                && surface["path"] != "src/regex-group.test.tsx"
                && surface["path"] != "src/arrow-regex-group.test.tsx"
                && surface["path"] != "src/await-regex.test.tsx"
                && surface["path"] != "src/if-regex.test.tsx"
                && surface["path"] != "src/else-regex.test.tsx"
                && surface["path"] != "src/throw-regex.test.tsx"
                && surface["path"] != "src/type-generic.test.tsx"
                && surface["path"] != "src/type-factory.test.tsx"
                && surface["path"] != "src/generic-arrow.test.tsx"
                && surface["path"] != "src/template-only.test.tsx"
                && surface["path"] != "src/for-await-shadow.test.tsx"
        ),
        "type-only/object-key/regex mentions must not become runtime proof: {proof:#}"
    );
    assert!(
        proof["fallback"].as_array().expect("fallback").is_empty(),
        "exact symbol proof should suppress broad fallback: {proof:#}"
    );

    let default_cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "src/default-card.tsx#DefaultCard",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &default_cone);
    assert!(
        default_cone["incoming"]
            .as_array()
            .expect("default incoming")
            .iter()
            .any(|edge| edge["from"] == "src/default-consumer.tsx"
                && edge["to"] == "src/default-card.tsx#DefaultCard"
                && edge["evidence"] == "imported_symbol_reference"),
        "default import aliases should link to the named default symbol anchor: {default_cone:#}"
    );

    let default_proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "src/default-card.tsx#DefaultCard",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &default_proof);
    assert!(
        default_proof["proofs"]
            .as_array()
            .expect("default proofs")
            .iter()
            .any(|surface| surface["path"] == "src/default-card.test.tsx"
                && surface["evidence"] == "test_imported_symbol_reference"),
        "default import aliases should become exact symbol proof: {default_proof:#}"
    );

    let default_const_cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "src/default-const-card.tsx#DefaultConstCard",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &default_const_cone);
    assert!(
        default_const_cone["incoming"]
            .as_array()
            .expect("default const incoming")
            .iter()
            .any(|edge| edge["from"] == "src/default-const-consumer.tsx"
                && edge["to"] == "src/default-const-card.tsx#DefaultConstCard"
                && edge["evidence"] == "imported_symbol_reference"),
        "default identifier aliases should link to the local default-exported symbol anchor: {default_const_cone:#}"
    );

    let default_const_proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "src/default-const-card.tsx#DefaultConstCard",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &default_const_proof);
    assert!(
        default_const_proof["proofs"]
            .as_array()
            .expect("default const proofs")
            .iter()
            .any(
                |surface| surface["path"] == "src/default-const-card.test.tsx"
                    && surface["evidence"] == "test_imported_symbol_reference"
            ),
        "default identifier aliases should become exact symbol proof: {default_const_proof:#}"
    );
}
