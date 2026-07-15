// Responsibility: exact-file-cone-xray-observation-provenance
use crate::map::ObservationProjection;
use crate::model::{
    ConeReport, CoverageCertificate, ExtractorCapability, FileInfo, ObservationLedger, Project,
    XrayCard,
};

use super::file_ls::record_file_symbol_observation;

pub(super) struct FileConeXrayObservationInput<'a> {
    pub info: &'a FileInfo,
    pub depth: usize,
    pub observed: &'a XrayCard,
    pub shown: &'a XrayCard,
    pub expand: String,
}

pub(super) fn record_file_xray_observations(
    project: &Project,
    input: FileConeXrayObservationInput<'_>,
    ledger: &mut ObservationLedger,
) {
    let observed = input.observed.group_fact_counts();
    let shown = input.shown.group_fact_counts();
    for (((group, query_kind), (_, observed)), (_, shown)) in
        ConeReport::XRAY_GROUPS.iter().zip(observed).zip(shown)
    {
        let projection = ObservationProjection {
            group,
            scope: &input.info.rel,
            observed,
            shown,
            expand: (shown < observed).then(|| input.expand.clone()),
        };
        if *group == "xray_outputs" {
            record_file_symbol_observation(project, input.info, projection, ledger);
        } else {
            record_xray_group(&input, projection, query_kind, ledger);
        }
    }
}

fn record_xray_group(
    input: &FileConeXrayObservationInput<'_>,
    projection: ObservationProjection<'_>,
    query_kind: &str,
    ledger: &mut ObservationLedger,
) {
    let basis_group = basis_group(projection.group);
    let mut certificate = basis_certificate(ledger, basis_group);
    certificate.id.clear();
    certificate.query_kind = format!("{query_kind}_depth_{}", input.depth);
    certificate.observed_facts = 0;
    if certificate.visited_files > 0 {
        certificate
            .extractor_capabilities
            .push(ExtractorCapability {
                extractor_id: "codemap.cone-xray-card".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                language: input.info.language.clone(),
                constructs: vec![projection.group.to_string()],
            });
    }
    if projection.group == "xray_unknowns" {
        inherit_all_known_gaps(ledger, &mut certificate);
    }
    ledger.record(
        projection.group,
        projection.scope,
        projection.observed as u64,
        projection.shown as u64,
        certificate,
        projection.expand,
    );
}

fn basis_certificate(ledger: &ObservationLedger, group: &str) -> CoverageCertificate {
    let horizon = ledger
        .horizons
        .iter()
        .find(|horizon| horizon.group == group)
        .unwrap_or_else(|| panic!("missing `{group}` basis for exact-file X-Ray observation"));
    ledger
        .certificates
        .get(&horizon.count.certificate_id)
        .unwrap_or_else(|| panic!("dangling `{group}` exact-file X-Ray basis"))
        .clone()
}

fn inherit_all_known_gaps(ledger: &ObservationLedger, target: &mut CoverageCertificate) {
    for certificate in ledger.certificates.values() {
        target.reasons.extend(certificate.reasons.iter().copied());
        target
            .unsupported
            .extend(certificate.unsupported.iter().cloned());
        target
            .dynamic_stops
            .extend(certificate.dynamic_stops.iter().cloned());
        target
            .unresolved_stops
            .extend(certificate.unresolved_stops.iter().cloned());
        target
            .external_stops
            .extend(certificate.external_stops.iter().cloned());
    }
}

fn basis_group(group: &str) -> &'static str {
    match group {
        "xray_direct_consumers" | "xray_mediated_consumers" => "incoming",
        "xray_proof_hard"
        | "xray_proof_direct"
        | "xray_proof_mediated"
        | "xray_proof_soft"
        | "xray_unknowns" => "verification",
        "xray_nearby" => "boundary",
        _ => "outgoing",
    }
}
