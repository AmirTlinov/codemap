// Responsibility: exact-cone public contract adjacency
use crate::map::{contract_proof_edges, sort_edges, symbol_contract_edges};
use crate::model::{Project, StructuralEdge};
use std::collections::BTreeSet;

pub(crate) fn adjacent_public_contract_edges(
    project: &Project,
    outgoing: &[StructuralEdge],
) -> Vec<StructuralEdge> {
    let targets = outgoing
        .iter()
        .filter(|edge| edge.edge_type == "imports")
        .map(|edge| edge.to.as_str())
        .collect::<BTreeSet<_>>();
    let mut edges = targets
        .into_iter()
        .filter_map(|target| project.files.get(target))
        .flat_map(|file| {
            file.symbols
                .iter()
                .filter(|symbol| symbol.exported)
                .flat_map(|symbol| symbol_contract_edges(project, &file.rel, &symbol.name))
        })
        .filter(|edge| edge.evidence == "public_symbol_type_dependency")
        .collect::<Vec<_>>();
    sort_edges(&mut edges);
    edges
}

pub(crate) fn public_contract_proof_edges(
    project: &Project,
    contract_edges: &[StructuralEdge],
) -> Vec<StructuralEdge> {
    let targets = contract_edges
        .iter()
        .filter(|edge| edge.evidence == "public_symbol_type_dependency")
        .map(|edge| edge.to.clone())
        .collect::<BTreeSet<_>>();
    let mut proof = targets
        .into_iter()
        .flat_map(|target| contract_proof_edges(project, &[target]))
        .filter(|edge| edge.evidence.ends_with("_via_direct_consumer"))
        .collect::<Vec<_>>();
    sort_edges(&mut proof);
    proof
}
