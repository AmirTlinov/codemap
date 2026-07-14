// Responsibility: map-consumer-count-provenance
use crate::model::{CountFact, Project};

const SUPPORTED_IMPORT_LANGUAGES: &[&str] =
    &["javascript/typescript", "python", "rust", "go", "swift"];

/// Provenance-carrying consumer counter: counted(n) only for observed edges,
/// proven_zero only when the import flow around `rel` is fully supported,
/// otherwise a typed unknown naming the blind spot.
pub(crate) fn consumer_count_fact(
    project: &Project,
    rel: &str,
    symbol: Option<&str>,
    raw: usize,
) -> CountFact {
    if raw > 0 {
        return CountFact::counted(raw);
    }
    match consumer_zero_blind_spot(project, rel, symbol) {
        Some(reason) => CountFact::unknown(reason),
        None => CountFact::proven_zero(),
    }
}

fn consumer_zero_blind_spot(project: &Project, rel: &str, symbol: Option<&str>) -> Option<String> {
    let Some(info) = project.files.get(rel) else {
        return Some("anchor is not indexed".to_string());
    };
    if !SUPPORTED_IMPORT_LANGUAGES.contains(&info.language.as_str()) {
        return Some(format!("{} import flow is not indexed", info.language));
    }
    if let Some(importers) = project.reverse_imports.get(rel) {
        for importer in importers {
            let Some(importer_info) = project.files.get(importer) else {
                continue;
            };
            if let Some(bindings) = importer_info.resolved_import_bindings.get(rel) {
                let reexported = bindings.keys().any(|key| match symbol {
                    Some(name) => key == "export:*" || key == &format!("export:{name}"),
                    None => key.starts_with("export:"),
                });
                if reexported {
                    return Some(format!("re-export flow via `{importer}`"));
                }
            }
            if importer_info
                .imports
                .iter()
                .any(|spec| spec.ends_with(".rs") && rel.ends_with(spec.trim_start_matches("./")))
            {
                return Some(format!("rust include! flow via `{importer}`"));
            }
        }
    }
    if let Some(source) = dynamic_import_neighbor(project, rel) {
        return Some(format!("dynamic import flow via `{source}`"));
    }
    None
}

fn dynamic_import_neighbor(project: &Project, rel: &str) -> Option<String> {
    let scope = package_scope_for(project, rel);
    project
        .files
        .values()
        .find(|file| {
            file.has_dynamic_import
                && file.rel != rel
                && package_scope_for(project, &file.rel) == scope
        })
        .map(|file| file.rel.clone())
}

fn package_scope_for<'a>(project: &'a Project, rel: &str) -> &'a str {
    project
        .packages
        .iter()
        .filter(|package| {
            package.path == "."
                || rel.starts_with(&format!("{}/", package.path.trim_end_matches('/')))
        })
        .map(|package| package.path.as_str())
        .max_by_key(|path| path.len())
        .unwrap_or(".")
}
