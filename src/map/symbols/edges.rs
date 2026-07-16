// Responsibility: map-symbols-edges
use crate::map::{
    BarrelResolutionCache, file_imported_symbol_reference_with_cache,
    first_identifier_reference_location, matching_symbols, same_scope_file_references_symbol,
    sort_edges, static_expression_reference_location, structural_edge_with_locations,
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
    Included,
    Reexported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImportedSymbolReference {
    pub(crate) kind: ImportedSymbolReferenceKind,
    pub(crate) expression: String,
}

pub(crate) struct SymbolReferenceEdgeSet {
    all: Vec<StructuralEdge>,
    production: Vec<StructuralEdge>,
}

impl SymbolReferenceEdgeSet {
    pub(crate) fn all(&self) -> &[StructuralEdge] {
        &self.all
    }

    pub(crate) fn production(&self) -> &[StructuralEdge] {
        &self.production
    }
}

pub(crate) fn symbol_reference_edge_set(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
) -> SymbolReferenceEdgeSet {
    let all = symbol_reference_edges(project, file_rel, symbol_name, true);
    let production = all
        .iter()
        .filter(|edge| {
            !project
                .files
                .get(&edge.from)
                .is_some_and(|file| file.has_role("test") || file.has_role("test_support"))
        })
        .cloned()
        .collect();
    SymbolReferenceEdgeSet { all, production }
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
    let mut barrel_cache = BarrelResolutionCache::default();
    for file in project.files.values() {
        if file.rel == file_rel {
            continue;
        }
        if !include_tests && (file.has_role("test") || file.has_role("test_support")) {
            continue;
        }
        if let Some(reference) = file_imported_symbol_reference_with_cache(
            project,
            file,
            file_rel,
            symbol_name,
            &mut barrel_cache,
        ) {
            edges.push(structural_edge_with_locations(
                file.rel.clone(),
                anchor_path.clone(),
                "symbol_reference",
                match reference.kind {
                    ImportedSymbolReferenceKind::Direct => "imported_symbol_reference",
                    ImportedSymbolReferenceKind::Included => "included_symbol_reference",
                    ImportedSymbolReferenceKind::Reexported => "reexported_symbol_reference",
                },
                EvidenceStrength::High,
                static_expression_reference_location(
                    project,
                    file,
                    &reference.expression,
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
