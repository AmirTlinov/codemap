// Responsibility: directory-cone-relationship-observation-provenance
use crate::map::{
    ObservationProjection, directory_relation_observation_for_query, path_under_scope,
};
use crate::model::{
    CoverageCertificate, CoverageClosure, CoverageLocation, CoverageReason, CoverageStop,
    ExtractorCapability, ObservationLedger, Project, UnsupportedObservation,
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct DirectoryConeObservationInput<'a> {
    pub depth: usize,
    pub outgoing: ObservationProjection<'a>,
    pub incoming: ObservationProjection<'a>,
    pub verification: ObservationProjection<'a>,
    pub contracts: ObservationProjection<'a>,
    pub boundary: ObservationProjection<'a>,
}

pub(crate) fn directory_cone_observations(
    project: &Project,
    input: DirectoryConeObservationInput<'_>,
) -> ObservationLedger {
    let outgoing_query = format!("directory_cone_outgoing_depth_{}", input.depth);
    let incoming_query = format!("directory_cone_incoming_depth_{}", input.depth);
    let mut ledger =
        directory_relation_observation_for_query(project, input.outgoing, &outgoing_query);
    ledger.merge(&directory_relation_observation_for_query(
        project,
        input.incoming,
        &incoming_query,
    ));
    for projection in [input.verification, input.contracts, input.boundary] {
        let query_kind = format!("directory_cone_{}_depth_{}", projection.group, input.depth);
        let certificate =
            cone_relationship_certificate(project, projection.scope, projection.group, &query_kind);
        ledger.record(
            projection.group,
            projection.scope,
            projection.observed as u64,
            projection.shown as u64,
            certificate,
            projection.expand,
        );
    }
    ledger
}

fn cone_relationship_certificate(
    project: &Project,
    scope: &str,
    group: &str,
    query_kind: &str,
) -> CoverageCertificate {
    let package_audit = crate::repo::audit_package_discovery(&project.root, &project.files);
    let package_gaps = package_audit
        .unsupported
        .iter()
        .map(|gap| (gap.manifest.as_str(), gap.construct))
        .collect::<BTreeMap<_, _>>();
    let candidates = project
        .files
        .values()
        .filter(|file| !file.has_role("generated") && group_candidate(file, scope, group))
        .collect::<Vec<_>>();
    let mut visited = 0_u64;
    let mut excluded = BTreeMap::<CoverageReason, Vec<String>>::new();
    let mut unsupported = Vec::new();
    let mut dynamic_stops = Vec::new();
    let mut unresolved_stops = Vec::new();
    let mut capabilities = BTreeSet::new();
    for file in &candidates {
        let unavailable = file
            .content_hash
            .is_none()
            .then_some("indexed relationship candidate body is unavailable");
        let gap = package_gaps.get(file.rel.as_str()).copied().or(unavailable);
        if let Some(construct) = gap {
            excluded
                .entry(CoverageReason::UnsupportedConstruct)
                .or_default()
                .push(file.rel.clone());
            unsupported.push(UnsupportedObservation {
                file: file.rel.clone(),
                construct: construct.to_string(),
                location: Some(CoverageLocation::path(&file.rel)),
            });
            continue;
        }
        visited += 1;
        capabilities.insert(group_capability(file, group));
        if group != "boundary" {
            record_flow_stops(file, &mut dynamic_stops, &mut unresolved_stops);
        }
    }
    let has_gaps =
        !excluded.is_empty() || !dynamic_stops.is_empty() || !unresolved_stops.is_empty();
    let mut certificate = CoverageCertificate::new(
        query_kind,
        scope,
        crate::cache::fingerprint(project, None),
        candidates.len() as u64,
        visited,
        CoverageClosure::from_gaps(has_gaps),
        Vec::new(),
    );
    certificate.excluded_files_by_reason = excluded;
    certificate.unsupported = unsupported;
    certificate.dynamic_stops = dynamic_stops;
    certificate.unresolved_stops = unresolved_stops;
    certificate.extractor_capabilities = capabilities.into_iter().collect();
    certificate
}

fn group_candidate(file: &crate::model::FileInfo, scope: &str, group: &str) -> bool {
    let scoped = path_under_scope(&file.rel, scope);
    match group {
        "verification" => scoped || file.has_role("test") || file.has_role("test_support"),
        "contracts" => scoped || file.has_role("schema_contract") || is_package_manifest(&file.rel),
        "boundary" => scoped || file.has_role("schema_contract") || is_package_manifest(&file.rel),
        _ => false,
    }
}

fn is_package_manifest(rel: &str) -> bool {
    matches!(
        std::path::Path::new(rel)
            .file_name()
            .and_then(|name| name.to_str()),
        Some("package.json" | "Cargo.toml" | "go.mod" | "pyproject.toml" | "Package.swift")
    )
}

fn group_capability(file: &crate::model::FileInfo, group: &str) -> ExtractorCapability {
    let constructs = match group {
        "verification" => vec!["test_import", "e2e_route_visit"],
        "contracts" => vec!["resolved_contract_import"],
        _ => vec!["configured_boundary_finding"],
    };
    ExtractorCapability {
        extractor_id: format!("codemap.directory-cone-{group}"),
        version: env!("CARGO_PKG_VERSION").to_string(),
        language: file.language.clone(),
        constructs: constructs.into_iter().map(str::to_string).collect(),
    }
}

fn record_flow_stops(
    file: &crate::model::FileInfo,
    dynamic: &mut Vec<CoverageStop>,
    unresolved: &mut Vec<CoverageStop>,
) {
    if file.has_dynamic_import {
        dynamic.push(CoverageStop {
            kind: CoverageReason::DynamicImportFlow,
            location: Some(CoverageLocation::path(&file.rel)),
            missing_surface: Some("dynamic flow may create a directory cone relation".to_string()),
        });
    }
    for spec in &file.unresolved_imports {
        unresolved.push(CoverageStop {
            kind: CoverageReason::IncompleteTraversal,
            location: Some(CoverageLocation::path(&file.rel)),
            missing_surface: Some(format!("unresolved local import `{spec}`")),
        });
    }
}
