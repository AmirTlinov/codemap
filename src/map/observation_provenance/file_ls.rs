// Responsibility: exact-file-ls-observation-provenance
use crate::map::{
    ConsumerObservationInput, ObservationProjection, consumer_observed_count,
    unavailable_observation, verification_observation,
};
use crate::model::{
    CoverageCertificate, CoverageClosure, CoverageLocation, CoverageReason, CoverageStop,
    ExtractorCapability, FileInfo, ObservationLedger, Project, UnsupportedObservation,
};

pub(crate) struct FileLsObservationInput<'a> {
    pub info: &'a FileInfo,
    pub imports_observed: usize,
    pub imports_shown: usize,
    pub imports_expand: Option<String>,
    pub consumers_observed: usize,
    pub consumers_shown: usize,
    pub consumers_expand: Option<String>,
    pub verification_observed: usize,
    pub verification_shown: usize,
    pub verification_expand: Option<String>,
    pub symbols_observed: usize,
    pub symbols_shown: usize,
    pub symbols_expand: Option<String>,
}

pub(crate) fn file_ls_observations(
    project: &Project,
    input: FileLsObservationInput<'_>,
) -> ObservationLedger {
    let mut ledger = ObservationLedger::default();
    if input.info.content_hash.is_none() {
        for group in ["imports", "consumers", "verification", "symbols"] {
            unavailable_observation(
                project,
                ObservationProjection {
                    group,
                    scope: &input.info.rel,
                    observed: 0,
                    shown: 0,
                    expand: None,
                },
                CoverageReason::UnsupportedConstruct,
                &mut ledger,
            );
        }
        return ledger;
    }

    record_import_observation(project, &input, &mut ledger);
    record_file_symbol_observation(
        project,
        input.info,
        ObservationProjection {
            group: "symbols",
            scope: &input.info.rel,
            observed: input.symbols_observed,
            shown: input.symbols_shown,
            expand: input.symbols_expand.clone(),
        },
        &mut ledger,
    );
    consumer_observed_count(
        project,
        ConsumerObservationInput {
            rel: &input.info.rel,
            symbol: None,
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
            scope: &input.info.rel,
            observed: input.verification_observed,
            shown: input.verification_shown,
            expand: input.verification_expand,
        },
        &mut ledger,
    );
    ledger
}

pub(super) fn record_file_symbol_observation(
    project: &Project,
    info: &FileInfo,
    projection: ObservationProjection<'_>,
    ledger: &mut ObservationLedger,
) {
    if info.content_hash.is_none() {
        unavailable_observation(
            project,
            projection,
            CoverageReason::UnsupportedConstruct,
            ledger,
        );
        return;
    }
    let supported = matches!(
        info.ext.as_str(),
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
    );
    let closure = if supported {
        CoverageClosure::Closed
    } else {
        CoverageClosure::Unavailable
    };
    let reasons = (!supported)
        .then_some(CoverageReason::UnsupportedLanguage)
        .into_iter()
        .collect();
    let query_kind = if projection.group == "xray_outputs" {
        "file_xray_output_surfaces"
    } else {
        "file_symbol_catalog"
    };
    let mut certificate = CoverageCertificate::new(
        query_kind,
        &info.rel,
        crate::cache::fingerprint(project, None),
        1,
        u64::from(supported),
        closure,
        reasons,
    );
    if supported {
        certificate
            .extractor_capabilities
            .push(ExtractorCapability {
                extractor_id: "codemap.indexed-symbol-table".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                language: info.language.clone(),
                constructs: vec![query_kind.to_string()],
            });
    } else {
        certificate
            .excluded_files_by_reason
            .insert(CoverageReason::UnsupportedLanguage, vec![info.rel.clone()]);
        certificate.unsupported.push(UnsupportedObservation {
            file: info.rel.clone(),
            construct: format!(".{} symbol extraction", info.ext),
            location: Some(CoverageLocation::path(&info.rel)),
        });
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

fn record_import_observation(
    project: &Project,
    input: &FileLsObservationInput<'_>,
    ledger: &mut ObservationLedger,
) {
    let info = input.info;
    let supported = matches!(
        info.ext.as_str(),
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "py" | "rs" | "go" | "swift"
    );
    let mut reasons = Vec::new();
    let mut dynamic_stops = Vec::new();
    let mut unresolved_stops = Vec::new();
    let mut unsupported = Vec::new();
    let closure = if !supported {
        reasons.push(CoverageReason::UnsupportedLanguage);
        unsupported.push(UnsupportedObservation {
            file: info.rel.clone(),
            construct: format!(".{} static import extraction", info.ext),
            location: Some(CoverageLocation::path(&info.rel)),
        });
        CoverageClosure::Unavailable
    } else {
        if info.has_dynamic_import {
            reasons.push(CoverageReason::DynamicImportFlow);
            dynamic_stops.push(CoverageStop {
                kind: CoverageReason::DynamicImportFlow,
                location: Some(CoverageLocation::path(&info.rel)),
                missing_surface: Some("dynamic import target".to_string()),
            });
        }
        for spec in &info.unresolved_imports {
            reasons.push(CoverageReason::IncompleteTraversal);
            unresolved_stops.push(CoverageStop {
                kind: CoverageReason::IncompleteTraversal,
                location: Some(CoverageLocation::path(&info.rel)),
                missing_surface: Some(format!("unresolved local import `{spec}`")),
            });
        }
        for gap in crate::map::unresolved_import_unknowns(project, info)
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
        if reasons.is_empty() {
            CoverageClosure::Closed
        } else {
            CoverageClosure::Open
        }
    };
    let mut certificate = CoverageCertificate::new(
        "file_import_relations",
        &info.rel,
        crate::cache::fingerprint(project, None),
        1,
        u64::from(supported),
        closure,
        reasons,
    );
    if supported {
        certificate
            .extractor_capabilities
            .push(ExtractorCapability {
                extractor_id: "codemap.static-file-imports".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                language: info.language.clone(),
                constructs: vec!["resolved_static_import".to_string()],
            });
    } else {
        certificate
            .excluded_files_by_reason
            .insert(CoverageReason::UnsupportedLanguage, vec![info.rel.clone()]);
    }
    certificate.unsupported = unsupported;
    certificate.dynamic_stops = dynamic_stops;
    certificate.unresolved_stops = unresolved_stops;
    ledger.record(
        "imports",
        &info.rel,
        input.imports_observed as u64,
        input.imports_shown as u64,
        certificate,
        input.imports_expand.clone(),
    );
}
