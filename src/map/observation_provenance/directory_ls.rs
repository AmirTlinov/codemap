// Responsibility: nested-directory-ls-observation-provenance
use crate::map::{ObservationProjection, path_under_scope, unresolved_import_unknowns};
use crate::model::{
    CoverageCertificate, CoverageClosure, CoverageLocation, CoverageReason, CoverageStop,
    ExtractorCapability, ObservationLedger, Project, UnsupportedObservation,
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn directory_relation_observation(
    project: &Project,
    projection: ObservationProjection<'_>,
) -> ObservationLedger {
    directory_relation_observation_for_query(project, projection, "nested_directory_relations")
}

pub(crate) fn directory_relation_observation_for_query(
    project: &Project,
    projection: ObservationProjection<'_>,
    query_kind: &str,
) -> ObservationLedger {
    let package_audit = crate::repo::audit_package_discovery(&project.root, &project.files);
    let package_paths = package_audit
        .candidates
        .iter()
        .map(|candidate| candidate.manifest.as_str())
        .collect::<BTreeSet<_>>();
    let package_gaps = package_audit
        .unsupported
        .iter()
        .map(|gap| (gap.manifest.as_str(), gap.construct))
        .collect::<BTreeMap<_, _>>();
    let script_paths = project
        .scripts
        .iter()
        .filter_map(|script| script.path.as_deref())
        .collect::<BTreeSet<_>>();
    let candidates = project
        .files
        .values()
        .filter(|file| {
            relation_source_candidate(&file.ext)
                || package_paths.contains(file.rel.as_str())
                || (path_under_scope(&file.rel, projection.scope)
                    && relation_owner_candidate(file, &script_paths))
        })
        .collect::<Vec<_>>();

    let mut visited = 0_u64;
    let mut reasons = Vec::new();
    let mut unsupported = Vec::new();
    let mut excluded = BTreeMap::<CoverageReason, Vec<String>>::new();
    let mut dynamic_stops = Vec::new();
    let mut unresolved_stops = Vec::new();
    let mut capabilities = BTreeMap::<(String, String), ExtractorCapability>::new();

    for file in &candidates {
        let is_package = package_paths.contains(file.rel.as_str());
        let is_source = relation_source_candidate(&file.ext);
        let is_owner = path_under_scope(&file.rel, projection.scope)
            && relation_owner_candidate(file, &script_paths);
        if is_package {
            if let Some(construct) = package_gaps.get(file.rel.as_str()) {
                exclude_candidate(
                    file,
                    CoverageReason::UnsupportedConstruct,
                    construct,
                    &mut reasons,
                    &mut unsupported,
                    &mut excluded,
                );
                continue;
            }
            record_capability(
                &mut capabilities,
                "codemap.package-relations",
                &file.language,
                &["package_dependency", "package_owner"],
            );
        }
        if is_source {
            if file.content_hash.is_none() {
                exclude_candidate(
                    file,
                    CoverageReason::UnsupportedConstruct,
                    "indexed source body is unavailable",
                    &mut reasons,
                    &mut unsupported,
                    &mut excluded,
                );
                continue;
            }
            if !supported_relation_source(&file.ext) {
                exclude_candidate(
                    file,
                    CoverageReason::UnsupportedLanguage,
                    "static directory relation extraction",
                    &mut reasons,
                    &mut unsupported,
                    &mut excluded,
                );
                continue;
            }
            record_capability(
                &mut capabilities,
                "codemap.directory-static-relations",
                &file.language,
                &["resolved_static_import", "reverse_static_import"],
            );
            record_source_stops(
                project,
                file,
                &mut reasons,
                &mut dynamic_stops,
                &mut unresolved_stops,
            );
        }
        if is_owner && file.content_hash.is_none() && !path_owned_relation(file) {
            exclude_candidate(
                file,
                CoverageReason::UnsupportedConstruct,
                "owner surface body is unavailable",
                &mut reasons,
                &mut unsupported,
                &mut excluded,
            );
            continue;
        }
        if is_owner {
            record_capability(
                &mut capabilities,
                "codemap.directory-owner-relations",
                &file.language,
                &["script", "ci", "env", "schema", "lockfile"],
            );
        }
        visited += 1;
    }

    let closure = CoverageClosure::from_gaps(!reasons.is_empty());
    let mut certificate = CoverageCertificate::new(
        query_kind,
        projection.scope,
        crate::cache::fingerprint(project, None),
        candidates.len() as u64,
        visited,
        closure,
        reasons,
    );
    certificate.extractor_capabilities = capabilities.into_values().collect();
    certificate.unsupported = unsupported;
    certificate.excluded_files_by_reason = excluded;
    certificate.dynamic_stops = dynamic_stops;
    certificate.unresolved_stops = unresolved_stops;
    let mut ledger = ObservationLedger::default();
    ledger.record(
        projection.group,
        projection.scope,
        projection.observed as u64,
        projection.shown as u64,
        certificate,
        projection.expand,
    );
    ledger
}

fn relation_source_candidate(ext: &str) -> bool {
    crate::repo::is_source_ext(ext) || matches!(ext, "css" | "scss" | "sass" | "less")
}

fn supported_relation_source(ext: &str) -> bool {
    matches!(
        ext,
        "ts" | "tsx"
            | "js"
            | "jsx"
            | "mjs"
            | "cjs"
            | "py"
            | "rs"
            | "go"
            | "swift"
            | "vue"
            | "svelte"
            | "css"
            | "scss"
            | "sass"
            | "less"
    )
}

fn relation_owner_candidate(file: &crate::model::FileInfo, script_paths: &BTreeSet<&str>) -> bool {
    script_paths.contains(file.rel.as_str())
        || [
            "build_ci",
            "env_config",
            "schema_contract",
            "migration",
            "lockfile",
        ]
        .iter()
        .any(|role| file.has_role(role))
}

fn path_owned_relation(file: &crate::model::FileInfo) -> bool {
    file.has_role("migration") || file.has_role("lockfile")
}

fn exclude_candidate(
    file: &crate::model::FileInfo,
    reason: CoverageReason,
    construct: &str,
    reasons: &mut Vec<CoverageReason>,
    unsupported: &mut Vec<UnsupportedObservation>,
    excluded: &mut BTreeMap<CoverageReason, Vec<String>>,
) {
    reasons.push(reason);
    excluded.entry(reason).or_default().push(file.rel.clone());
    unsupported.push(UnsupportedObservation {
        file: file.rel.clone(),
        construct: construct.to_string(),
        location: Some(CoverageLocation::path(&file.rel)),
    });
}

fn record_capability(
    capabilities: &mut BTreeMap<(String, String), ExtractorCapability>,
    extractor_id: &str,
    language: &str,
    constructs: &[&str],
) {
    capabilities
        .entry((extractor_id.to_string(), language.to_string()))
        .or_insert_with(|| ExtractorCapability {
            extractor_id: extractor_id.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            language: language.to_string(),
            constructs: constructs
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
        });
}

fn record_source_stops(
    project: &Project,
    file: &crate::model::FileInfo,
    reasons: &mut Vec<CoverageReason>,
    dynamic_stops: &mut Vec<CoverageStop>,
    unresolved_stops: &mut Vec<CoverageStop>,
) {
    if file.has_dynamic_import {
        reasons.push(CoverageReason::DynamicImportFlow);
        dynamic_stops.push(CoverageStop {
            kind: CoverageReason::DynamicImportFlow,
            location: Some(CoverageLocation::path(&file.rel)),
            missing_surface: Some("dynamic import may cross the directory boundary".to_string()),
        });
    }
    for spec in &file.unresolved_imports {
        reasons.push(CoverageReason::IncompleteTraversal);
        unresolved_stops.push(CoverageStop {
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
            unresolved_stops.push(CoverageStop {
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
}
