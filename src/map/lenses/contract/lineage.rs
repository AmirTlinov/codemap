// Responsibility: contract-lineage-fact-assembly
mod codegen;
mod declarations;
mod documentation;
mod graph;
mod parallel;
mod sql;

use crate::map::sort_edges;
use crate::model::{Project, StructuralEdge, Surface, Unknown};

pub(crate) use documentation::contract_document_candidates;

#[derive(Default)]
pub(crate) struct ContractLineageFacts {
    pub(crate) declarations: Vec<Surface>,
    pub(crate) edges: Vec<StructuralEdge>,
    pub(crate) proof: Vec<StructuralEdge>,
    pub(crate) unknowns: Vec<Unknown>,
}

pub(crate) fn contract_lineage(project: &Project, rel: &str) -> ContractLineageFacts {
    let mut facts = ContractLineageFacts::default();
    let mut seeds = Vec::new();
    facts
        .edges
        .extend(parallel::parallel_contract_edges(project, rel));
    facts
        .edges
        .extend(documentation::contract_document_candidates(project, rel));
    if rel.ends_with(".sql") {
        let sql = sql::sql_lineage(project, rel);
        facts.declarations.extend(sql.declarations);
        facts.edges.extend(sql.edges);
        facts.unknowns.extend(sql.unknowns);
        seeds.extend(sql.consumer_files);
    }
    if codegen::supported_contract_source(project, rel) {
        let generated = codegen::codegen_lineage(project, rel);
        facts.declarations.extend(generated.declarations);
        facts.edges.extend(generated.edges);
        facts.proof.extend(generated.proof);
        facts.unknowns.extend(generated.unknowns);
    }
    if !seeds.is_empty() {
        let connected = graph::connected_lineage(project, &seeds);
        facts.declarations.extend(connected.declarations);
        facts.edges.extend(connected.edges);
        facts.proof.extend(connected.proof);
        facts.unknowns.extend(connected.unknowns);
    }
    facts.declarations.sort_by(|a, b| a.id.cmp(&b.id));
    facts.declarations.dedup_by(|a, b| a.id == b.id);
    sort_edges(&mut facts.edges);
    sort_edges(&mut facts.proof);
    facts.unknowns.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then_with(|| a.line_start.cmp(&b.line_start))
            .then_with(|| a.kind.cmp(&b.kind))
    });
    facts
        .unknowns
        .dedup_by(|a, b| a.kind == b.kind && a.path == b.path && a.line_start == b.line_start);
    facts
}

pub(super) fn entity_surface(
    id: String,
    kind: &str,
    rel: &str,
    line: usize,
    evidence: &str,
) -> Surface {
    Surface {
        id,
        kind: kind.to_string(),
        path: Some(rel.to_string()),
        role: Some("contract_lineage_entity".to_string()),
        evidence: evidence.to_string(),
        strength: crate::model::EvidenceStrength::High,
        count: Some(1),
        examples: vec![format!("{rel}:{line}")],
        hidden_count: 0,
    }
}
