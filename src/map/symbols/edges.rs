// Responsibility: map-symbols-edges
use crate::map::{
    file_imported_symbol_reference_kind, first_identifier_reference_location, matching_symbols,
    same_scope_file_references_symbol, sort_edges, structural_edge_with_locations,
    symbol_anchor_path,
};
use crate::model::{EvidenceStrength, Project, StructuralEdge};

mod proof_edges;
pub(crate) use proof_edges::*;
mod symbol_uses;
pub(crate) use symbol_uses::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImportedSymbolReferenceKind {
    Direct,
    Reexported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BarrelPublicNameResolution {
    Explicit {
        target_rel: String,
        imported_name: String,
    },
    Star {
        target_rel: String,
    },
}

pub(crate) fn symbol_reference_edges(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
    include_tests: bool,
) -> Vec<StructuralEdge> {
    let Some(anchor) = project.files.get(file_rel) else {
        return Vec::new();
    };
    if matching_symbols(anchor, symbol_name).is_empty() {
        return Vec::new();
    }
    let anchor_path = symbol_anchor_path(file_rel, symbol_name);
    let mut edges = Vec::new();
    for file in project.files.values() {
        if file.rel == file_rel {
            continue;
        }
        if !include_tests && (file.has_role("test") || file.has_role("test_support")) {
            continue;
        }
        if let Some(kind) =
            file_imported_symbol_reference_kind(project, file, file_rel, symbol_name)
        {
            edges.push(structural_edge_with_locations(
                file.rel.clone(),
                anchor_path.clone(),
                "symbol_reference",
                match kind {
                    ImportedSymbolReferenceKind::Direct => "imported_symbol_reference",
                    ImportedSymbolReferenceKind::Reexported => "reexported_symbol_reference",
                },
                EvidenceStrength::High,
                first_identifier_reference_location(
                    project,
                    &file.rel,
                    symbol_name,
                    "symbol_reference",
                ),
            ));
            continue;
        }
        if same_scope_file_references_symbol(anchor, file, symbol_name) {
            edges.push(structural_edge_with_locations(
                file.rel.clone(),
                anchor_path.clone(),
                "symbol_reference",
                "same_scope_symbol_reference",
                EvidenceStrength::High,
                first_identifier_reference_location(
                    project,
                    &file.rel,
                    symbol_name,
                    "symbol_reference",
                ),
            ));
        }
    }
    sort_edges(&mut edges);
    edges
}
