#[test]
fn symbol_anchor_cone_links_same_file_symbol_body_uses() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "local-symbol-flow-fixture",
  "private": true,
  "scripts": { "test": "vitest run" }
}
"#,
    );
    write(
        &repo.path().join("src/local-flow.tsx"),
        "function formatFocus(id: string) {\n  return id.trim();\n}\n\nfunction combineFocus(prefix: string, formatter: (id: string) => string) {\n  return formatter(prefix);\n}\n\nfunction chooseFocus(ids: string[]) {\n  return formatFocus(ids[0] ?? '');\n}\n\nexport function SelectionPanel({ ids }: { ids: string[] }) {\n  const focus = chooseFocus(ids);\n  return <section>{focus}</section>;\n}\n\nexport function ArgumentUse() {\n  return combineFocus('x', formatFocus);\n}\n\nexport function ParameterShadow(formatFocus: (id: string) => string) {\n  return formatFocus('local');\n}\n\nexport function MultiLineParameterShadow(\n  formatFocus: (id: string) => string\n) {\n  return formatFocus('local');\n}\n\nexport function LaterMultiLineParameterShadow(\n  makeFocus: (id: string) => string,\n  formatFocus: (id: string) => string\n) {\n  return formatFocus('local');\n}\n\nexport const MultiLineArrowShadow = (\n  formatFocus: (id: string) => string\n) => formatFocus('local');\n\nexport const LaterMultiLineArrowShadow = (\n  makeFocus: (id: string) => string,\n  formatFocus: (id: string) => string\n) => formatFocus('local');\n\nexport function MultiLineDestructureShadow(props: { formatFocus: (id: string) => string }) {\n  const {\n    formatFocus,\n  } = props;\n  return formatFocus('local');\n}\n\nexport function LocalConstShadow() {\n  const formatFocus = (id: string) => id.toUpperCase();\n  return formatFocus('local');\n}\n",
    );
    write(
        &repo.path().join("src/local-flow.test.tsx"),
        "import { SelectionPanel } from './local-flow';\n\ntest('selection panel export stays usable', () => {\n  expect(SelectionPanel).toBeDefined();\n});\n",
    );
    write(
        &repo.path().join("tests/support/local-flow-page.tsx"),
        "import { SelectionPanel } from '../../src/local-flow';\n\nexport function renderSelectionPanel() {\n  return SelectionPanel;\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "local symbol flow fixture"]);

    let file_ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", "src/local-flow.tsx", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &file_ls);
    assert!(
        file_ls["anchor"]["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .all(|symbol| symbol["name"] != "focus"),
        "default TSX file map should hide local constants inside functions: {file_ls:#}"
    );
    assert!(
        file_ls["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|group| group["reason"] == "nested symbols hidden by default"),
        "hidden group should distinguish default symbol filtering from limit truncation: {file_ls:#}"
    );
    let full_file_ls = run_json(
        repo.path(),
        cache.path(),
        &[
            "ls",
            "src/local-flow.tsx",
            "--all",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/ls.schema.json", &full_file_ls);
    assert!(
        full_file_ls["anchor"]["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .any(|symbol| symbol["name"] == "focus" && symbol["kind"] == "const"),
        "include-hidden should expose local TSX constants on demand: {full_file_ls:#}"
    );

    let panel_cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "src/local-flow.tsx#SelectionPanel",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &panel_cone);
    assert!(
        panel_cone["outgoing"]
            .as_array()
            .expect("panel outgoing")
            .iter()
            .any(|edge| edge["from"] == "src/local-flow.tsx#SelectionPanel"
                && edge["to"] == "src/local-flow.tsx#chooseFocus"
                && edge["type"] == "symbol_uses"
                && edge["evidence"] == "local_symbol_in_symbol_body"),
        "symbol cone should show same-file symbol uses without requiring import edges: {panel_cone:#}"
    );
    assert!(
        panel_cone["unknowns"]
            .as_array()
            .expect("panel unknowns")
            .is_empty(),
        "same-file symbol uses should make the symbol cone structurally grounded: {panel_cone:#}"
    );

    let choose_cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/local-flow.tsx#chooseFocus", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &choose_cone);
    assert!(
        choose_cone["outgoing"]
            .as_array()
            .expect("choose outgoing")
            .iter()
            .any(|edge| edge["from"] == "src/local-flow.tsx#chooseFocus"
                && edge["to"] == "src/local-flow.tsx#formatFocus"
                && edge["evidence"] == "local_symbol_in_symbol_body"),
        "same-file helper chains should be visible at symbol level: {choose_cone:#}"
    );
    assert!(
        choose_cone["incoming"]
            .as_array()
            .expect("choose incoming")
            .iter()
            .any(|edge| edge["from"] == "src/local-flow.tsx#SelectionPanel"
                && edge["to"] == "src/local-flow.tsx#chooseFocus"
                && edge["evidence"] == "local_symbol_in_symbol_body"),
        "symbol cone should show same-file symbols that consume the selected helper: {choose_cone:#}"
    );

    let choose_proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "src/local-flow.tsx#chooseFocus",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &choose_proof);
    assert!(
        choose_proof["proofs"]
            .as_array()
            .expect("choose proofs")
            .iter()
            .any(|surface| surface["path"] == "src/local-flow.test.tsx"
                && surface["evidence"]
                    == "test_imported_symbol_reference_via_local_symbol_consumer"
                && surface["strength"] == "medium"),
        "proof should traverse exact tests of same-file symbol consumers before broad fallback: {choose_proof:#}"
    );
    assert!(
        choose_proof["proofs"]
            .as_array()
            .expect("choose proofs")
            .iter()
            .all(|surface| surface["path"] != "tests/support/local-flow-page.tsx"),
        "test support files may explain chains but must not become runnable proof surfaces: {choose_proof:#}"
    );
    assert!(
        choose_proof["fallback"]
            .as_array()
            .expect("choose fallback")
            .is_empty(),
        "same-file consumer proof should suppress broad fallback: {choose_proof:#}"
    );
    assert!(
        choose_proof["unknowns"]
            .as_array()
            .expect("choose unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "direct_test_import_not_found"
                && unknown["path"] == "src/local-flow.tsx#chooseFocus"),
        "mediated symbol-consumer surface must not hide the missing direct verification surface for the selected symbol: {choose_proof:#}"
    );

    let argument_cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/local-flow.tsx#ArgumentUse", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &argument_cone);
    let argument_outgoing = argument_cone["outgoing"]
        .as_array()
        .expect("argument outgoing");
    assert!(
        argument_outgoing
            .iter()
            .any(|edge| edge["to"] == "src/local-flow.tsx#combineFocus"
                && edge["evidence"] == "local_symbol_in_symbol_body"),
        "same-file function calls should be visible from selected symbol bodies: {argument_cone:#}"
    );
    assert!(
        argument_outgoing
            .iter()
            .any(|edge| edge["to"] == "src/local-flow.tsx#formatFocus"
                && edge["evidence"] == "local_symbol_in_symbol_body"),
        "same-file symbol references passed as later call arguments must not be mistaken for local bindings: {argument_cone:#}"
    );

    for false_anchor in [
        "src/local-flow.tsx#ParameterShadow",
        "src/local-flow.tsx#MultiLineParameterShadow",
        "src/local-flow.tsx#LaterMultiLineParameterShadow",
        "src/local-flow.tsx#MultiLineArrowShadow",
        "src/local-flow.tsx#LaterMultiLineArrowShadow",
        "src/local-flow.tsx#MultiLineDestructureShadow",
        "src/local-flow.tsx#LocalConstShadow",
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
                .all(|edge| edge["to"] != "src/local-flow.tsx#formatFocus"),
            "local params or const bindings must not become same-file symbol edges for {false_anchor}: {false_cone:#}"
        );
    }
}
