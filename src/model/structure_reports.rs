// Responsibility: structural-listing-and-graph-report-types
use serde::{Deserialize, Serialize};

use super::{
    BoundaryFacts, DomainRef, EvidenceLocation, EvidenceStrength, FileSummary, HiddenGroup,
    StructuralEdge,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LsReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub path: String,
    pub mode: String,
    pub anchor: Option<FileSummary>,
    pub directory: Vec<DirectorySurface>,
    pub boundary_facts: BoundaryFacts,
    pub edges: Vec<StructuralEdge>,
    pub hidden: Vec<HiddenGroup>,
    pub next: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DirectorySurface {
    pub id: String,
    pub kind: String,
    pub path: Option<String>,
    pub role: Option<String>,
    pub evidence: String,
    pub strength: EvidenceStrength,
    pub count: usize,
    pub examples: Vec<String>,
    pub hidden_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphLens {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub domain: DomainRef,
    pub lens: String,
    pub nodes: Vec<String>,
    pub edges: Vec<GraphEdge>,
    pub hidden: Vec<HiddenGroup>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    #[serde(rename = "type")]
    pub edge_type: String,
    pub evidence: String,
    pub strength: EvidenceStrength,
    pub locations: Vec<EvidenceLocation>,
}
