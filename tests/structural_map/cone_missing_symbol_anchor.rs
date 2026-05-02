#[test]
fn cone_missing_symbol_anchor_fails_closed() {
    let (repo, cache) = fixture();

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/replay/src/session.ts#NotARealSymbol",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert_eq!(
        cone["anchor"]["path"],
        "packages/replay/src/session.ts#NotARealSymbol"
    );
    assert_eq!(cone["anchor"]["kind"], "missing_symbol");
    assert!(
        cone["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| {
                unknown["kind"] == "missing_symbol_anchor"
                    && unknown["path"] == "packages/replay/src/session.ts"
            }),
        "missing symbol anchor must be explicit instead of looking like a missing file: {cone:#}"
    );
    for section in ["outgoing", "incoming", "proof", "contracts", "boundary"] {
        assert!(
            cone[section].as_array().expect(section).is_empty(),
            "missing symbol anchor must not silently fall back to whole-file cone edges in {section}: {cone:#}"
        );
    }
}
