// Responsibility: map-symbols-jsx-and-exports
use crate::map::{
    ImportedSymbolReferenceKind, file_has_local_value_shadow,
    file_references_identifier_after_imports, identifier_ranges, imported_symbol_binding_matches,
    is_identifier_byte, previous_nonspace_byte, structural_edge_with_locations, symbol_anchor_path,
    symbol_definition_location, symbol_is_exported,
};
use crate::model::{EvidenceStrength, FileInfo, Project, StructuralEdge};

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

pub(crate) fn file_imported_symbol_reference_kind(
    project: &Project,
    file: &FileInfo,
    file_rel: &str,
    symbol_name: &str,
) -> Option<ImportedSymbolReferenceKind> {
    if file_directly_references_imported_symbol(project, file, file_rel, symbol_name) {
        return Some(ImportedSymbolReferenceKind::Direct);
    }
    if file_references_reexported_symbol(project, file, file_rel, symbol_name) {
        return Some(ImportedSymbolReferenceKind::Reexported);
    }
    None
}

fn file_directly_references_imported_symbol(
    project: &Project,
    file: &FileInfo,
    file_rel: &str,
    symbol_name: &str,
) -> bool {
    let Some(bindings) = file.resolved_import_bindings.get(file_rel) else {
        return false;
    };
    bindings.iter().any(|(local, imported)| {
        imported_symbol_binding_matches(project, file_rel, symbol_name, imported)
            && !file_has_local_value_shadow(file, local)
            && (file.jsx_tags.contains(local)
                || file_references_identifier_after_imports(project, file, local))
    })
}

fn file_references_reexported_symbol(
    project: &Project,
    file: &FileInfo,
    file_rel: &str,
    symbol_name: &str,
) -> bool {
    for (barrel_rel, imported_from_barrel) in &file.resolved_import_bindings {
        if barrel_rel == file_rel {
            continue;
        }
        let Some(barrel) = project.files.get(barrel_rel) else {
            continue;
        };
        for (local, imported_public_name) in imported_from_barrel {
            if !barrel_reexports_symbol_from_file(
                project,
                barrel,
                file_rel,
                symbol_name,
                imported_public_name,
            ) {
                continue;
            }
            if file_has_local_value_shadow(file, local) {
                continue;
            }
            if file.jsx_tags.contains(local)
                || file_references_identifier_after_imports(project, file, local)
            {
                return true;
            }
        }
    }
    false
}
