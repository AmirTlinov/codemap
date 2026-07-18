// Responsibility: downstream consumers of public symbol contract dependencies
use crate::map::{
    direct_consumer_edges, package_for_rel, sort_edges, structural_edge_with_locations,
};
use crate::model::{EvidenceStrength, Project, StructuralEdge};

pub(crate) fn symbol_contract_consumer_edges(
    project: &Project,
    contract_edges: &[StructuralEdge],
) -> Vec<StructuralEdge> {
    let mut consumers = contract_edges
        .iter()
        .filter(|edge| edge.evidence == "public_symbol_type_dependency")
        .flat_map(|edge| cross_package_consumers(project, &edge.to))
        .collect::<Vec<_>>();
    sort_edges(&mut consumers);
    consumers
}

fn cross_package_consumers(project: &Project, contract_rel: &str) -> Vec<StructuralEdge> {
    let contract_package = package_for_rel(project, contract_rel).map(|package| &package.path);
    direct_consumer_edges(project, contract_rel)
        .into_iter()
        .filter(|edge| {
            package_for_rel(project, &edge.from).map(|package| &package.path) != contract_package
        })
        .map(|edge| {
            structural_edge_with_locations(
                edge.from,
                contract_rel.to_string(),
                "contract_consumer",
                "public_symbol_type_consumer",
                EvidenceStrength::High,
                edge.locations,
            )
        })
        .collect()
}
