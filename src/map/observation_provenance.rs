// Responsibility: map-report-observation-provenance
use crate::model::{
    CoverageCertificate, CoverageClosure, CoverageLocation, CoverageReason, ExtractorCapability,
    FileInfo, ObservationLedger, ObservedCount, Project, UnsupportedObservation,
};

use crate::map::{ConsumerObservationInput, consumer_observed_count};

mod definition_coverage;
use definition_coverage::definition_extractor_capability;
mod directory_ls;
pub(crate) use directory_ls::directory_relation_observation;
mod file_ls;
pub(crate) use file_ls::{FileLsObservationInput, file_ls_observations};
mod root_inventory;
pub(crate) use root_inventory::{
    RootInventoryGroupVisibility, RootInventoryObservationInput,
    record_root_inventory_observations, root_script_manifest_partition,
};
mod runtime;
pub(crate) use runtime::{
    RuntimeGroupObservationInput, RuntimeGroupProjection, RuntimeRouteObservationInput,
    runtime_group_observations, runtime_route_observations,
};

pub(crate) struct ObservationProjection<'a> {
    pub group: &'a str,
    pub scope: &'a str,
    pub observed: usize,
    pub shown: usize,
    pub expand: Option<String>,
}

pub(crate) struct SymbolConeObservationInput<'a> {
    pub file_rel: &'a str,
    pub symbol_name: &'a str,
    pub incoming_observed: usize,
    pub incoming_shown: usize,
    pub incoming_expand: Option<String>,
    pub verification_observed: usize,
    pub verification_shown: usize,
    pub verification_expand: Option<String>,
}

pub(crate) struct SymbolLsObservationInput<'a> {
    pub file_rel: &'a str,
    pub symbol_name: &'a str,
    pub consumers_observed: usize,
    pub consumers_shown: usize,
    pub consumers_expand: Option<String>,
    pub verification_observed: usize,
    pub verification_shown: usize,
    pub verification_expand: Option<String>,
}

pub(crate) fn symbol_ls_observations(
    project: &Project,
    input: SymbolLsObservationInput<'_>,
) -> ObservationLedger {
    let scope = format!("{}#{}", input.file_rel, input.symbol_name);
    let mut ledger = ObservationLedger::default();
    consumer_observed_count(
        project,
        ConsumerObservationInput {
            rel: input.file_rel,
            symbol: Some(input.symbol_name),
            raw: input.consumers_observed,
            shown: input.consumers_shown,
            group: "consumers",
            expand: input.consumers_expand,
            include_local: false,
        },
        &mut ledger,
    );
    verification_observation(
        project,
        ObservationProjection {
            group: "verification",
            scope: &scope,
            observed: input.verification_observed,
            shown: input.verification_shown,
            expand: input.verification_expand,
        },
        &mut ledger,
    );
    ledger
}

pub(crate) fn symbol_cone_observations(
    project: &Project,
    input: SymbolConeObservationInput<'_>,
) -> ObservationLedger {
    let scope = format!("{}#{}", input.file_rel, input.symbol_name);
    let mut ledger = ObservationLedger::default();
    consumer_observed_count(
        project,
        ConsumerObservationInput {
            rel: input.file_rel,
            symbol: Some(input.symbol_name),
            raw: input.incoming_observed,
            shown: input.incoming_shown,
            group: "incoming",
            expand: input.incoming_expand,
            include_local: true,
        },
        &mut ledger,
    );
    verification_observation(
        project,
        ObservationProjection {
            group: "verification",
            scope: &scope,
            observed: input.verification_observed,
            shown: input.verification_shown,
            expand: input.verification_expand,
        },
        &mut ledger,
    );
    ledger
}

pub(crate) fn missing_symbol_observations(project: &Project, scope: &str) -> ObservationLedger {
    missing_symbol_group_observations(project, scope, &["incoming", "verification"])
}

pub(crate) fn missing_symbol_ls_observations(project: &Project, scope: &str) -> ObservationLedger {
    missing_symbol_group_observations(project, scope, &["consumers", "verification"])
}

fn missing_symbol_group_observations(
    project: &Project,
    scope: &str,
    groups: &[&str],
) -> ObservationLedger {
    let mut ledger = ObservationLedger::default();
    for group in groups {
        unavailable_observation(
            project,
            ObservationProjection {
                group,
                scope,
                observed: 0,
                shown: 0,
                expand: None,
            },
            CoverageReason::AnchorNotIndexed,
            &mut ledger,
        );
    }
    ledger
}

pub(crate) fn definition_match_observation(
    project: &Project,
    query: &str,
    projection: ObservationProjection<'_>,
    ledger: &mut ObservationLedger,
) -> ObservedCount {
    let candidates = project
        .files
        .values()
        .filter(|file| definition_candidate(file))
        .collect::<Vec<_>>();
    let mut visited_files = 0_u64;
    let mut reasons = Vec::new();
    let mut capabilities = Vec::new();
    let mut unsupported = Vec::new();
    let mut excluded_files_by_reason = std::collections::BTreeMap::new();

    for file in &candidates {
        let (reason, construct) = match definition_extractor_capability(project, file, query) {
            Ok(capability) => {
                visited_files += 1;
                capabilities.push(capability);
                continue;
            }
            Err(gap) => gap,
        };
        reasons.push(reason);
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

    let closure = if unsupported.is_empty() {
        CoverageClosure::Closed
    } else {
        CoverageClosure::Open
    };
    let mut certificate = CoverageCertificate::new(
        "indexed_definition_matches",
        projection.scope,
        crate::cache::fingerprint(project, None),
        candidates.len() as u64,
        visited_files,
        closure,
        reasons,
    );
    certificate.extractor_capabilities = capabilities;
    certificate.unsupported = unsupported;
    certificate.excluded_files_by_reason = excluded_files_by_reason;
    ledger.record(
        projection.group,
        projection.scope,
        projection.observed as u64,
        projection.shown as u64,
        certificate,
        projection.expand,
    )
}

fn definition_candidate(file: &FileInfo) -> bool {
    crate::repo::is_source_ext(&file.ext) || crate::repo::is_script_ext(&file.ext)
}

pub(crate) fn verification_observation(
    project: &Project,
    projection: ObservationProjection<'_>,
    ledger: &mut ObservationLedger,
) -> ObservedCount {
    let candidates = project
        .files
        .values()
        .filter(|file| file.has_role("test") || file.has_role("test_support"))
        .collect::<Vec<_>>();
    let mut visited_files = 0_u64;
    let mut capabilities = Vec::new();
    let mut unsupported = Vec::new();
    let mut excluded_files_by_reason = std::collections::BTreeMap::new();
    for file in &candidates {
        if let Some(capability) = verification_extractor_capability(file) {
            visited_files += 1;
            capabilities.push(capability);
            continue;
        }
        let (reason, construct) = unsupported_verification_candidate(file);
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
    let mut certificate = CoverageCertificate::new(
        "structural_verification_relations",
        projection.scope,
        crate::cache::fingerprint(project, None),
        candidates.len() as u64,
        visited_files,
        CoverageClosure::Open,
        vec![CoverageReason::VerificationRelationFlow],
    );
    certificate.extractor_capabilities = capabilities;
    certificate.unsupported = unsupported;
    certificate.excluded_files_by_reason = excluded_files_by_reason;
    ledger.record(
        projection.group,
        projection.scope,
        projection.observed as u64,
        projection.shown as u64,
        certificate,
        projection.expand,
    )
}

fn verification_extractor_capability(file: &FileInfo) -> Option<ExtractorCapability> {
    if file.content_hash.is_none()
        || !matches!(
            file.ext.as_str(),
            "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "py" | "rs" | "go" | "swift"
        )
    {
        return None;
    }
    Some(ExtractorCapability {
        extractor_id: "codemap.structural-verification-links".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        language: file.language.clone(),
        constructs: vec![
            "resolved_test_import".to_string(),
            "indexed_test_surface".to_string(),
        ],
    })
}

fn unsupported_verification_candidate(file: &FileInfo) -> (CoverageReason, String) {
    if file.content_hash.is_none() {
        return (
            CoverageReason::UnsupportedConstruct,
            "verification source could not be read".to_string(),
        );
    }
    if matches!(file.ext.as_str(), "vue" | "svelte") {
        return (
            CoverageReason::UnsupportedConstruct,
            format!(".{} component-container verification extraction", file.ext),
        );
    }
    (
        CoverageReason::UnsupportedLanguage,
        format!(".{} verification relation extraction", file.ext),
    )
}

pub(crate) fn unavailable_observation(
    project: &Project,
    projection: ObservationProjection<'_>,
    reason: CoverageReason,
    ledger: &mut ObservationLedger,
) -> ObservedCount {
    let certificate = CoverageCertificate::new(
        projection.group,
        projection.scope,
        crate::cache::fingerprint(project, None),
        0,
        0,
        CoverageClosure::Unavailable,
        vec![reason],
    );
    ledger.record(
        projection.group,
        projection.scope,
        projection.observed as u64,
        projection.shown as u64,
        certificate,
        projection.expand,
    )
}
