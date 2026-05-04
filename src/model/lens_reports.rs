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
pub struct ImpactReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub changed: Vec<FileSummary>,
    pub clusters: Vec<ImpactCluster>,
    pub hidden: Vec<HiddenGroup>,
    pub unknowns: Vec<Unknown>,
    pub expand: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImpactCluster {
    pub id: String,
    #[serde(default = "default_low_risk", skip_serializing)]
    pub risk: String,
    pub changed: Vec<String>,
    pub direct_consumers: Vec<StructuralEdge>,
    pub cross_boundary_consumers: Vec<StructuralEdge>,
    #[serde(rename = "contract_links", alias = "contract_risks")]
    pub contract_links: Vec<StructuralEdge>,
    pub proof: Vec<StructuralEdge>,
    pub reasons: Vec<String>,
}

fn default_low_risk() -> String {
    "low".to_string()
}

#[derive(Debug, Clone, Serialize)]
pub struct DiffMapReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
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
    #[serde(skip)]
    pub display_limit: usize,
    #[serde(skip)]
    pub proof_plan_cache: Option<Box<ProofReport>>,
    pub total_changed_count: usize,
    pub changed: Vec<FileSummary>,
    pub git_state: Vec<GitChange>,
    pub structural_events: Vec<ChangedStructuralEvent>,
    pub map_delta: ChangedMapDelta,
    pub risks: Vec<ChangedRisk>,
    pub coupling: Vec<ChangedCouplingFact>,
    pub impact: Vec<ImpactCluster>,
    pub proof: ChangedProofSummary,
    pub unknowns: Vec<Unknown>,
    pub hidden: Vec<HiddenGroup>,
    pub expand: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GitChange {
    pub path: String,
    pub old_path: Option<String>,
    pub status: String,
    pub staged: bool,
    pub unstaged: bool,
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
    pub scope: Option<String>,
    pub changed: Vec<String>,
    pub hard: Vec<ProofSurface>,
    pub direct_evidence: Vec<ProofSurface>,
    pub mediated_evidence: Vec<ProofSurface>,
    pub soft_evidence: Vec<ProofSurface>,
    pub setup_support: Vec<ProofSurface>,
    pub missing_direct: Vec<Surface>,
    pub commands: Vec<ProofSurface>,
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
