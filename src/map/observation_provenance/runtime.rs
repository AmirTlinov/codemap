// Responsibility: runtime-route-observation-provenance
use std::collections::{BTreeMap, BTreeSet};

use crate::map::runtime_route_extractor_capability;
use crate::model::{
    CoverageCertificate, CoverageClosure, CoverageLocation, CoverageReason, CoverageStop, FileInfo,
    ObservationLedger, Project, Unknown, UnsupportedObservation,
};

/// Exact inputs already owned by runtime report assembly. This projection does
/// not rediscover scope or recompute route facts; it only certifies what that
/// report observed and chose to show.
pub(crate) struct RuntimeRouteObservationInput<'a> {
    pub scope: &'a str,
    pub scope_indexed: bool,
    pub candidate_files: &'a [&'a FileInfo],
    pub observed: usize,
    pub shown: usize,
    pub route_unknowns: &'a [Unknown],
    pub expand: Option<String>,
}

pub(crate) fn runtime_route_observations(
    project: &Project,
    input: RuntimeRouteObservationInput<'_>,
) -> ObservationLedger {
    if !input.scope_indexed {
        let certificate = CoverageCertificate::new(
            "supported_static_runtime_routes",
            input.scope,
            crate::cache::fingerprint(project, None),
            0,
            0,
            CoverageClosure::Unavailable,
            vec![CoverageReason::AnchorNotIndexed],
        );
        let mut ledger = ObservationLedger::default();
        ledger.record("routes", input.scope, 0, 0, certificate, None);
        return ledger;
    }
    let candidates = input
        .candidate_files
        .iter()
        .copied()
        .map(|file| (file.rel.as_str(), file))
        .collect::<BTreeMap<_, _>>();
    let mut capabilities = Vec::new();
    let mut unsupported = Vec::new();
    let mut excluded_files_by_reason = BTreeMap::new();
    let mut visited_files = 0_u64;

    for file in candidates.values() {
        match runtime_route_extractor_capability(file) {
            Ok(capability) => {
                visited_files += 1;
                capabilities.push(capability);
            }
            Err((reason, construct)) => {
                excluded_files_by_reason
                    .entry(reason)
                    .or_insert_with(Vec::new)
                    .push(file.rel.clone());
                unsupported.push(UnsupportedObservation {
                    file: file.rel.clone(),
                    construct,
                    location: Some(CoverageLocation::path(&file.rel)),
                });
            }
        }
    }

    let mut dynamic_stops = Vec::new();
    let mut seen_unknowns = BTreeSet::new();
    for unknown in input.route_unknowns {
        let key = (
            unknown.kind.as_str(),
            unknown.path.as_deref(),
            unknown.line_start,
            unknown.reason.as_str(),
            unknown.effect.as_str(),
        );
        if !seen_unknowns.insert(key) {
            continue;
        }
        if unknown.kind == "unsupported_framework_route" {
            unsupported.push(UnsupportedObservation {
                file: unknown
                    .path
                    .clone()
                    .unwrap_or_else(|| input.scope.to_string()),
                construct: unknown.kind.clone(),
                location: unknown_location(unknown),
            });
        } else if unknown.kind.starts_with("route_") {
            dynamic_stops.push(CoverageStop {
                kind: CoverageReason::DynamicRuntimeRegistration,
                location: unknown_location(unknown),
                missing_surface: Some(format!(
                    "{}: {}; {}",
                    unknown.kind, unknown.effect, unknown.reason
                )),
            });
        }
    }

    let closure = if unsupported.is_empty() && dynamic_stops.is_empty() {
        CoverageClosure::Closed
    } else {
        CoverageClosure::Open
    };
    let mut certificate = CoverageCertificate::new(
        "supported_static_runtime_routes",
        input.scope,
        crate::cache::fingerprint(project, None),
        candidates.len() as u64,
        visited_files,
        closure,
        Vec::new(),
    );
    certificate.extractor_capabilities = capabilities;
    certificate.unsupported = unsupported;
    certificate.dynamic_stops = dynamic_stops;
    certificate.excluded_files_by_reason = excluded_files_by_reason;

    let mut ledger = ObservationLedger::default();
    ledger.record(
        "routes",
        input.scope,
        input.observed as u64,
        input.shown as u64,
        certificate,
        input.expand,
    );
    ledger
}

fn unknown_location(unknown: &Unknown) -> Option<CoverageLocation> {
    unknown.path.as_ref().map(|path| CoverageLocation {
        path: path.clone(),
        line_start: unknown.line_start,
        line_end: unknown.line_start,
    })
}
