#[test]
fn symbol_anchor_cone_rejects_inexact_type_only_and_shadowed_barrel_reexports() {
    let (repo, cache) = barrel_negative_fixture();

    let value_cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/features/studio/canvas/selection-core.ts#pickFocusForSelection",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &value_cone);
    for false_consumer in [
        "packages/app/src/features/studio/canvas/not-reexported-consumer.ts",
        "packages/app/src/features/studio/canvas/shadowed-consumer.ts",
        "packages/app/src/features/studio/canvas/conflict-consumer.ts",
        "packages/app/src/features/studio/canvas/same-file-override-consumer.ts",
        "packages/app/src/features/studio/canvas/commented-reexport-consumer.ts",
        "packages/app/src/features/studio/canvas/duplicate-star-consumer.ts",
        "packages/app/src/features/studio/canvas/transitive-duplicate-consumer.ts",
        "packages/app/src/features/studio/canvas/transitive-local-override-consumer.ts",
        "packages/app/src/features/studio/canvas/cycle-consumer.ts",
        "packages/app/src/features/studio/canvas/local-consumer.ts",
        "packages/app/src/features/studio/canvas/multiline-local-consumer.ts",
        "packages/app/src/features/studio/canvas/commented-local-consumer.ts",
    ] {
        assert!(
            value_cone["incoming"]
                .as_array()
                .expect("incoming")
                .iter()
                .all(|edge| edge["from"] != false_consumer),
            "barrel xref must not link inexact or locally shadowed consumers: {value_cone:#}"
        );
    }

    let other_symbol_cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/features/studio/canvas/selection-core.ts#otherSymbol",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &other_symbol_cone);
    assert!(
        other_symbol_cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .any(|edge| edge["from"]
                == "packages/app/src/features/studio/canvas/same-file-override-consumer.ts"
                && edge["evidence"] == "reexported_symbol_reference"),
        "explicit same-file re-export should resolve to the exact imported symbol binding: {other_symbol_cone:#}"
    );
    assert!(
        other_symbol_cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .any(|edge| edge["from"]
                == "packages/app/src/features/studio/canvas/commented-reexport-consumer.ts"
                && edge["evidence"] == "reexported_symbol_reference"),
        "comments inside re-export clauses must not become imported symbol bindings: {other_symbol_cone:#}"
    );

    let default_cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/features/studio/canvas/default-core.ts#PickFocus",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &default_cone);
    assert!(
        default_cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .all(|edge| edge["from"]
                != "packages/app/src/features/studio/canvas/default-star-consumer.ts"),
        "export-star must not expose default export symbol names as named public exports: {default_cone:#}"
    );
    assert!(
        default_cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .all(|edge| edge["from"]
                != "packages/app/src/features/studio/canvas/default-transitive-star-consumer.ts"),
        "transitive export-star must not expose default export symbol names as named public exports: {default_cone:#}"
    );
    assert!(
        default_cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .any(|edge| edge["from"]
                == "packages/app/src/features/studio/canvas/default-named-consumer.ts"
                && edge["evidence"] == "reexported_symbol_reference"),
        "explicit default-as named re-export should still link to the default symbol: {default_cone:#}"
    );

    let default_list_cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/features/studio/canvas/default-list-core.ts#pickFocusForSelection",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &default_list_cone);
    assert!(
        default_list_cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .all(|edge| edge["from"]
                != "packages/app/src/features/studio/canvas/default-list-star-consumer.ts"),
        "export-star must not expose target-side default export-list aliases as named public exports: {default_list_cone:#}"
    );

    for (anchor, false_consumer, false_test) in [
        (
            "packages/app/src/features/studio/canvas/fake-string-core.ts#pickFocusForSelection",
            "packages/app/src/features/studio/canvas/fake-string-consumer.ts",
            "packages/app/src/features/studio/canvas/fake-string-core.test.ts",
        ),
        (
            "packages/app/src/features/studio/canvas/fake-comment-core.ts#pickFocusForSelection",
            "packages/app/src/features/studio/canvas/fake-comment-consumer.ts",
            "packages/app/src/features/studio/canvas/fake-comment-core.test.ts",
        ),
        (
            "packages/app/src/features/studio/canvas/fake-regex-core.ts#pickFocusForSelection",
            "packages/app/src/features/studio/canvas/fake-regex-consumer.ts",
            "packages/app/src/features/studio/canvas/fake-regex-core.test.ts",
        ),
        (
            "packages/app/src/features/studio/canvas/comment-gap-core.ts#localPick",
            "packages/app/src/features/studio/canvas/comment-gap-consumer.ts",
            "packages/app/src/features/studio/canvas/comment-gap-core.test.ts",
        ),
    ] {
        let fake_cone = run_json(
            repo.path(),
            cache.path(),
            &["cone", anchor, "--format", "json"],
        );
        assert_schema("schemas/cone.schema.json", &fake_cone);
        assert!(
            fake_cone["incoming"]
                .as_array()
                .expect("incoming")
                .iter()
                .all(|edge| edge["from"] != false_consumer),
            "export-list text inside strings/comments must not create re-exported symbol xrefs for {anchor}: {fake_cone:#}"
        );
        assert!(
            fake_cone["proof"]
                .as_array()
                .expect("proof")
                .iter()
                .all(|edge| edge["from"] != false_test),
            "export-list text inside strings/comments must not create proof edges for {anchor}: {fake_cone:#}"
        );
    }

    let type_cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/features/studio/canvas/selection-core.ts#SelectionFocus",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &type_cone);
    assert!(
        type_cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .all(|edge| edge["from"]
                != "packages/app/src/features/studio/canvas/type-only-consumer.ts"),
        "type-only re-export/import must not become a runtime symbol xref: {type_cone:#}"
    );
}

