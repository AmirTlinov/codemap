// Responsibility: repo-resolution-imports
use crate::model::{FileInfo, ImportBindingsBySpec, PackageInfo};
use crate::repo::{
    TsPathAlias, accessible_name_surfaces_from_component_labelled_ids,
    file_exports_dialog_labelledby_contract, is_uppercase_symbol, resolve_import,
};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

struct ImportResolutionSeed {
    rel: String,
    ext: String,
    imports: Vec<String>,
    import_bindings: ImportBindingsBySpec,
}

pub(crate) fn resolve_imports(
    root: &Path,
    files: &mut BTreeMap<String, FileInfo>,
    packages: &[PackageInfo],
    ts_path_aliases: &[TsPathAlias],
) {
    let paths: BTreeSet<String> = files.keys().cloned().collect();
    let snapshot: Vec<ImportResolutionSeed> = files
        .values()
        .map(|f| ImportResolutionSeed {
            rel: f.rel.clone(),
            ext: f.ext.clone(),
            imports: f.imports.iter().cloned().collect(),
            import_bindings: f.import_bindings.clone(),
        })
        .collect();
    for seed in snapshot {
        let mut resolved = BTreeSet::new();
        let mut unresolved = BTreeSet::new();
        let mut resolved_bindings = BTreeMap::new();
        for spec in seed.imports {
            if let Some(target) = resolve_import(
                root,
                &seed.rel,
                &seed.ext,
                &spec,
                &paths,
                packages,
                ts_path_aliases,
            ) {
                if let Some(bindings) = seed.import_bindings.get(&spec) {
                    resolved_bindings
                        .entry(target.clone())
                        .or_insert_with(BTreeMap::new)
                        .extend(
                            bindings
                                .iter()
                                .map(|(local, imported)| (local.clone(), imported.clone())),
                        );
                }
                resolved.insert(target);
            } else if unresolved_import_should_be_reported(&seed.ext, &spec) {
                unresolved.insert(spec);
            }
        }
        if let Some(info) = files.get_mut(&seed.rel) {
            info.resolved_imports = resolved;
            info.unresolved_imports = unresolved;
            info.resolved_import_bindings = resolved_bindings;
        }
    }
}

fn unresolved_import_should_be_reported(ext: &str, spec: &str) -> bool {
    if matches!(spec, "crate::" | "self::" | "super::") {
        return false;
    }
    if spec.starts_with('.') || spec.starts_with('/') {
        return true;
    }
    matches!(ext, "rs")
        && (spec.starts_with("crate::")
            || spec.starts_with("self::")
            || spec.starts_with("super::")
            || spec.ends_with(".rs"))
}

pub(crate) fn enrich_accessible_surfaces_from_component_contracts(
    root: &Path,
    files: &mut BTreeMap<String, FileInfo>,
) {
    let rels = files.keys().cloned().collect::<Vec<_>>();
    for rel in rels {
        let component_roles = {
            let Some(info) = files.get(&rel) else {
                continue;
            };
            if !matches!(
                info.ext.as_str(),
                "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte"
            ) {
                continue;
            }
            imported_accessible_component_roles(root, files, info)
        };
        if component_roles.is_empty() {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(&rel)) else {
            continue;
        };
        let extra = accessible_name_surfaces_from_component_labelled_ids(&text, &component_roles);
        if extra.tokens.is_empty() && extra.phrases.is_empty() && extra.visited_routes.is_empty() {
            continue;
        }
        if let Some(info) = files.get_mut(&rel) {
            info.surface_tokens.extend(extra.tokens);
            info.surface_phrases.extend(extra.phrases);
        }
    }
}

fn imported_accessible_component_roles(
    root: &Path,
    files: &BTreeMap<String, FileInfo>,
    info: &FileInfo,
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (target, bindings) in &info.resolved_import_bindings {
        for (local, imported) in bindings {
            if local.starts_with("export:") || !is_uppercase_symbol(local) {
                continue;
            }
            if file_declares_local_value(info, local) {
                continue;
            }
            let mut seen = BTreeSet::new();
            if component_export_resolves_to_dialog_labelledby_contract(
                root, files, target, imported, 0, &mut seen,
            ) {
                out.insert(local.clone(), "dialog".to_string());
            }
        }
    }
    out
}

fn file_declares_local_value(info: &FileInfo, name: &str) -> bool {
    info.local_bindings.contains(name) || info.symbols.iter().any(|symbol| symbol.name == name)
}

fn component_export_resolves_to_dialog_labelledby_contract(
    root: &Path,
    files: &BTreeMap<String, FileInfo>,
    file_rel: &str,
    export_name: &str,
    depth: usize,
    seen: &mut BTreeSet<(String, String)>,
) -> bool {
    if depth > 8 || !seen.insert((file_rel.to_string(), export_name.to_string())) {
        return false;
    }
    let Some(info) = files.get(file_rel) else {
        return false;
    };
    if file_exports_dialog_labelledby_contract(root, info, export_name) {
        return true;
    }
    let mut explicit = Vec::new();
    let mut stars = Vec::new();
    for (target, bindings) in &info.resolved_import_bindings {
        for (exported, imported) in bindings {
            let Some(exported_name) = exported.strip_prefix("export:") else {
                continue;
            };
            if exported_name == export_name {
                explicit.push((target.as_str(), imported.as_str()));
            } else if exported_name == "*" {
                stars.push(target.as_str());
            }
        }
    }
    if explicit.len() == 1 {
        let (target, imported) = explicit[0];
        return component_export_resolves_to_dialog_labelledby_contract(
            root,
            files,
            target,
            imported,
            depth + 1,
            seen,
        );
    }
    if explicit.len() > 1 {
        return false;
    }
    if stars.len() == 1 {
        return component_export_resolves_to_dialog_labelledby_contract(
            root,
            files,
            stars[0],
            export_name,
            depth + 1,
            seen,
        );
    }
    false
}
