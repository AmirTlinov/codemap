struct FixtureMatrixCase {
    name: &'static str,
    file_anchor: &'static str,
    dir_anchor: &'static str,
    contract_anchor: &'static str,
    delete_anchor: &'static str,
    flow_anchor: &'static str,
    unknown_anchor: &'static str,
    unknown_kind: &'static str,
    place_scope: &'static str,
    place_kind: &'static str,
    dirty_file: &'static str,
    dirty_append: &'static str,
}

#[test]
fn named_fixture_matrix_covers_public_lenses() {
    for case in fixture_matrix_cases() {
        let (repo, cache) = fixture_matrix_repo(case.name);

        let root_ls = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
        assert_schema("schemas/ls.schema.json", &root_ls);
        assert!(
            root_ls["directory"].as_array().expect("root directory").len() <= 20,
            "root ls for {} must stay bounded: {root_ls:#}",
            case.name
        );
        assert_root_ls_has_no_recursive_source_examples(case.name, &root_ls);

        let graph = run_json(
            repo.path(),
            cache.path(),
            &["graph", "--lens", "causal", "--format", "json"],
        );
        assert_schema("schemas/graph.schema.json", &graph);
        assert!(
            graph["nodes"].as_array().expect("graph nodes").len() <= 12,
            "root graph for {} must stay current-level bounded: {graph:#}",
            case.name
        );
        assert_root_graph_has_no_recursive_source_nodes(case.name, &graph);

        assert_schema(
            "schemas/ls.schema.json",
            &run_json(repo.path(), cache.path(), &["ls", case.dir_anchor, "--format", "json"]),
        );
        let file_cone = run_json(
            repo.path(),
            cache.path(),
            &["cone", case.file_anchor, "--format", "json"],
        );
        assert_schema("schemas/cone.schema.json", &file_cone);
        assert_ne!(
            file_cone["anchor"]["kind"], "missing",
            "file cone anchor should exist for {}: {file_cone:#}",
            case.name
        );
        let dir_cone = run_json(
            repo.path(),
            cache.path(),
            &["cone", case.dir_anchor, "--format", "json"],
        );
        assert_schema(
            "schemas/cone.schema.json",
            &dir_cone,
        );
        assert!(
            dir_cone["unknowns"]
                .as_array()
                .expect("dir cone unknowns")
                .iter()
                .any(|unknown| unknown["kind"] == "directory_aggregate"),
            "directory cone should stay aggregate-level for {}: {dir_cone:#}",
            case.name
        );

        let runtime = run_json(repo.path(), cache.path(), &["runtime", ".", "--format", "json"]);
        assert_schema("schemas/runtime.schema.json", &runtime);
        assert!(
            runtime["scripts"].as_array().expect("scripts").len()
                + runtime["entrypoints"].as_array().expect("entrypoints").len()
                + runtime["routes"].as_array().expect("routes").len()
                <= 20,
            "root runtime for {} must stay bounded: {runtime:#}",
            case.name
        );

        let contract = run_json(
            repo.path(),
            cache.path(),
            &["contract", case.contract_anchor, "--format", "json"],
        );
        assert_schema("schemas/contract.schema.json", &contract);
        assert_ne!(
            contract["anchor"]["kind"], "missing",
            "contract anchor should exist for {}: {contract:#}",
            case.name
        );

        let proof_map = run_json(repo.path(), cache.path(), &["proof-map", ".", "--format", "json"]);
        assert_schema("schemas/proof-map.schema.json", &proof_map);
        assert!(
            proof_map_has_sensor_or_hidden(&proof_map),
            "proof-map should expose proof containers/sensors or hidden expansion for {}: {proof_map:#}",
            case.name
        );

        let delete_map = run_json(
            repo.path(),
            cache.path(),
            &["delete", case.delete_anchor, "--format", "json"],
        );
        assert_schema("schemas/delete.schema.json", &delete_map);
        assert_eq!(
            delete_map.get("safe_to_delete"),
            None,
            "delete lens must not claim safety for {}",
            case.name
        );

        let boundary_map = run_json(
            repo.path(),
            cache.path(),
            &["boundary-map", ".", "--format", "json"],
        );
        assert_schema("schemas/boundary-map.schema.json", &boundary_map);
        assert!(
            !boundary_map["public_boundary_files"]
                .as_array()
                .expect("public boundary files")
                .is_empty(),
            "boundary-map should expose public boundary files for {}: {boundary_map:#}",
            case.name
        );

        let flow = run_json(
            repo.path(),
            cache.path(),
            &["flow", case.flow_anchor, "--format", "json"],
        );
        assert_schema("schemas/flow.schema.json", &flow);
        assert!(
            !flow["steps"].as_array().expect("flow steps").is_empty()
                || !flow["unknown_breaks"]
                    .as_array()
                    .expect("flow unknowns")
                    .is_empty(),
            "flow should expose a bounded path or explicit stop for {}: {flow:#}",
            case.name
        );

        let siblings = run_json(
            repo.path(),
            cache.path(),
            &["siblings", case.dir_anchor, "--format", "json"],
        );
        assert_schema("schemas/siblings.schema.json", &siblings);
        assert!(
            !siblings["same_kind"].as_array().expect("same kind").is_empty()
                || !siblings["shared_helpers"]
                    .as_array()
                    .expect("shared helpers")
                    .is_empty(),
            "siblings should expose local structural groups for {}: {siblings:#}",
            case.name
        );
        let place = run_json(
            repo.path(),
            cache.path(),
            &[
                "place",
                case.place_scope,
                "--kind",
                case.place_kind,
                "--format",
                "json",
            ],
        );
        assert_schema("schemas/place.schema.json", &place);
        assert!(
            !place["existing_surfaces"]
                .as_array()
                .expect("existing surfaces")
                .is_empty(),
            "place should expose existing local convention for {}: {place:#}",
            case.name
        );

        let unknown_runtime = run_json(
            repo.path(),
            cache.path(),
            &["runtime", case.unknown_anchor, "--format", "json"],
        );
        assert_schema("schemas/runtime.schema.json", &unknown_runtime);
        assert!(
            unknown_runtime["unknowns"]
                .as_array()
                .expect("runtime unknowns")
                .iter()
                .any(|unknown| unknown["kind"] == case.unknown_kind),
            "fixture {} should expose dynamic blind spot `{}`: {unknown_runtime:#}",
            case.name,
            case.unknown_kind
        );

        let dirty_path = repo.path().join(case.dirty_file);
        let mut dirty_text = std::fs::read_to_string(&dirty_path).expect("dirty fixture file");
        dirty_text.push_str(case.dirty_append);
        write(&dirty_path, &dirty_text);

        let diff = run_json(
            repo.path(),
            cache.path(),
            &["diff-map", "--changed", "--format", "json"],
        );
        assert_schema("schemas/diff-map.schema.json", &diff);
        assert!(
            diff["changed"]
                .as_array()
                .expect("changed")
                .iter()
                .any(|file| file["path"] == case.dirty_file),
            "diff-map should see changed fixture file for {}: {diff:#}",
            case.name
        );

        let impact = run_json(
            repo.path(),
            cache.path(),
            &["impact", "--changed", "--format", "json"],
        );
        assert_schema("schemas/impact.schema.json", &impact);
        assert!(
            impact["changed"]
                .as_array()
                .expect("impact changed")
                .iter()
                .any(|file| file["path"] == case.dirty_file),
            "impact should see changed fixture file for {}: {impact:#}",
            case.name
        );

        let changed = run_json(repo.path(), cache.path(), &["changed", "--format", "json"]);
        assert_schema("schemas/changed.schema.json", &changed);
        assert!(
            changed["changed"]
                .as_array()
                .expect("changed anchors")
                .iter()
                .any(|file| file["path"] == case.dirty_file),
            "changed should expose the dirty fixture file for {}: {changed:#}",
            case.name
        );

        let proof = run_json(repo.path(), cache.path(), &["proof", "changed", "--format", "json"]);
        assert_schema("schemas/proof.schema.json", &proof);
        assert!(
            proof["changed"]
                .as_array()
                .expect("proof changed anchors")
                .iter()
                .any(|path| path == case.dirty_file),
            "proof changed should keep the dirty fixture anchor for {}: {proof:#}",
            case.name
        );
    }
}

fn assert_root_ls_has_no_recursive_source_examples(name: &str, root_ls: &Value) {
    for surface in root_ls["directory"].as_array().expect("root directory") {
        for example in surface["examples"].as_array().expect("surface examples") {
            let example = example.as_str().expect("example string");
            assert!(
                !looks_like_recursive_source_file(example),
                "root ls for {name} must not dump recursive source file examples: {root_ls:#}"
            );
        }
    }
}

fn assert_root_graph_has_no_recursive_source_nodes(name: &str, graph: &Value) {
    for node in graph["nodes"].as_array().expect("graph nodes") {
        let node = node.as_str().expect("graph node string");
        assert!(
            !looks_like_recursive_source_file(node),
            "root graph for {name} must not dump recursive source file nodes: {graph:#}"
        );
    }
}

fn looks_like_recursive_source_file(value: &str) -> bool {
    value.contains('/')
        && matches!(
            Path::new(value).extension().and_then(|ext| ext.to_str()),
            Some("ts" | "tsx" | "js" | "jsx" | "py" | "go" | "rs" | "swift")
        )
}

fn proof_map_has_sensor_or_hidden(proof_map: &Value) -> bool {
    [
        "hard",
        "direct_evidence",
        "mediated_evidence",
        "soft_evidence",
        "setup_support",
        "commands",
        "hidden",
    ]
        .into_iter()
        .any(|section| {
            proof_map[section]
                .as_array()
                .is_some_and(|items| !items.is_empty())
        })
}

fn fixture_matrix_cases() -> Vec<FixtureMatrixCase> {
    vec![
        FixtureMatrixCase {
            name: "ts-monorepo",
            file_anchor: "packages/api/src/server.ts",
            dir_anchor: "packages/api",
            contract_anchor: "packages/contracts/src/auth.ts",
            delete_anchor: "packages/contracts/src/auth.ts",
            flow_anchor: "packages/api/src/server.ts",
            unknown_anchor: "packages/api/src/server.ts",
            unknown_kind: "route_string_concat",
            place_scope: "packages/api",
            place_kind: "test",
            dirty_file: "packages/api/src/server.ts",
            dirty_append: "\nexport const changedSurface = true;\n",
        },
        FixtureMatrixCase {
            name: "next-app",
            file_anchor: "app/api/login/route.ts",
            dir_anchor: "app/api",
            contract_anchor: "lib/auth.schema.ts",
            delete_anchor: "lib/auth.schema.ts",
            flow_anchor: "app/api/login/route.ts",
            unknown_anchor: "app/api/proxy/route.ts",
            unknown_kind: "dynamic_import",
            place_scope: ".",
            place_kind: "test",
            dirty_file: "app/api/login/route.ts",
            dirty_append: "\nexport const runtime = \"nodejs\";\n",
        },
        FixtureMatrixCase {
            name: "node-backend",
            file_anchor: "src/server.ts",
            dir_anchor: "src",
            contract_anchor: "src/contracts/user.ts",
            delete_anchor: "src/users.ts",
            flow_anchor: "src/server.ts",
            unknown_anchor: "src/server.ts",
            unknown_kind: "route_string_concat",
            place_scope: ".",
            place_kind: "test",
            dirty_file: "src/users.ts",
            dirty_append: "\nexport const changedSurface = true;\n",
        },
        FixtureMatrixCase {
            name: "python-fastapi",
            file_anchor: "app/main.py",
            dir_anchor: "app",
            contract_anchor: "app/schemas.py",
            delete_anchor: "app/schemas.py",
            flow_anchor: "app/main.py",
            unknown_anchor: "app/main.py",
            unknown_kind: "route_string_concat",
            place_scope: ".",
            place_kind: "test",
            dirty_file: "app/schemas.py",
            dirty_append: "\n\ndef changed_surface():\n    return True\n",
        },
        FixtureMatrixCase {
            name: "go-http-service",
            file_anchor: "internal/api/routes.go",
            dir_anchor: "internal/api",
            contract_anchor: "internal/api/contracts.go",
            delete_anchor: "internal/api/contracts.go",
            flow_anchor: "internal/api/routes.go",
            unknown_anchor: "internal/api/routes.go",
            unknown_kind: "route_dynamic_method",
            place_scope: ".",
            place_kind: "test",
            dirty_file: "internal/api/contracts.go",
            dirty_append: "\nfunc ChangedSurface() bool { return true }\n",
        },
        FixtureMatrixCase {
            name: "rust-cli",
            file_anchor: "src/main.rs",
            dir_anchor: "src",
            contract_anchor: "src/config.rs",
            delete_anchor: "src/config.rs",
            flow_anchor: "src/main.rs",
            unknown_anchor: "src/main.rs",
            unknown_kind: "env_dynamic_lookup",
            place_scope: ".",
            place_kind: "test",
            dirty_file: "src/config.rs",
            dirty_append: "\npub fn changed_surface() -> bool { true }\n",
        },
        FixtureMatrixCase {
            name: "mixed-monorepo",
            file_anchor: "domains/replay/src/replay-session.ts",
            dir_anchor: "domains/replay",
            contract_anchor: "domains/replay/src/replay-package-format-schema.ts",
            delete_anchor: "domains/replay/src/replay-session.ts",
            flow_anchor: "apps/web/src/runtime.ts",
            unknown_anchor: "apps/web/src/runtime.ts",
            unknown_kind: "env_dynamic_lookup",
            place_scope: "domains/replay",
            place_kind: "test",
            dirty_file: "domains/replay/src/replay-session.ts",
            dirty_append: "\nexport const changedSurface = true;\n",
        },
    ]
}

fn fixture_matrix_repo(name: &str) -> (TempDir, TempDir) {
    let repo = TempDir::new().expect("fixture matrix repo");
    let cache = TempDir::new().expect("fixture matrix cache");
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name);
    copy_fixture_matrix_dir(&source, repo.path());
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture matrix baseline"]);
    (repo, cache)
}

fn copy_fixture_matrix_dir(source: &Path, dest: &Path) {
    std::fs::create_dir_all(dest).expect("create fixture destination");
    for entry in std::fs::read_dir(source).expect("read fixture source") {
        let entry = entry.expect("fixture entry");
        let file_type = entry.file_type().expect("fixture file type");
        let target = dest.join(entry.file_name());
        if file_type.is_dir() {
            copy_fixture_matrix_dir(&entry.path(), &target);
        } else if file_type.is_file() {
            std::fs::copy(entry.path(), target).expect("copy fixture file");
        }
    }
}
