// Responsibility: consumer-coverage-blind-spot-audit
use crate::model::{
    CoverageLocation, CoverageReason, CoverageStop, ExtractorCapability, FileInfo, Project,
    UnsupportedObservation,
};
use std::collections::BTreeSet;

mod query_audit;
use query_audit::collect_unobserved_query_gap;

const SUPPORTED_IMPORT_LANGUAGES: &[&str] =
    &["javascript/typescript", "python", "rust", "go", "swift"];
const JS_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "jsx", "mjs", "cjs", "vue", "svelte"];

pub(super) struct ConsumerBlindSpot {
    pub stop: CoverageStop,
    pub unsupported: Option<UnsupportedObservation>,
}

pub(super) fn supports_import_language(language: &str) -> bool {
    SUPPORTED_IMPORT_LANGUAGES.contains(&language)
}

pub(super) fn consumer_blind_spots(
    project: &Project,
    rel: &str,
    symbol: Option<&str>,
    include_local: bool,
) -> Vec<ConsumerBlindSpot> {
    let mut blind_spots = Vec::new();
    let observed_sources = symbol
        .map(|name| {
            crate::map::symbol_reference_edges(project, rel, name, false)
                .into_iter()
                .map(|edge| edge.from)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    collect_language_closure_gap(project, rel, &mut blind_spots);
    if include_local && let Some(anchor) = project.files.get(rel) {
        push_unsupported_binding_gap(
            anchor,
            "same_file_symbol_reference_closure",
            "same-file symbol body consumers",
            &mut blind_spots,
        );
    }
    if let Some(importers) = project.reverse_imports.get(rel) {
        for importer in importers {
            let Some(importer_info) = project.files.get(importer) else {
                continue;
            };
            if !is_consumer_universe_file(project, rel, importer_info) {
                continue;
            }
            collect_reexport_gap(rel, symbol, importer_info, &mut blind_spots);
            collect_rust_include_gap(rel, importer_info, &mut blind_spots);
            collect_js_static_binding_gaps(rel, symbol, importer_info, &mut blind_spots);
        }
    }
    for source in dynamic_import_neighbors(project, rel) {
        blind_spots.push(ConsumerBlindSpot {
            stop: CoverageStop {
                kind: CoverageReason::DynamicImportFlow,
                location: Some(CoverageLocation::path(source)),
                missing_surface: Some("dynamically selected symbol consumers".to_string()),
            },
            unsupported: None,
        });
    }
    for candidate in consumer_universe(project, rel) {
        collect_unreadable_candidate_gap(candidate, &mut blind_spots);
        collect_unresolved_import_gap(candidate, &mut blind_spots);
        collect_unobserved_query_gap(
            project,
            rel,
            candidate,
            symbol,
            &observed_sources,
            &mut blind_spots,
        );
    }
    for source in dynamic_require_neighbors(project, rel) {
        blind_spots.push(ConsumerBlindSpot {
            stop: CoverageStop {
                kind: CoverageReason::DynamicImportFlow,
                location: Some(CoverageLocation::path(source)),
                missing_surface: Some("dynamically required symbol consumers".to_string()),
            },
            unsupported: None,
        });
    }
    blind_spots.sort_by(|left, right| left.stop.cmp(&right.stop));
    blind_spots.dedup_by(|left, right| left.stop == right.stop);
    blind_spots
}

fn collect_unreadable_candidate_gap(candidate: &FileInfo, out: &mut Vec<ConsumerBlindSpot>) {
    if candidate.content_hash.is_some() {
        return;
    }
    push_unsupported_binding_gap(
        candidate,
        "consumer candidate source could not be read",
        "unreadable consumer candidate",
        out,
    );
}

fn collect_unresolved_import_gap(candidate: &FileInfo, out: &mut Vec<ConsumerBlindSpot>) {
    if candidate.unresolved_imports.is_empty() {
        return;
    }
    push_unsupported_binding_gap(
        candidate,
        "unresolved_static_import_target",
        "symbol consumers behind an unresolved static import target",
        out,
    );
}

fn collect_language_closure_gap(project: &Project, rel: &str, out: &mut Vec<ConsumerBlindSpot>) {
    let Some(anchor) = project.files.get(rel) else {
        return;
    };
    let javascript_module = anchor.language == "javascript/typescript"
        && !matches!(anchor.ext.as_str(), "vue" | "svelte");
    if javascript_module {
        return;
    }
    push_unsupported_binding_gap(
        anchor,
        &format!(
            "partial_{}_symbol_consumer_closure",
            anchor.language.replace('/', "_")
        ),
        "symbol-consumer closure for this language/container",
        out,
    );
}

fn collect_reexport_gap(
    rel: &str,
    symbol: Option<&str>,
    importer: &FileInfo,
    out: &mut Vec<ConsumerBlindSpot>,
) {
    let Some(bindings) = importer.resolved_import_bindings.get(rel) else {
        return;
    };
    let reexported = bindings.keys().any(|key| match symbol {
        Some(name) => key == "export:*" || key == &format!("export:{name}"),
        None => key.starts_with("export:"),
    });
    if reexported {
        out.push(ConsumerBlindSpot {
            stop: CoverageStop {
                kind: CoverageReason::ReexportFlow,
                location: Some(CoverageLocation::path(&importer.rel)),
                missing_surface: Some("mediated symbol consumers".to_string()),
            },
            unsupported: None,
        });
    }
}

fn collect_rust_include_gap(rel: &str, importer: &FileInfo, out: &mut Vec<ConsumerBlindSpot>) {
    let includes_anchor = importer
        .imports
        .iter()
        .any(|spec| spec.ends_with(".rs") && rel.ends_with(spec.trim_start_matches("./")));
    if includes_anchor {
        out.push(ConsumerBlindSpot {
            stop: CoverageStop {
                kind: CoverageReason::RustIncludeFlow,
                location: Some(CoverageLocation::path(&importer.rel)),
                missing_surface: Some("include expansion consumers".to_string()),
            },
            unsupported: None,
        });
    }
}

fn collect_js_static_binding_gaps(
    rel: &str,
    symbol: Option<&str>,
    importer: &FileInfo,
    out: &mut Vec<ConsumerBlindSpot>,
) {
    if !JS_EXTENSIONS.contains(&importer.ext.as_str()) || !importer.resolved_imports.contains(rel) {
        return;
    }
    let bindings = importer.resolved_import_bindings.get(rel);
    if bindings.is_none_or(|bindings| bindings.is_empty()) {
        push_unsupported_binding_gap(
            importer,
            "commonjs_or_unbound_static_import",
            "CommonJS or side-effect import symbol binding",
            out,
        );
        return;
    }
    let bindings = bindings.expect("non-empty bindings checked above");
    if bindings.values().any(|imported| imported == "*") {
        push_unsupported_binding_gap(
            importer,
            "namespace_import_member_binding",
            "namespace member consumers",
            out,
        );
    }
    let queried_binding_is_unscoped = symbol.is_some_and(|name| {
        bindings.values().any(|imported| imported == name) && importer.symbols.is_empty()
    });
    if queried_binding_is_unscoped {
        push_unsupported_binding_gap(
            importer,
            "unscoped_static_import_reference",
            "top-level imported-symbol consumers outside an indexed symbol body",
            out,
        );
    }
}

fn push_unsupported_binding_gap(
    importer: &FileInfo,
    construct: &str,
    missing_surface: &str,
    out: &mut Vec<ConsumerBlindSpot>,
) {
    let location = CoverageLocation::path(&importer.rel);
    out.push(ConsumerBlindSpot {
        stop: CoverageStop {
            kind: CoverageReason::UnsupportedConstruct,
            location: Some(location.clone()),
            missing_surface: Some(missing_surface.to_string()),
        },
        unsupported: Some(UnsupportedObservation {
            file: importer.rel.clone(),
            construct: construct.to_string(),
            location: Some(location),
        }),
    });
}

fn dynamic_import_neighbors(project: &Project, rel: &str) -> Vec<String> {
    consumer_universe(project, rel)
        .into_iter()
        .filter(|file| file.has_dynamic_import)
        .map(|file| file.rel.clone())
        .collect()
}

fn dynamic_require_neighbors(project: &Project, rel: &str) -> Vec<String> {
    consumer_universe(project, rel)
        .into_iter()
        .filter(|file| {
            if file.content_hash.is_none() {
                return false;
            }
            std::fs::read_to_string(project.root.join(&file.rel))
                .ok()
                .is_some_and(|text| text.lines().any(crate::map::dynamic_require_line))
        })
        .map(|file| file.rel.clone())
        .collect()
}

pub(super) fn consumer_universe<'a>(project: &'a Project, rel: &str) -> Vec<&'a FileInfo> {
    project
        .files
        .values()
        .filter(|file| is_consumer_universe_file(project, rel, file))
        .collect()
}

fn is_consumer_universe_file(project: &Project, rel: &str, file: &FileInfo) -> bool {
    let Some(anchor) = project.files.get(rel) else {
        return false;
    };
    consumer_candidate_matches(
        rel,
        &anchor.language,
        &file.rel,
        &file.language,
        file.has_role("test"),
        file.has_role("test_support"),
    )
}

fn consumer_candidate_matches(
    anchor_rel: &str,
    anchor_language: &str,
    candidate_rel: &str,
    candidate_language: &str,
    is_test: bool,
    is_test_support: bool,
) -> bool {
    candidate_rel != anchor_rel
        && candidate_language == anchor_language
        && !is_test
        && !is_test_support
}

pub(super) fn consumer_extractor_capabilities(
    project: &Project,
    rel: &str,
    include_local: bool,
) -> Vec<ExtractorCapability> {
    project
        .files
        .get(rel)
        .filter(|file| supports_import_language(&file.language))
        .map(|file| {
            let mut constructs = vec![
                "resolved_static_import".to_string(),
                "same_package_symbol_reference".to_string(),
            ];
            if include_local {
                constructs.push("same_file_symbol_body_reference".to_string());
            }
            vec![ExtractorCapability {
                extractor_id: "codemap.static-consumer-flow".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                language: file.language.clone(),
                constructs,
            }]
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::consumer_candidate_matches;

    #[test]
    fn consumer_universe_crosses_packages_but_keeps_language_and_test_boundaries() {
        let anchor = "packages/a/src/owner.ts";
        assert!(consumer_candidate_matches(
            anchor,
            "javascript/typescript",
            "packages/b/src/consumer.ts",
            "javascript/typescript",
            false,
            false,
        ));
        assert!(!consumer_candidate_matches(
            anchor,
            "javascript/typescript",
            "packages/b/src/consumer.py",
            "python",
            false,
            false,
        ));
        assert!(!consumer_candidate_matches(
            anchor,
            "javascript/typescript",
            "packages/b/tests/consumer.ts",
            "javascript/typescript",
            true,
            false,
        ));
        assert!(!consumer_candidate_matches(
            anchor,
            "javascript/typescript",
            anchor,
            "javascript/typescript",
            false,
            false,
        ));
    }
}
