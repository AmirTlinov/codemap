// Responsibility: remaining-runtime-group-observation-provenance
use std::collections::{BTreeMap, BTreeSet};

use crate::model::{
    CoverageCertificate, CoverageClosure, CoverageLocation, CoverageReason, CoverageStop,
    ExtractorCapability, FileInfo, ObservationLedger, Project, ScanInventoryBoundary, Unknown,
    UnsupportedObservation,
};

mod capabilities;
mod scan_boundaries;
mod scripts;
use capabilities::*;
use scan_boundaries::apply_scan_boundary_stops;
use scripts::script_certificate;

pub(crate) struct RuntimeGroupProjection {
    pub group: &'static str,
    pub observed: usize,
    pub shown: usize,
    pub expand: Option<String>,
}

pub(crate) struct RuntimeGroupObservationInput<'a> {
    pub scope: &'a str,
    pub snapshot: &'a str,
    pub scope_indexed: bool,
    pub scope_logically_empty: bool,
    pub scope_has_unindexed_entries: bool,
    pub candidate_files: &'a [&'a FileInfo],
    pub incomplete_boundaries: &'a [&'a FileInfo],
    pub external_boundaries: &'a [&'a FileInfo],
    pub scan_boundaries: &'a [ScanInventoryBoundary],
    pub visited_files: &'a [&'a FileInfo],
    pub unknowns: &'a [Unknown],
    pub route_observations: &'a ObservationLedger,
    pub projections: Vec<RuntimeGroupProjection>,
}

pub(crate) fn runtime_group_observations(
    project: &Project,
    input: RuntimeGroupObservationInput<'_>,
) -> ObservationLedger {
    let visited = input
        .visited_files
        .iter()
        .map(|file| file.rel.as_str())
        .collect::<BTreeSet<_>>();
    let mut ledger = ObservationLedger::default();
    for projection in &input.projections {
        let certificate = if input.scope_indexed {
            group_certificate(project, &input, &visited, projection.group)
        } else {
            CoverageCertificate::new(
                query_kind(projection.group),
                input.scope,
                input.snapshot,
                0,
                0,
                CoverageClosure::Unavailable,
                vec![CoverageReason::AnchorNotIndexed],
            )
        };
        ledger.record(
            projection.group,
            input.scope,
            projection.observed as u64,
            projection.shown as u64,
            certificate,
            projection.expand.clone(),
        );
    }
    ledger
}

fn group_certificate(
    project: &Project,
    input: &RuntimeGroupObservationInput<'_>,
    visited: &BTreeSet<&str>,
    group: &str,
) -> CoverageCertificate {
    let mut certificate = match group {
        "entrypoints" => partial_category_certificate(
            file_certificate(
                input,
                visited,
                group,
                entrypoint_candidate,
                entrypoint_capability,
            ),
            input,
            "entrypoint extractors cover declared path, manifest, and Rust Clap forms only",
        ),
        "scripts" => script_certificate(project, input, visited),
        "env" => {
            let mut certificate =
                file_certificate(input, visited, group, source_candidate, env_capability);
            certificate.dynamic_stops.extend(
                input
                    .unknowns
                    .iter()
                    .filter(|unknown| unknown.kind == "env_dynamic_lookup")
                    .map(|unknown| CoverageStop {
                        kind: CoverageReason::UnsupportedConstruct,
                        location: unknown_location(unknown),
                        missing_surface: Some(format!(
                            "{}: {}; {}",
                            unknown.kind, unknown.effect, unknown.reason
                        )),
                    }),
            );
            if !certificate.dynamic_stops.is_empty() {
                certificate.closure = CoverageClosure::Open;
            }
            partial_category_certificate(
                certificate,
                input,
                "static environment extraction covers declared access forms, not arbitrary syntax",
            )
        }
        "workers" => file_certificate(
            input,
            visited,
            group,
            |_| true,
            |_| {
                Ok(capability(
                    "codemap.runtime-worker-job-paths",
                    "path",
                    &["worker_job_path_convention"],
                ))
            },
        ),
        "ci" => file_certificate(input, visited, group, |_| true, ci_capability),
        "proof" => proof_certificate(input, visited),
        "unknowns" => unknown_certificate(input, visited),
        _ => unreachable!("runtime group contract rejects unknown groups"),
    };
    apply_scan_boundary_stops(&mut certificate, input, group);
    certificate
}

fn partial_category_certificate(
    mut certificate: CoverageCertificate,
    input: &RuntimeGroupObservationInput<'_>,
    missing_surface: &str,
) -> CoverageCertificate {
    if !input.scope_logically_empty {
        certificate.closure = CoverageClosure::Open;
        certificate.unresolved_stops.push(CoverageStop {
            kind: CoverageReason::UnsupportedConstruct,
            location: Some(CoverageLocation::path(input.scope)),
            missing_surface: Some(missing_surface.to_string()),
        });
    }
    certificate
}

fn file_certificate<Eligible, Classify>(
    input: &RuntimeGroupObservationInput<'_>,
    visited: &BTreeSet<&str>,
    group: &str,
    eligible: Eligible,
    classify: Classify,
) -> CoverageCertificate
where
    Eligible: Fn(&FileInfo) -> bool,
    Classify: Fn(&FileInfo) -> Result<ExtractorCapability, (CoverageReason, String)>,
{
    let candidates = input
        .candidate_files
        .iter()
        .copied()
        .filter(|file| eligible(file))
        .collect::<Vec<_>>();
    let mut visited_count = 0_u64;
    let mut capabilities = Vec::new();
    let mut unsupported = Vec::new();
    let mut exclusions = BTreeMap::<CoverageReason, Vec<String>>::new();
    let incomplete_boundaries = input
        .incomplete_boundaries
        .iter()
        .map(|file| file.rel.as_str())
        .collect::<BTreeSet<_>>();
    for file in &candidates {
        if incomplete_boundaries.contains(file.rel.as_str()) {
            exclusions
                .entry(CoverageReason::IncompleteTraversal)
                .or_default()
                .push(file.rel.clone());
            continue;
        }
        if !visited.contains(file.rel.as_str()) {
            exclusions
                .entry(CoverageReason::IncompleteTraversal)
                .or_default()
                .push(file.rel.clone());
            continue;
        }
        match classify(file) {
            Ok(capability) => {
                visited_count += 1;
                capabilities.push(capability);
            }
            Err((reason, construct)) => {
                exclusions.entry(reason).or_default().push(file.rel.clone());
                unsupported.push(UnsupportedObservation {
                    file: file.rel.clone(),
                    construct,
                    location: Some(CoverageLocation::path(&file.rel)),
                });
            }
        }
    }
    let closure = if exclusions.is_empty() {
        CoverageClosure::Closed
    } else {
        CoverageClosure::Open
    };
    let mut certificate = CoverageCertificate::new(
        query_kind(group),
        input.scope,
        input.snapshot,
        candidates.len() as u64,
        visited_count,
        closure,
        Vec::new(),
    );
    certificate.extractor_capabilities = capabilities;
    certificate.unsupported = unsupported;
    certificate.excluded_files_by_reason = exclusions;
    certificate
}

fn proof_certificate(
    input: &RuntimeGroupObservationInput<'_>,
    visited: &BTreeSet<&str>,
) -> CoverageCertificate {
    let incomplete_boundaries = input
        .incomplete_boundaries
        .iter()
        .map(|file| file.rel.as_str())
        .collect::<BTreeSet<_>>();
    let mut certificate = file_certificate(
        input,
        visited,
        "proof",
        |file| source_candidate(file) || incomplete_boundaries.contains(file.rel.as_str()),
        relation_capability,
    );
    let route_horizon = input
        .route_observations
        .horizons
        .iter()
        .find(|horizon| horizon.group == "routes")
        .expect("route observation owner must precede proof coverage");
    let route_certificate = input
        .route_observations
        .certificates
        .get(&route_horizon.count.certificate_id)
        .expect("route horizon must resolve its certificate");
    certificate
        .dynamic_stops
        .extend(route_certificate.dynamic_stops.clone());
    certificate
        .unsupported
        .extend(route_certificate.unsupported.clone());
    certificate
        .unresolved_stops
        .extend(route_certificate.unresolved_stops.clone());
    certificate
        .external_stops
        .extend(route_certificate.external_stops.clone());
    if route_horizon.count.observed > 0 || route_horizon.count.closure != CoverageClosure::Closed {
        certificate.closure = CoverageClosure::Open;
        certificate.unresolved_stops.push(CoverageStop {
            kind: CoverageReason::VerificationRelationFlow,
            location: Some(CoverageLocation::path(input.scope)),
            missing_surface: Some(
                "runtime proof joins cover static route visits with unique structural owners only"
                    .to_string(),
            ),
        });
    }
    certificate
}

fn unknown_certificate(
    input: &RuntimeGroupObservationInput<'_>,
    visited: &BTreeSet<&str>,
) -> CoverageCertificate {
    let mut certificate = file_certificate(
        input,
        visited,
        "unknowns",
        source_candidate,
        unknown_capability,
    );
    if !input.scope_logically_empty {
        certificate.closure = CoverageClosure::Open;
        certificate.unresolved_stops.push(CoverageStop {
            kind: CoverageReason::UnclassifiedCoverageGap,
            location: Some(CoverageLocation::path(input.scope)),
            missing_surface: Some(
                "runtime unknown detectors cover declared constructs, not arbitrary syntax"
                    .to_string(),
            ),
        });
    }
    certificate
}

fn unknown_location(unknown: &Unknown) -> Option<CoverageLocation> {
    unknown.path.as_ref().map(|path| CoverageLocation {
        path: path.clone(),
        line_start: unknown.line_start,
        line_end: unknown.line_start,
    })
}

fn query_kind(group: &str) -> &'static str {
    crate::model::RuntimeReport::observation_query_kind(group)
        .expect("runtime group contract rejects unknown groups")
}
