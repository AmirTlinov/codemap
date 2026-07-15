// Responsibility: map-symbols-barrel-resolution
use super::local_named_export_statement_slices;
use crate::map::{
    BarrelPublicNameResolution, file_has_inline_named_export, file_has_named_public_export,
    imported_symbol_binding_matches, local_named_export_bindings_from_statement,
    symbol_exported_under_public_name,
};
use crate::model::{FileInfo, Project};
use std::collections::BTreeSet;

pub(crate) fn barrel_reexports_symbol_from_file(
    project: &Project,
    barrel: &FileInfo,
    file_rel: &str,
    symbol_name: &str,
    imported_public_name: &str,
) -> bool {
    let mut seen = BTreeSet::new();
    barrel_reexports_symbol_from_file_inner(
        project,
        barrel,
        file_rel,
        symbol_name,
        imported_public_name,
        &mut seen,
    )
}

fn barrel_reexports_symbol_from_file_inner(
    project: &Project,
    barrel: &FileInfo,
    file_rel: &str,
    symbol_name: &str,
    imported_public_name: &str,
    seen: &mut BTreeSet<(String, String)>,
) -> bool {
    if !seen.insert((barrel.rel.clone(), imported_public_name.to_string())) {
        return false;
    }
    match barrel_public_name_resolution(project, barrel, imported_public_name) {
        Some(BarrelPublicNameResolution::Explicit {
            target_rel,
            imported_name,
        }) => {
            let direct_match = target_rel == file_rel
                && imported_symbol_binding_matches(project, file_rel, symbol_name, &imported_name);
            if direct_match {
                return true;
            }
            project
                .files
                .get(&target_rel)
                .map(|target| {
                    barrel_reexports_symbol_from_file_inner(
                        project,
                        target,
                        file_rel,
                        symbol_name,
                        &imported_name,
                        seen,
                    )
                })
                .unwrap_or(false)
        }
        Some(BarrelPublicNameResolution::Star { target_rel }) => {
            let direct_match = target_rel == file_rel
                && symbol_exported_under_public_name(
                    project,
                    file_rel,
                    symbol_name,
                    imported_public_name,
                );
            if direct_match {
                return true;
            }
            project
                .files
                .get(&target_rel)
                .map(|target| {
                    barrel_reexports_symbol_from_file_inner(
                        project,
                        target,
                        file_rel,
                        symbol_name,
                        imported_public_name,
                        seen,
                    )
                })
                .unwrap_or(false)
        }
        None => false,
    }
}

fn barrel_public_name_resolution(
    project: &Project,
    barrel: &FileInfo,
    public_name: &str,
) -> Option<BarrelPublicNameResolution> {
    let mut seen = BTreeSet::new();
    barrel_public_name_resolution_inner(project, barrel, public_name, &mut seen)
}

fn barrel_public_name_resolution_inner(
    project: &Project,
    barrel: &FileInfo,
    public_name: &str,
    seen: &mut BTreeSet<(String, String)>,
) -> Option<BarrelPublicNameResolution> {
    if !seen.insert((barrel.rel.clone(), public_name.to_string())) {
        return None;
    }
    let mut explicit_owners = BTreeSet::new();
    if file_has_inline_named_export(project, &barrel.rel, public_name)
        || barrel_has_local_named_export_public_name(project, barrel, public_name)
    {
        explicit_owners.insert((barrel.rel.clone(), public_name.to_string()));
    }
    for (target_rel, bindings) in &barrel.resolved_import_bindings {
        for (exported, imported_name) in bindings {
            if exported.strip_prefix("export:") == Some(public_name) {
                explicit_owners.insert((target_rel.clone(), imported_name.clone()));
            }
        }
    }
    if !explicit_owners.is_empty() {
        if explicit_owners.len() != 1 {
            return None;
        }
        let (target_rel, imported_name) = explicit_owners.into_iter().next()?;
        return Some(BarrelPublicNameResolution::Explicit {
            target_rel,
            imported_name,
        });
    }
    if public_name == "default" {
        return None;
    }

    let star_owners = barrel
        .resolved_import_bindings
        .iter()
        .filter(|(target_rel, bindings)| {
            bindings
                .get("export:*")
                .map(|value| {
                    value == "*"
                        && file_exposes_public_name_for_star(project, target_rel, public_name, seen)
                })
                .unwrap_or(false)
        })
        .map(|(target_rel, _)| target_rel.clone())
        .collect::<BTreeSet<_>>();
    if star_owners.len() != 1 {
        return None;
    }
    star_owners
        .into_iter()
        .next()
        .map(|target_rel| BarrelPublicNameResolution::Star { target_rel })
}

fn file_exposes_public_name_for_star(
    project: &Project,
    file_rel: &str,
    public_name: &str,
    seen: &BTreeSet<(String, String)>,
) -> bool {
    if public_name == "default" {
        return false;
    }
    if file_has_named_public_export(project, file_rel, public_name) {
        return true;
    }
    let Some(file) = project.files.get(file_rel) else {
        return false;
    };
    let mut candidate_seen = seen.clone();
    barrel_public_name_resolution_inner(project, file, public_name, &mut candidate_seen).is_some()
}

fn barrel_has_local_named_export_public_name(
    project: &Project,
    barrel: &FileInfo,
    public_name: &str,
) -> bool {
    let Some(text) = project.read_indexed_text(&barrel.rel) else {
        return false;
    };
    for statement in local_named_export_statement_slices(&text) {
        if local_named_export_bindings_from_statement(statement).contains_key(public_name) {
            return true;
        }
    }
    false
}
