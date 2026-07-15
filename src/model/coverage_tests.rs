use super::*;

fn certificate(
    scope: &str,
    closure: CoverageClosure,
    reasons: Vec<CoverageReason>,
) -> CoverageCertificate {
    CoverageCertificate::new(
        "symbol_consumers",
        scope,
        "snapshot-1",
        3,
        3,
        closure,
        reasons,
    )
}

fn record(
    ledger: &mut ObservationLedger,
    group: &str,
    scope: &str,
    observed: u64,
    shown: u64,
    certificate: CoverageCertificate,
    expand: Option<&str>,
) -> ObservedCount {
    ledger.record(
        group,
        scope,
        observed,
        shown,
        certificate,
        expand.map(str::to_string),
    )
}

#[test]
fn closed_zero_is_proven_and_its_certificate_is_registered() {
    let mut ledger = ObservationLedger::default();
    let count = record(
        &mut ledger,
        "consumers",
        "src/lib.rs#Thing",
        0,
        0,
        certificate("src/lib.rs#Thing", CoverageClosure::Closed, Vec::new()),
        None,
    );

    assert_eq!(count.closure, CoverageClosure::Closed);
    assert_eq!(count.display(), "proven-zero");
    assert!(ledger.certificate(&count.certificate_id).is_some());
    let json = serde_json::to_value(&count).expect("count json");
    for field in ["observed", "closure", "reasons", "certificate_id"] {
        assert!(json.get(field).is_some(), "missing {field}: {json}");
    }
}

#[test]
fn open_positive_count_keeps_lower_bound_and_has_canonical_id() {
    let stops = vec![
        CoverageStop {
            kind: CoverageReason::RustIncludeFlow,
            location: Some(CoverageLocation::path("src/include.rs")),
            missing_surface: None,
        },
        CoverageStop {
            kind: CoverageReason::ReexportFlow,
            location: Some(CoverageLocation::path("src/lib.rs")),
            missing_surface: None,
        },
    ];
    let mut first_certificate = certificate("src/a.rs#Thing", CoverageClosure::Open, Vec::new());
    first_certificate.unresolved_stops = stops.clone();
    let mut second_certificate = certificate("src/a.rs#Thing", CoverageClosure::Open, Vec::new());
    second_certificate.unresolved_stops = stops.into_iter().rev().collect();
    let mut first = ObservationLedger::default();
    let first_count = record(
        &mut first,
        "consumers",
        "src/a.rs#Thing",
        4,
        4,
        first_certificate,
        None,
    );
    let mut second = ObservationLedger::default();
    let second_count = record(
        &mut second,
        "consumers",
        "src/a.rs#Thing",
        4,
        4,
        second_certificate,
        None,
    );

    assert_eq!(first_count.closure, CoverageClosure::Open);
    assert!(first_count.display().starts_with("counted-at-least(4,"));
    assert_eq!(first_count.certificate_id, second_count.certificate_id);
}

#[test]
fn unavailable_does_not_gain_incomplete_traversal_noise() {
    let mut unavailable = certificate(
        "missing.rs#Thing",
        CoverageClosure::Unavailable,
        vec![CoverageReason::AnchorNotIndexed],
    );
    unavailable.eligible_files = 0;
    unavailable.visited_files = 0;
    let mut ledger = ObservationLedger::default();
    let count = record(
        &mut ledger,
        "consumers",
        "missing.rs#Thing",
        0,
        0,
        unavailable,
        None,
    );

    assert_eq!(count.closure, CoverageClosure::Unavailable);
    assert_eq!(count.reasons, vec![CoverageReason::AnchorNotIndexed]);
}

#[test]
#[should_panic(expected = "coverage exclusions must account for every unvisited eligible file")]
fn record_rejects_unexplained_unvisited_files() {
    let mut invalid = certificate("src/a.rs#Thing", CoverageClosure::Open, Vec::new());
    invalid.visited_files = invalid.eligible_files - 1;
    record(
        &mut ObservationLedger::default(),
        "consumers",
        "src/a.rs#Thing",
        0,
        0,
        invalid,
        None,
    );
}

#[test]
fn record_derives_hidden_and_copies_gap_details() {
    let mut basis = certificate("src/a.rs#Thing", CoverageClosure::Open, Vec::new());
    basis.dynamic_stops.push(CoverageStop {
        kind: CoverageReason::DynamicImportFlow,
        location: Some(CoverageLocation::path("src/load.ts")),
        missing_surface: Some("selected consumer".to_string()),
    });
    basis.unsupported.push(UnsupportedObservation {
        file: "src/opaque.ts".to_string(),
        construct: "decorator import".to_string(),
        location: None,
    });
    basis.visited_files -= 1;
    basis.excluded_files_by_reason.insert(
        CoverageReason::UnsupportedConstruct,
        vec!["src/opaque.ts".to_string()],
    );
    let mut ledger = ObservationLedger::default();
    record(
        &mut ledger,
        "consumers",
        "src/a.rs#Thing",
        8,
        3,
        basis,
        Some("codemap cone src/a.rs#Thing --all"),
    );

    let horizon = ledger.horizon("consumers").expect("consumer horizon");
    assert_eq!((horizon.shown, horizon.hidden), (3, 5));
    assert_eq!(horizon.dynamic.len(), 1);
    assert_eq!(horizon.unsupported.len(), 1);
}

#[test]
fn unsupported_observation_forces_an_erroneous_closed_certificate_open() {
    let mut basis = certificate("src/a.rs#Thing", CoverageClosure::Closed, Vec::new());
    basis.unsupported.push(UnsupportedObservation {
        file: "src/a.rs".to_string(),
        construct: "unsupported macro expansion".to_string(),
        location: Some(CoverageLocation::path("src/a.rs")),
    });
    let mut ledger = ObservationLedger::default();
    let count = record(
        &mut ledger,
        "consumers",
        "src/a.rs#Thing",
        0,
        0,
        basis,
        None,
    );

    assert_eq!(count.closure, CoverageClosure::Open);
    assert_eq!(count.reasons, vec![CoverageReason::UnsupportedConstruct]);
    assert_eq!(
        count.display(),
        "unknown lower bound: 0 (unsupported construct)"
    );
    assert_eq!(
        ledger
            .certificate(&count.certificate_id)
            .expect("sealed certificate")
            .closure,
        CoverageClosure::Open
    );
    assert!(ledger.validate().is_ok());
}

#[test]
#[should_panic(expected = "shown coverage cannot exceed observed")]
fn record_rejects_shown_above_observed() {
    record(
        &mut ObservationLedger::default(),
        "consumers",
        "src/a.rs#Thing",
        1,
        2,
        certificate("src/a.rs#Thing", CoverageClosure::Closed, Vec::new()),
        None,
    );
}

#[test]
#[should_panic(expected = "coverage expand must exist exactly when facts are hidden")]
fn record_rejects_expand_without_hidden_facts() {
    record(
        &mut ObservationLedger::default(),
        "consumers",
        "src/a.rs#Thing",
        1,
        1,
        certificate("src/a.rs#Thing", CoverageClosure::Closed, Vec::new()),
        Some("codemap cone src/a.rs#Thing --all"),
    );
}

#[test]
#[should_panic(expected = "coverage expand must exist exactly when facts are hidden")]
fn record_rejects_hidden_facts_without_expand() {
    record(
        &mut ObservationLedger::default(),
        "consumers",
        "src/a.rs#Thing",
        2,
        1,
        certificate("src/a.rs#Thing", CoverageClosure::Closed, Vec::new()),
        None,
    );
}

#[test]
#[should_panic(expected = "coverage visited files cannot exceed eligible files")]
fn record_rejects_more_visited_than_eligible_files() {
    let mut invalid = certificate("src/a.rs#Thing", CoverageClosure::Closed, Vec::new());
    invalid.visited_files = invalid.eligible_files + 1;
    record(
        &mut ObservationLedger::default(),
        "consumers",
        "src/a.rs#Thing",
        0,
        0,
        invalid,
        None,
    );
}

#[test]
fn merge_preserves_every_horizon_certificate_reference() {
    let mut first = ObservationLedger::default();
    record(
        &mut first,
        "incoming",
        "src/a.rs#Thing",
        1,
        1,
        certificate("src/a.rs#Thing", CoverageClosure::Closed, Vec::new()),
        None,
    );
    let mut second = ObservationLedger::default();
    record(
        &mut second,
        "consumers",
        "src/b.rs#Other",
        0,
        0,
        certificate("src/b.rs#Other", CoverageClosure::Closed, Vec::new()),
        None,
    );

    first.merge(&second);
    assert_eq!(
        first
            .horizons
            .iter()
            .map(|horizon| horizon.group.as_str())
            .collect::<Vec<_>>(),
        vec!["consumers", "incoming"]
    );
    assert!(
        first
            .horizons
            .iter()
            .all(|horizon| first.certificate(&horizon.count.certificate_id).is_some())
    );
}
