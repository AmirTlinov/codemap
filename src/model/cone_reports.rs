// Responsibility: cone-and-where-report-types
use serde::{Deserialize, Serialize};

use super::FlowStep;

use super::{
    CountFact, EnvDeclaration, EvidenceStrength, FileSummary, HiddenGroup, StructuralEdge, Unknown,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Surface {
    pub id: String,
    pub kind: String,
    pub path: Option<String>,
    pub role: Option<String>,
    pub evidence: String,
    pub strength: EvidenceStrength,
    pub count: Option<usize>,
    pub examples: Vec<String>,
    pub hidden_count: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConeReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub anchor: FileSummary,
    pub depth: usize,
    pub xray: XrayCard,
    pub declared_env: Vec<EnvDeclaration>,
    pub outgoing: Vec<StructuralEdge>,
    pub incoming: Vec<StructuralEdge>,
    pub proof: Vec<StructuralEdge>,
    pub contracts: Vec<StructuralEdge>,
    pub boundary: Vec<StructuralEdge>,
    pub hidden: Vec<HiddenGroup>,
    pub unknowns: Vec<Unknown>,
    pub expand: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct XrayCard {
    pub roles: Vec<Surface>,
    pub inputs: Vec<StructuralEdge>,
    pub outputs: Vec<Surface>,
    pub state: Vec<Surface>,
    pub side_effects: Vec<Surface>,
    pub direct_consumers: Vec<StructuralEdge>,
    pub mediated_consumers: Vec<StructuralEdge>,
    pub flow: Vec<FlowStep>,
    pub nearby: Vec<Surface>,
    pub proof_hard: Vec<StructuralEdge>,
    pub proof_direct: Vec<StructuralEdge>,
    pub proof_mediated: Vec<StructuralEdge>,
    pub proof_soft: Vec<StructuralEdge>,
    pub unknowns: Vec<Unknown>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WhereReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub query: String,
    pub kind_filter: Option<String>,
    pub total_matches: usize,
    pub definitions: Vec<WhereDefinition>,
    pub soft_suggestions: Vec<WhereSuggestion>,
    pub unknowns: Vec<Unknown>,
    pub hidden: Vec<HiddenGroup>,
    pub expand: Vec<String>,
    // Rich single-definition cone map, rendered only (kept out of JSON to keep the
    // where contract flat). When there is exactly one match, `where` is structurally
    // identical to `cone file#symbol`.
    #[serde(skip)]
    pub detail: Option<Box<ConeReport>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WhereDefinition {
    pub anchor: FileSummary,
    pub consumers: Vec<StructuralEdge>,
    pub consumers_total: CountFact,
    pub hidden: Vec<HiddenGroup>,
    pub expand: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WhereSuggestion {
    pub name: String,
    pub defined_in: String,
    pub definition_count: usize,
    pub expand: String,
}
