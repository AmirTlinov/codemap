// Responsibility: map-symbols-imported-owner-resolution
use crate::map::{
    BarrelResolutionCache, barrel_reexports_symbol_from_file, imported_binding_target_symbol_name,
    local_named_export_bindings, matching_symbols,
};
use crate::model::Project;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImportedSymbolOwner {
    pub(crate) rel: String,
    pub(crate) symbol: String,
    pub(crate) reexported: bool,
}

pub(crate) fn imported_symbol_owner(
    project: &Project,
    target_rel: &str,
    imported: &str,
) -> Option<ImportedSymbolOwner> {
    if let Some(symbol) = imported_binding_target_symbol_name(project, target_rel, imported) {
        return Some(ImportedSymbolOwner {
            rel: target_rel.to_string(),
            symbol,
            reexported: false,
        });
    }
    let barrel = project.files.get(target_rel)?;
    let mut cache = BarrelResolutionCache::default();
    let mut owners = reexport_candidates(project, target_rel, imported, 0, &mut BTreeSet::new())
        .into_iter()
        .filter(|(rel, symbol)| {
            barrel_reexports_symbol_from_file(project, barrel, rel, symbol, imported, &mut cache)
        })
        .collect::<Vec<_>>();
    owners.sort();
    owners.dedup();
    (owners.len() == 1).then(|| {
        let (rel, symbol) = owners.remove(0);
        ImportedSymbolOwner {
            rel,
            symbol,
            reexported: true,
        }
    })
}

fn reexport_candidates(
    project: &Project,
    rel: &str,
    public_name: &str,
    depth: usize,
    seen: &mut BTreeSet<(String, String)>,
) -> BTreeSet<(String, String)> {
    if depth > 12 || !seen.insert((rel.to_string(), public_name.to_string())) {
        return BTreeSet::new();
    }
    let Some(file) = project.files.get(rel) else {
        return BTreeSet::new();
    };
    let mut candidates = matching_symbols(file, public_name)
        .into_iter()
        .filter(|symbol| symbol.kind != "module")
        .map(|symbol| (rel.to_string(), symbol.name.clone()))
        .collect::<BTreeSet<_>>();
    if let Some(locals) = local_named_export_bindings(project, rel).get(public_name) {
        candidates.extend(
            locals
                .iter()
                .filter(|local| !matching_symbols(file, local).is_empty())
                .map(|local| (rel.to_string(), local.clone())),
        );
    }
    for (target, bindings) in &file.resolved_import_bindings {
        for (exported, imported) in bindings {
            let next_name = if exported.strip_prefix("export:") == Some(public_name) {
                Some(imported.as_str())
            } else if exported == "export:*" && imported == "*" && public_name != "default" {
                Some(public_name)
            } else {
                None
            };
            if let Some(next_name) = next_name {
                candidates.extend(reexport_candidates(
                    project,
                    target,
                    next_name,
                    depth + 1,
                    seen,
                ));
            }
        }
    }
    candidates
}
