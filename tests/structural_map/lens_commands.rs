#[test]
fn new_lenses_return_deterministic_structural_maps() {
    let (repo, cache) = fixture();

    let contract = run_json(
        repo.path(),
        cache.path(),
        &[
            "contract",
            "packages/replay/src/session.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/contract.schema.json", &contract);
    assert_eq!(contract["kind"], "contract_report");
    assert!(
        contract["consumers"]
            .as_array()
            .expect("contract consumers")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/src/index.ts"),
        "contract lens should show structural consumers, not rank files: {contract:#}"
    );

    let runtime = run_json(repo.path(), cache.path(), &["runtime", ".", "--format", "json"]);
    assert_schema("schemas/runtime.schema.json", &runtime);
    assert_eq!(runtime["kind"], "runtime_report");
    assert!(
        runtime["scripts"]
            .as_array()
            .expect("runtime scripts")
            .iter()
            .any(|surface| surface["kind"] == "script"),
        "runtime lens should expose package scripts as runtime surfaces: {runtime:#}"
    );

    let boundary_map = run_json(
        repo.path(),
        cache.path(),
        &["boundary-map", ".", "--format", "json"],
    );
    assert_schema("schemas/boundary-map.schema.json", &boundary_map);
    assert!(
        boundary_map["actual_cross_edges"]
            .as_array()
            .expect("cross edges")
            .iter()
            .any(|edge| edge["from"] == "packages/app/src/badInternal.ts"
                && edge["to"] == "packages/replay/src/internal.ts"),
        "boundary-map should show actual cross-package imports as a map: {boundary_map:#}"
    );

    let delete_map = run_json(
        repo.path(),
        cache.path(),
        &["delete", "packages/replay/src/session.ts", "--format", "json"],
    );
    assert_schema("schemas/delete.schema.json", &delete_map);
    assert!(
        delete_map["direct_users"]
            .as_array()
            .expect("direct users")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/src/index.ts"),
        "delete lens should show blockers instead of claiming safety: {delete_map:#}"
    );
    assert_eq!(delete_map.get("safe_to_delete"), None);

    let siblings = run_json(
        repo.path(),
        cache.path(),
        &["siblings", "packages/replay/src", "--format", "json"],
    );
    assert_schema("schemas/siblings.schema.json", &siblings);
    assert!(
        !siblings["same_kind"]
            .as_array()
            .expect("same kind")
            .is_empty(),
        "siblings lens should show local structural groups: {siblings:#}"
    );

    let place = run_json(
        repo.path(),
        cache.path(),
        &[
            "place",
            "packages/replay",
            "--kind",
            "test",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/place.schema.json", &place);
    assert_eq!(place["requested_kind"], "test");
    assert!(
        place["existing_surfaces"]
            .as_array()
            .expect("existing surfaces")
            .iter()
            .any(|surface| surface["examples"]
                .as_array()
                .is_some_and(|examples| examples.iter().any(|example| example
                    == "packages/replay/tests/session.test.ts"))),
        "place lens should show existing local placement convention: {place:#}"
    );
}

#[test]
fn edge_locations_and_typed_unknowns_are_first_class() {
    let (repo, cache) = fixture();

    let ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/replay/src/session.ts", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &ls);
    let import_edge = ls["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .find(|edge| edge["type"] == "imports")
        .expect("import edge");
    assert!(
        !import_edge["locations"]
            .as_array()
            .expect("locations")
            .is_empty(),
        "import edges must carry evidence locations: {ls:#}"
    );

    let cone = run_json(repo.path(), cache.path(), &["cone", ".", "--format", "json"]);
    assert_schema("schemas/cone.schema.json", &cone);
    assert!(
        cone["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "directory_aggregate"
                && unknown["effect"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("file-level edges")),
        "unknowns must be typed map facts, not free-form strings: {cone:#}"
    );
}

#[test]
fn diff_map_uses_selected_git_delta_mode() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/staged-delta.ts"),
        "import { Timeline } from './timeline';\n\nexport const stagedDelta = new Timeline();\n",
    );
    git(repo.path(), &["add", "packages/replay/src/staged-delta.ts"]);

    let staged = run_json(repo.path(), cache.path(), &["diff-map", "--staged", "--format", "json"]);
    assert_schema("schemas/diff-map.schema.json", &staged);
    assert!(
        staged["added_edges"]
            .as_array()
            .expect("staged added edges")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/src/staged-delta.ts"
                && edge["type"] == "added_structural_line"
                && edge["locations"][0]["kind"] == "diff_added_line:1"),
        "diff-map --staged must read the staged delta, not the unstaged working tree: {staged:#}"
    );

    git(repo.path(), &["commit", "-qm", "add staged delta fixture"]);
    let since = run_json(
        repo.path(),
        cache.path(),
        &["diff-map", "--since", "HEAD~1", "--format", "json"],
    );
    assert_schema("schemas/diff-map.schema.json", &since);
    assert!(
        since["added_edges"]
            .as_array()
            .expect("since added edges")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/src/staged-delta.ts"
                && edge["type"] == "added_structural_line"
                && edge["locations"][0]["kind"] == "diff_added_line:1"),
        "diff-map --since must read the selected base delta, not the ambient working tree: {since:#}"
    );
}

#[test]
fn diff_map_changed_includes_untracked_new_file_structural_lines() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/untracked-delta.ts"),
        "import { Timeline } from './timeline';\n\nexport const untrackedDelta = new Timeline();\n",
    );

    let changed = run_json(repo.path(), cache.path(), &["diff-map", "--changed", "--format", "json"]);
    assert_schema("schemas/diff-map.schema.json", &changed);
    assert!(
        changed["added_edges"]
            .as_array()
            .expect("changed added edges")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/src/untracked-delta.ts"
                && edge["type"] == "added_structural_line"
                && edge["locations"][0]["kind"] == "diff_added_line:1"),
        "diff-map --changed must synthesize structural lines for untracked files selected by git status: {changed:#}"
    );
    assert!(
        changed["added_exports"]
            .as_array()
            .expect("changed added exports")
            .iter()
            .any(|surface| surface["path"] == "packages/replay/src/untracked-delta.ts"),
        "diff-map --changed must expose export surfaces for untracked files: {changed:#}"
    );
}

#[test]
fn delete_missing_symbol_anchor_fails_closed() {
    let (repo, cache) = fixture();

    let delete_map = run_json(
        repo.path(),
        cache.path(),
        &[
            "delete",
            "packages/replay/src/session.ts#NotARealSymbol",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/delete.schema.json", &delete_map);
    assert_eq!(
        delete_map["anchor"]["path"],
        "packages/replay/src/session.ts#NotARealSymbol"
    );
    assert!(
        delete_map["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "missing_symbol_anchor"),
        "missing symbol anchor must be an explicit unknown, not a file-level fallback: {delete_map:#}"
    );
    assert!(
        delete_map["direct_users"]
            .as_array()
            .expect("direct users")
            .is_empty(),
        "missing symbol anchor must not silently show whole-file deletion blockers: {delete_map:#}"
    );
}
