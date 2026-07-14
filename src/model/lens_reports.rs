// Responsibility: focused-lens-report-types
use serde::{Deserialize, Serialize};

use super::Surface;

use super::{
    BoundaryFinding, DomainRef, EvidenceLocation, EvidenceStrength, FileSummary, HiddenGroup,
    PackageDependency, ProofSurface, ProofWiringFact, StructuralEdge, Unknown,
};

#[derive(Debug, Clone, Serialize)]
pub struct ContractReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub anchor: FileSummary,
    pub contract_kind: String,
    pub public_surface: bool,
    pub exported_contracts: Vec<Surface>,
    pub package_exports: Vec<StructuralEdge>,
    pub producers: Vec<StructuralEdge>,
    pub consumers: Vec<StructuralEdge>,
    pub cross_package_consumers: Vec<StructuralEdge>,
    pub proof: Vec<StructuralEdge>,
    pub unknowns: Vec<Unknown>,
    pub hidden: Vec<HiddenGroup>,
    pub expand: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub scope: String,
    pub entrypoints: Vec<Surface>,
    pub routes: Vec<RuntimeRoute>,
    pub scripts: Vec<Surface>,
    pub env: Vec<EnvSurface>,
    pub workers: Vec<Surface>,
    pub ci: Vec<Surface>,
    pub proof: Vec<StructuralEdge>,
    pub unknowns: Vec<Unknown>,
    pub hidden: Vec<HiddenGroup>,
    pub expand: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuntimeRoute {
    pub method: Option<String>,
    pub path: String,
    pub file: String,
    pub handler_symbol: Option<String>,
    pub evidence: String,
    pub strength: EvidenceStrength,
    pub locations: Vec<EvidenceLocation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnvSurface {
    pub name: String,
    pub used_by: String,
    pub declaration: Option<String>,
    pub evidence: String,
    pub strength: EvidenceStrength,
    pub locations: Vec<EvidenceLocation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProofMapReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub selector: String,
    pub scope: Option<String>,
    pub changed: Vec<String>,
    pub hard: Vec<ProofSurface>,
    pub direct_evidence: Vec<ProofSurface>,
    pub mediated_evidence: Vec<ProofSurface>,
    pub soft_evidence: Vec<ProofSurface>,
    pub setup_support: Vec<ProofSurface>,
    pub missing_direct: Vec<Surface>,
    pub commands: Vec<ProofSurface>,
    pub wiring: Vec<ProofWiringFact>,
    pub fallback: Vec<String>,
    pub unknowns: Vec<Unknown>,
    pub hidden: Vec<HiddenGroup>,
    pub expand: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeleteReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub anchor: FileSummary,
    pub direct_users: Vec<StructuralEdge>,
    pub symbol_users: Vec<StructuralEdge>,
    pub reexports: Vec<StructuralEdge>,
    pub package_exports: Vec<StructuralEdge>,
    pub tests: Vec<StructuralEdge>,
    pub runtime_refs: Vec<StructuralEdge>,
    pub unknowns: Vec<Unknown>,
    pub checklist: Vec<String>,
    pub hidden: Vec<HiddenGroup>,
    pub expand: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoundaryMapReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub scope: String,
    pub domains: Vec<DomainRef>,
    pub actual_cross_edges: Vec<StructuralEdge>,
    pub public_boundary_files: Vec<FileSummary>,
    pub test_only_crossings: Vec<StructuralEdge>,
    pub package_edges: Vec<PackageDependency>,
    pub explicit_forbidden_findings: Vec<BoundaryFinding>,
    pub unknowns: Vec<Unknown>,
    pub hidden: Vec<HiddenGroup>,
    pub expand: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FlowReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub anchor: String,
    pub flow_kind: String,
    pub precision: String,
    pub entry: Option<FileSummary>,
    pub steps: Vec<FlowStep>,
    pub side_effects: Vec<Surface>,
    pub contracts: Vec<StructuralEdge>,
    pub proof: Vec<StructuralEdge>,
    pub unknown_breaks: Vec<Unknown>,
    pub hidden: Vec<HiddenGroup>,
    pub expand: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FlowStep {
    pub index: usize,
    pub anchor: String,
    pub kind: String,
    pub evidence: String,
    pub locations: Vec<EvidenceLocation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SiblingsReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub scope: String,
    pub same_kind: Vec<Surface>,
    pub route_service_test_triplets: Vec<Surface>,
    pub shared_helpers: Vec<StructuralEdge>,
    pub shared_contracts: Vec<StructuralEdge>,
    pub proof_pattern: Vec<ProofSurface>,
    pub unknowns: Vec<Unknown>,
    pub hidden: Vec<HiddenGroup>,
    pub expand: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlaceReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub scope: String,
    pub requested_kind: String,
    pub existing_surfaces: Vec<Surface>,
    pub local_conventions: Vec<String>,
    pub paired_proof_pattern: Vec<ProofSurface>,
    pub shared_contracts: Vec<StructuralEdge>,
    pub unknowns: Vec<Unknown>,
    pub hidden: Vec<HiddenGroup>,
    pub expand: Vec<String>,
}
