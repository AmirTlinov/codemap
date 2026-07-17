// Responsibility: map-symbols-jsx-and-exports
use crate::map::{
    ImportedSymbolReference, ImportedSymbolReferenceKind, file_has_local_value_shadow,
    file_references_static_expression_after_imports, identifier_ranges,
    imported_symbol_binding_matches, is_identifier_byte, matching_symbols, previous_nonspace_byte,
    structural_edge_with_locations, symbol_anchor_path, symbol_definition_location,
    symbol_is_exported,
};
use crate::model::{EvidenceStrength, FileInfo, Project, StructuralEdge};
use std::collections::BTreeSet;
use std::path::Path;

mod barrel_resolution;
pub(crate) use barrel_resolution::*;
mod local_named_exports;
pub(crate) use local_named_exports::*;

pub(crate) fn line_has_jsx_tag_identifier_reference(line: &str, name: &str) -> bool {
    if !name
        .bytes()
        .next()
        .map(|byte| byte.is_ascii_uppercase())
        .unwrap_or(false)
    {
        return false;
    }
    identifier_ranges(line, name).any(|(start, end)| {
        let before = &line[..start];
        let Some(tag_start) = before.rfind('<') else {
            return false;
        };
        if previous_nonspace_byte(&before[tag_start + 1..]).is_some() {
            return false;
        }
        let tag_before = before[..tag_start].bytes().next_back();
        if tag_before
            .map(|byte| is_identifier_byte(byte) || matches!(byte, b'.' | b'$' | b'/'))
            .unwrap_or(false)
        {
            return false;
        }
        let after = line[end..].trim_start();
        if matches!(after.bytes().next(), Some(b'|' | b'&' | b',' | b'=')) {
            return false;
        }
        if after.starts_with("extends ")
            || after.starts_with("extends\t")
            || after.starts_with("extends\n")
            || after.starts_with(">()")
            || after.starts_with("> ()")
        {
            return false;
        }
        line[end..]
            .bytes()
            .next()
            .map(|byte| byte.is_ascii_whitespace() || matches!(byte, b'>' | b'/'))
            .unwrap_or(true)
    })
}

pub(crate) fn symbol_contract_edges(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
) -> Vec<StructuralEdge> {
    let exported = symbol_is_exported(project, file_rel, symbol_name);
    if !exported {
        return Vec::new();
    }
    vec![structural_edge_with_locations(
        symbol_anchor_path(file_rel, symbol_name),
        file_rel.to_string(),
        "contract",
        "exported_symbol",
        EvidenceStrength::High,
        symbol_definition_location(project, file_rel, symbol_name, "exported_symbol"),
    )]
}

pub(crate) fn file_imported_symbol_reference(
    project: &Project,
    file: &FileInfo,
    file_rel: &str,
    symbol_name: &str,
) -> Option<ImportedSymbolReference> {
    file_imported_symbol_reference_with_cache(
        project,
        file,
        file_rel,
        symbol_name,
        &mut BarrelResolutionCache::default(),
    )
}

pub(crate) fn file_imported_symbol_reference_with_cache(
    project: &Project,
    file: &FileInfo,
    file_rel: &str,
    symbol_name: &str,
    barrel_cache: &mut BarrelResolutionCache,
) -> Option<ImportedSymbolReference> {
    if let Some(expression) =
        file_directly_references_imported_symbol(project, file, file_rel, symbol_name)
    {
        return Some(ImportedSymbolReference {
            kind: ImportedSymbolReferenceKind::Direct,
            expression,
        });
    }
    if file_statically_includes(file, file_rel)
        && file_references_static_expression_after_imports(project, file, symbol_name)
    {
        return Some(ImportedSymbolReference {
            kind: ImportedSymbolReferenceKind::Included,
            expression: symbol_name.to_string(),
        });
    }
    if let Some(expression) =
        file_references_reexported_symbol(project, file, file_rel, symbol_name, barrel_cache)
    {
        return Some(ImportedSymbolReference {
            kind: ImportedSymbolReferenceKind::Reexported,
            expression,
        });
    }
    None
}

fn file_statically_includes(file: &FileInfo, target_rel: &str) -> bool {
    if file.ext != "rs" {
        return false;
    }
    let parent = Path::new(&file.rel)
        .parent()
        .unwrap_or_else(|| Path::new("."));
    file.imports.iter().any(|spec| {
        spec.ends_with(".rs")
            && crate::repo::normalize_rel_path(&parent.join(spec).to_string_lossy()) == target_rel
    })
}

pub(crate) fn file_imported_symbol_reference_kind(
    project: &Project,
    file: &FileInfo,
    file_rel: &str,
    symbol_name: &str,
) -> Option<ImportedSymbolReferenceKind> {
    file_imported_symbol_reference(project, file, file_rel, symbol_name)
        .map(|reference| reference.kind)
}

fn file_directly_references_imported_symbol(
    project: &Project,
    file: &FileInfo,
    file_rel: &str,
    symbol_name: &str,
) -> Option<String> {
    for (target_rel, bindings) in &file.resolved_import_bindings {
        if !import_target_contains_anchor(project, target_rel, file_rel) {
            continue;
        }
        for (local, imported) in bindings {
            if imported == "*" {
                let expression = if local == "*" {
                    symbol_name.to_string()
                } else {
                    qualified_symbol_expression(file, local, symbol_name)
                };
                let shadowed = if local == "*" {
                    file_has_local_value_shadow(file, symbol_name)
                } else {
                    file_has_local_value_shadow(file, local)
                };
                if !shadowed
                    && file_references_static_expression_after_imports(project, file, &expression)
                {
                    return Some(expression);
                }
                continue;
            }
            let qualified = qualified_symbol_expression(file, local, symbol_name);
            if module_binding_matches_target(target_rel, imported)
                && !file_has_local_value_shadow(file, local)
                && file_references_static_expression_after_imports(project, file, &qualified)
            {
                return Some(qualified);
            }
            if imported_binding_matches_anchor(project, file_rel, symbol_name, imported)
                && !file_has_local_value_shadow(file, local)
                && (file.jsx_tags.contains(local)
                    || file_references_static_expression_after_imports(project, file, local))
            {
                return Some(local.clone());
            }
        }
    }
    None
}

pub(crate) fn module_binding_matches_target(target_rel: &str, imported: &str) -> bool {
    let target = Path::new(target_rel);
    target.file_stem().and_then(|name| name.to_str()) == Some(imported)
        || (target.file_name().and_then(|name| name.to_str()) == Some("mod.rs")
            && target
                .parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                == Some(imported))
}

fn imported_binding_matches_anchor(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
    imported: &str,
) -> bool {
    if imported_symbol_binding_matches(project, file_rel, symbol_name, imported) {
        return true;
    }
    project.files.get(file_rel).is_some_and(|anchor| {
        anchor.ext == "py"
            && imported == symbol_name
            && !matching_symbols(anchor, symbol_name).is_empty()
    })
}

fn import_target_contains_anchor(project: &Project, target_rel: &str, anchor_rel: &str) -> bool {
    if target_rel == anchor_rel {
        return true;
    }
    let Some(target) = project.files.get(target_rel) else {
        return false;
    };
    let Some(anchor) = project.files.get(anchor_rel) else {
        return false;
    };
    target.ext == "go"
        && anchor.ext == "go"
        && Path::new(&target.rel).parent() == Path::new(&anchor.rel).parent()
}

fn file_references_reexported_symbol(
    project: &Project,
    file: &FileInfo,
    file_rel: &str,
    symbol_name: &str,
    barrel_cache: &mut BarrelResolutionCache,
) -> Option<String> {
    let mut expressions = BTreeSet::new();
    for (barrel_rel, imported_from_barrel) in &file.resolved_import_bindings {
        if barrel_rel == file_rel || !barrel_cache.may_reexport_from(project, barrel_rel, file_rel)
        {
            continue;
        }
        for (local, imported_public_name) in imported_from_barrel {
            if imported_public_name == "*" {
                if local == "*" {
                    expressions.insert(symbol_name.to_string());
                } else {
                    expressions.insert(qualified_symbol_expression(file, local, symbol_name));
                }
            } else {
                expressions.insert(local.clone());
                expressions.insert(qualified_symbol_expression(file, local, symbol_name));
            }
        }
    }
    let referenced = crate::map::file_referenced_static_expressions(project, file, &expressions);
    for (barrel_rel, imported_from_barrel) in &file.resolved_import_bindings {
        if barrel_rel == file_rel {
            continue;
        }
        if !barrel_cache.may_reexport_from(project, barrel_rel, file_rel) {
            continue;
        }
        let Some(barrel) = project.files.get(barrel_rel) else {
            continue;
        };
        for (local, imported_public_name) in imported_from_barrel {
            if file_has_local_value_shadow(file, local) {
                continue;
            }
            let qualified = qualified_symbol_expression(file, local, symbol_name);
            let plain_reference = imported_public_name != "*"
                && (file.jsx_tags.contains(local) || referenced.contains(local));
            let glob_expression = if local == "*" {
                symbol_name.to_string()
            } else {
                qualified.clone()
            };
            let glob_reference =
                imported_public_name == "*" && referenced.contains(&glob_expression);
            let qualified_reference =
                imported_public_name != "*" && referenced.contains(&qualified);
            if !plain_reference && !glob_reference && !qualified_reference {
                continue;
            }
            if plain_reference
                && barrel_reexports_symbol_from_file(
                    project,
                    barrel,
                    file_rel,
                    symbol_name,
                    imported_public_name,
                    barrel_cache,
                )
            {
                return Some(local.clone());
            }
            if glob_reference
                && barrel_reexports_symbol_from_file(
                    project,
                    barrel,
                    file_rel,
                    symbol_name,
                    symbol_name,
                    barrel_cache,
                )
            {
                return Some(glob_expression);
            }
            if qualified_reference
                && barrel_reexports_symbol_from_file(
                    project,
                    barrel,
                    file_rel,
                    symbol_name,
                    symbol_name,
                    barrel_cache,
                )
            {
                return Some(qualified);
            }
        }
    }
    None
}

fn qualified_symbol_expression(file: &FileInfo, local: &str, symbol_name: &str) -> String {
    if matches!(
        file.ext.as_str(),
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte" | "py" | "go"
    ) {
        format!("{local}.{symbol_name}")
    } else {
        format!("{local}::{symbol_name}")
    }
}
