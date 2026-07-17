// Responsibility: exact-parallel-contract-definition-lineage
use crate::evidence::import_statement_locations;
use crate::map::{
    schema_owner_path, structural_edge_with_locations, symbol_anchor_path,
    symbol_definition_location,
};
use crate::model::{EvidenceStrength, Project, StructuralEdge};

pub(super) fn parallel_contract_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let Some(anchor) = project.files.get(rel) else {
        return Vec::new();
    };
    let mut edges = Vec::new();
    for name in anchor
        .exports
        .iter()
        .filter(|name| anchor.symbols.iter().any(|symbol| symbol.name == ***name))
    {
        let anchor_symbol = symbol_anchor_path(rel, name);
        for candidate in project.files.values().filter(|candidate| {
            candidate.rel != rel && candidate.symbols.iter().any(|symbol| symbol.name == *name)
        }) {
            let candidate_symbol = symbol_anchor_path(&candidate.rel, name);
            let mut locations =
                symbol_definition_location(project, rel, name, "contract_definition");
            locations.extend(symbol_definition_location(
                project,
                &candidate.rel,
                name,
                "parallel_definition",
            ));
            edges.push(structural_edge_with_locations(
                anchor_symbol.clone(),
                candidate_symbol,
                "parallel_definition",
                "exact_exported_symbol_name",
                EvidenceStrength::High,
                locations,
            ));
            for dependency in candidate.resolved_imports.iter().filter(|dependency| {
                project.files.get(*dependency).is_some_and(|file| {
                    file.has_role("schema_contract") || schema_owner_path(dependency)
                })
            }) {
                let mut locations = symbol_definition_location(
                    project,
                    &candidate.rel,
                    name,
                    "parallel_definition",
                );
                locations.extend(import_statement_locations(
                    project,
                    &candidate.rel,
                    dependency,
                ));
                edges.push(structural_edge_with_locations(
                    anchor_symbol.clone(),
                    dependency.clone(),
                    "parallel_contract_dependency",
                    "exact_parallel_definition_schema_import",
                    EvidenceStrength::High,
                    locations,
                ));
            }
        }
    }
    edges
}
