// Responsibility: changed-diff-impact-report-types
use serde::{Deserialize, Serialize};

use super::{EnvSurface, ProofMapReport, RuntimeRoute, Surface};

use super::{
    BoundaryFacts, EvidenceLocation, FileSummary, HiddenGroup, ProofReport, ProofSurface,
    ProofWiringFact, StructuralEdge, Unknown,
};

#[derive(Debug, Clone, Serialize)]
pub struct ImpactReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub selector: String,
    pub changed: Vec<FileSummary>,
    pub clusters: Vec<ImpactCluster>,
    pub hidden: Vec<HiddenGroup>,
    pub unknowns: Vec<Unknown>,
    pub expand: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImpactCluster {
    pub id: String,
    pub changed: Vec<String>,
    pub direct_consumers: Vec<StructuralEdge>,
    pub cross_boundary_consumers: Vec<StructuralEdge>,
    #[serde(rename = "contract_links", alias = "contract_risks")]
    pub contract_links: Vec<StructuralEdge>,
    pub proof: Vec<StructuralEdge>,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiffMapReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub selector: String,
    pub changed: Vec<FileSummary>,
    pub added_edges: Vec<StructuralEdge>,
    pub removed_edges: Vec<StructuralEdge>,
    pub changed_symbols: Vec<ChangedSymbol>,
    pub added_exports: Vec<Surface>,
    pub removed_exports: Vec<Surface>,
    pub added_runtime_routes: Vec<RuntimeRoute>,
    pub removed_runtime_routes: Vec<RuntimeRoute>,
    pub added_env: Vec<EnvSurface>,
    pub removed_env: Vec<EnvSurface>,
    pub added_proof_surfaces: Vec<ProofSurface>,
    pub removed_proof_surfaces: Vec<ProofSurface>,
    pub new_unknowns: Vec<Unknown>,
    pub hidden: Vec<HiddenGroup>,
    pub expand: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChangedReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub selector: String,
    pub session_snapshot: SessionSnapshot,
    pub selection: ChangeSelection,
    #[serde(skip)]
    pub display_limit: usize,
    #[serde(skip)]
    pub proof_plan_cache: Option<Box<ProofReport>>,
    #[serde(skip)]
    pub proof_map_cache: Option<Box<ProofMapReport>>,
    pub total_changed_count: usize,
    pub changed: Vec<FileSummary>,
    pub git_state: Vec<GitChange>,
    pub structural_events: Vec<ChangedStructuralEvent>,
    pub map_delta: ChangedMapDelta,
    pub risks: Vec<ChangedRisk>,
    pub coupling: Vec<ChangedCouplingFact>,
    pub boundary_facts: BoundaryFacts,
    pub impact: Vec<ImpactCluster>,
    pub proof: ChangedProofSummary,
    pub unknowns: Vec<Unknown>,
    pub hidden: Vec<HiddenGroup>,
    pub expand: Vec<String>,
}

impl ChangedReport {
    pub const SCHEMA_VERSION: &'static str = "11";
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SessionSnapshot {
    pub token: String,
    pub created_unix_seconds: Option<u64>,
    pub file_count: usize,
    pub content_files: usize,
    pub storage: String,
    pub freshness: String,
    pub reuse: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ChangeSelection {
    pub kind: String,
    pub requested: Option<String>,
    pub resolved: bool,
    pub selected_files: usize,
    pub fallback_files: usize,
    pub content_complete: bool,
    pub baseline_snapshot: Option<SessionSnapshot>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GitChange {
    pub path: String,
    pub old_path: Option<String>,
    pub status: String,
    pub staged: bool,
    pub unstaged: bool,
    #[serde(default = "default_git_change_provenance")]
    pub provenance: String,
}

fn default_git_change_provenance() -> String {
    "git_status".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChangedStructuralEvent {
    pub kind: String,
    pub path: String,
    pub old_path: Option<String>,
    pub evidence: String,
    pub effect: String,
    pub locations: Vec<EvidenceLocation>,
    pub expand: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChangedMapDelta {
    pub added_edges: usize,
    pub removed_edges: usize,
    pub changed_symbols: usize,
    pub added_exports: usize,
    pub removed_exports: usize,
    pub added_runtime_routes: usize,
    pub removed_runtime_routes: usize,
    pub added_env: usize,
    pub removed_env: usize,
    pub added_proof_surfaces: usize,
    pub removed_proof_surfaces: usize,
    pub new_unknowns: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChangedRisk {
    pub kind: String,
    pub severity: String,
    pub count: usize,
    pub paths: Vec<String>,
    pub evidence: Vec<EvidenceLocation>,
    pub effect: String,
    pub expand: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChangedCouplingFact {
    pub kind: String,
    pub status: String,
    pub paths: Vec<String>,
    pub evidence: Vec<EvidenceLocation>,
    pub effect: String,
    pub expand: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChangedProofSummary {
    pub commands: Vec<ChangedProofCommand>,
    pub fallback: Vec<String>,
    pub hard: Vec<ProofSurface>,
    pub direct_evidence: Vec<ProofSurface>,
    pub mediated_evidence: Vec<ProofSurface>,
    pub soft_evidence: Vec<ProofSurface>,
    pub setup_support: Vec<ProofSurface>,
    pub missing_direct: Vec<Surface>,
    #[serde(default)]
    pub wiring: Vec<ProofWiringFact>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChangedProofCommand {
    pub command: String,
    pub sensors: Vec<ProofSurface>,
    pub hidden_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChangedSymbol {
    pub path: String,
    pub name: String,
    pub change: String,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
}
