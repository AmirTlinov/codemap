// Responsibility: exact-file-cone-relationship-observation-provenance
use crate::map::{
    ConsumerObservationInput, ObservationProjection, consumer_observed_count,
    unresolved_import_unknowns,
};
use crate::model::{
    CoverageCertificate, CoverageClosure, CoverageLocation, CoverageReason, CoverageStop,
    ExtractorCapability, FileInfo, ObservationLedger, Project, UnsupportedObservation,
};
use std::collections::{BTreeMap, BTreeSet};

use super::file_ls::record_file_symbol_observation;

pub(crate) struct FileConeObservationInput<'a> {
    pub info: &'a FileInfo,
    pub depth: usize,
    pub depths: &'a BTreeMap<String, usize>,
    pub outgoing: ObservationProjection<'a>,
    pub incoming: ObservationProjection<'a>,
    pub verification: ObservationProjection<'a>,
    pub contracts: ObservationProjection<'a>,
    pub boundary: ObservationProjection<'a>,
    pub symbols: ObservationProjection<'a>,
}

pub(crate) fn file_cone_observations(
    project: &Project,
    input: FileConeObservationInput<'_>,
) -> ObservationLedger {
    let mut ledger = ObservationLedger::default();
    record_group(
        project,
        &input,
        input.outgoing.clone(),
        GroupBasis::Depth,
        &mut ledger,
    );
    if supported_import_language(&input.info.language) && input.info.content_hash.is_some() {
        consumer_observed_count(
            project,
            ConsumerObservationInput {
                rel: &input.info.rel,
                symbol: None,
                raw: input.incoming.observed,
                shown: input.incoming.shown,
                group: "incoming",
                expand: input.incoming.expand.clone(),
                include_local: false,
            },
            &mut ledger,
        );
    } else {
        record_group(
            project,
            &input,
            input.incoming.clone(),
            GroupBasis::Anchor,
            &mut ledger,
        );
    }
    record_group(
        project,
        &input,
        input.verification.clone(),
        GroupBasis::Verification,
        &mut ledger,
    );
    record_group(
        project,
        &input,
        input.contracts.clone(),
        GroupBasis::Depth,
        &mut ledger,
    );
    record_group(
        project,
        &input,
        input.boundary.clone(),
        GroupBasis::Boundary,
        &mut ledger,
    );
    record_file_symbol_observation(project, input.info, input.symbols.clone(), &mut ledger);
    ledger
}

#[derive(Clone, Copy)]
enum GroupBasis {
    Anchor,
    Depth,
    Verification,
    Boundary,
}

fn record_group(
    project: &Project,
    input: &FileConeObservationInput<'_>,
    projection: ObservationProjection<'_>,
    basis: GroupBasis,
    ledger: &mut ObservationLedger,
) {
    let candidates = candidates(project, input, basis);
    let certificate = relationship_certificate(project, input, &projection, candidates, basis);
    ledger.record(
        projection.group,
        projection.scope,
        projection.observed as u64,
        projection.shown as u64,
        certificate,
        projection.expand,
    );
}

fn candidates<'a>(
    project: &'a Project,
    input: &FileConeObservationInput<'_>,
    basis: GroupBasis,
) -> Vec<&'a FileInfo> {
    let mut paths = BTreeSet::new();
    paths.insert(input.info.rel.clone());
    match basis {
        GroupBasis::Anchor => {}
        GroupBasis::Depth => {
            paths.extend(
                input
                    .depths
                    .iter()
                    .filter(|(_, level)| **level < input.depth)
                    .map(|(path, _)| path.clone()),
            );
        }
        GroupBasis::Boundary => {
            paths.extend(input.depths.keys().cloned());
        }
        GroupBasis::Verification => {
            paths.extend(
                project
                    .files
                    .values()
                    .filter(|file| {
                        file.has_role("test")
                            || file.has_role("test_support")
                            || (file.language == input.info.language && !file.has_role("generated"))
                    })
                    .map(|file| file.rel.clone()),
            );
        }
    }
    paths
        .into_iter()
        .filter_map(|path| project.files.get(&path))
        .collect()
}

fn relationship_certificate(
    project: &Project,
    input: &FileConeObservationInput<'_>,
    projection: &ObservationProjection<'_>,
    candidates: Vec<&FileInfo>,
    basis: GroupBasis,
) -> CoverageCertificate {
    let package_audit = crate::repo::audit_package_discovery(&project.root, &project.files);
    let package_gaps = package_audit
        .unsupported
        .iter()
        .map(|gap| (gap.manifest.as_str(), gap.construct))
        .collect::<BTreeMap<_, _>>();
    let mut visited = 0_u64;
    let mut reasons = Vec::new();
    let mut excluded = BTreeMap::<CoverageReason, Vec<String>>::new();
    let mut unsupported = Vec::new();
    let mut dynamic_stops = Vec::new();
    let mut unresolved_stops = Vec::new();
    let mut capabilities = BTreeMap::<(String, String), ExtractorCapability>::new();

    for file in &candidates {
        if let Some(construct) = package_gaps.get(file.rel.as_str()).copied() {
            exclude(
                file,
                construct,
                &mut reasons,
                &mut excluded,
                &mut unsupported,
            );
            continue;
        }
        if file.content_hash.is_none() {
            exclude(
                file,
                "indexed relationship candidate body is unavailable",
                &mut reasons,
                &mut excluded,
                &mut unsupported,
            );
            continue;
        }
        if source_candidate(file) && !supported_source(&file.ext) {
            reasons.push(CoverageReason::UnsupportedLanguage);
            excluded
                .entry(CoverageReason::UnsupportedLanguage)
                .or_default()
                .push(file.rel.clone());
            unsupported.push(UnsupportedObservation {
                file: file.rel.clone(),
                construct: format!(".{} exact-file cone relationship extraction", file.ext),
                location: Some(CoverageLocation::path(&file.rel)),
            });
            continue;
        }
        visited += 1;
        let extractor_id = format!("codemap.file-cone-{}", projection.group);
        capabilities
            .entry((extractor_id.clone(), file.language.clone()))
            .or_insert_with(|| ExtractorCapability {
                extractor_id,
                version: env!("CARGO_PKG_VERSION").to_string(),
                language: file.language.clone(),
                constructs: group_constructs(projection.group),
            });
        if !matches!(basis, GroupBasis::Boundary) {
            record_flow_stops(
                project,
                file,
                &mut reasons,
                &mut dynamic_stops,
                &mut unresolved_stops,
            );
        }
    }
    if matches!(basis, GroupBasis::Boundary) {
        for error in &project.config_errors {
            reasons.push(CoverageReason::UnsupportedConstruct);
            unsupported.push(UnsupportedObservation {
                file: error.path.clone(),
                construct: "semantic boundary config could not be parsed".to_string(),
                location: Some(CoverageLocation::path(&error.path)),
            });
            excluded
                .entry(CoverageReason::UnsupportedConstruct)
                .or_default()
                .push(error.path.clone());
        }
    }
    let has_gaps = !reasons.is_empty();
    let query = format!("file_cone_{}_depth_{}", projection.group, input.depth);
    let mut certificate = CoverageCertificate::new(
        query,
        projection.scope,
        crate::cache::fingerprint(project, None),
        candidates.len() as u64
            + project.config_errors.len() as u64 * u64::from(matches!(basis, GroupBasis::Boundary)),
        visited,
        CoverageClosure::from_gaps(has_gaps),
        reasons,
    );
    certificate.excluded_files_by_reason = excluded;
    certificate.unsupported = unsupported;
    certificate.dynamic_stops = dynamic_stops;
    certificate.unresolved_stops = unresolved_stops;
    certificate.extractor_capabilities = capabilities.into_values().collect();
    certificate
}

fn exclude(
    file: &FileInfo,
    construct: &str,
    reasons: &mut Vec<CoverageReason>,
    excluded: &mut BTreeMap<CoverageReason, Vec<String>>,
    unsupported: &mut Vec<UnsupportedObservation>,
) {
    reasons.push(CoverageReason::UnsupportedConstruct);
    excluded
        .entry(CoverageReason::UnsupportedConstruct)
        .or_default()
        .push(file.rel.clone());
    unsupported.push(UnsupportedObservation {
        file: file.rel.clone(),
        construct: construct.to_string(),
        location: Some(CoverageLocation::path(&file.rel)),
    });
}

fn record_flow_stops(
    project: &Project,
    file: &FileInfo,
    reasons: &mut Vec<CoverageReason>,
    dynamic: &mut Vec<CoverageStop>,
    unresolved: &mut Vec<CoverageStop>,
) {
    if file.has_dynamic_import {
        reasons.push(CoverageReason::DynamicImportFlow);
        dynamic.push(CoverageStop {
            kind: CoverageReason::DynamicImportFlow,
            location: Some(CoverageLocation::path(&file.rel)),
            missing_surface: Some(
                "dynamic flow may create an exact-file cone relation".to_string(),
            ),
        });
    }
    for spec in &file.unresolved_imports {
        reasons.push(CoverageReason::IncompleteTraversal);
        unresolved.push(CoverageStop {
            kind: CoverageReason::IncompleteTraversal,
            location: Some(CoverageLocation::path(&file.rel)),
            missing_surface: Some(format!("unresolved local import `{spec}`")),
        });
    }
    if file.ext == "rs" {
        for gap in unresolved_import_unknowns(project, file)
            .into_iter()
            .filter(|gap| gap.kind == "rust_include_unresolved")
        {
            reasons.push(CoverageReason::RustIncludeFlow);
            unresolved.push(CoverageStop {
                kind: CoverageReason::RustIncludeFlow,
                location: gap.path.map(|path| CoverageLocation {
                    path,
                    line_start: gap.line_start,
                    line_end: gap.line_start,
                }),
                missing_surface: Some("dynamic Rust include! target".to_string()),
            });
        }
    }
    if file
        .resolved_import_bindings
        .values()
        .any(|bindings| bindings.keys().any(|name| name.starts_with("export:")))
    {
        reasons.push(CoverageReason::ReexportFlow);
        unresolved.push(CoverageStop {
            kind: CoverageReason::ReexportFlow,
            location: Some(CoverageLocation::path(&file.rel)),
            missing_surface: Some("mediated re-export relationship".to_string()),
        });
    }
}

fn supported_import_language(language: &str) -> bool {
    matches!(
        language,
        "javascript/typescript" | "python" | "rust" | "go" | "swift"
    )
}

fn source_candidate(file: &FileInfo) -> bool {
    crate::repo::is_source_ext(&file.ext) || crate::repo::is_script_ext(&file.ext)
}

fn supported_source(ext: &str) -> bool {
    matches!(
        ext,
        "ts" | "tsx"
            | "js"
            | "jsx"
            | "mjs"
            | "cjs"
            | "vue"
            | "svelte"
            | "py"
            | "rs"
            | "go"
            | "swift"
    )
}

fn group_constructs(group: &str) -> Vec<String> {
    match group {
        "verification" => vec!["test_import", "indexed_test_surface"],
        "contracts" => vec!["resolved_contract_import"],
        "boundary" => vec!["configured_boundary_finding"],
        "incoming" => vec!["owner_incoming_relation"],
        _ => vec!["resolved_static_import", "owner_outgoing_relation"],
    }
    .into_iter()
    .map(str::to_string)
    .collect()
}
