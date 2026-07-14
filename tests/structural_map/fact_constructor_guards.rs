#[test]
fn shared_surface_constructors_are_used_by_first_boundary_lenses() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let facts = fs::read_to_string(root.join("src/map/facts.rs")).expect("facts source");
    assert!(
        facts.contains("fn surface(")
            && facts.contains("fn surface_from_path(")
            && facts.contains("fn exported_symbol_surface(")
            && facts.contains("fn proof_surface("),
        "shared fact constructors should live in src/map/facts.rs"
    );

    for rel in [
        "src/map/lenses/contract.rs",
        "src/map/lenses/runtime/root_containers.rs",
    ] {
        let source = fs::read_to_string(root.join(rel)).expect("lens source");
        assert!(
            !source.contains("Surface {"),
            "{rel} should use shared surface constructors instead of ad-hoc Surface literals"
        );
    }

    let proof_map = fs::read_to_string(root.join("src/map/lenses/proof_map/current_level.rs"))
        .expect("proof-map source");
    assert!(
        !proof_map.contains("ProofSurface {\n        command"),
        "current-level proof containers should use shared proof_surface constructor"
    );
}
