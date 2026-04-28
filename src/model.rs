use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct Project {
    pub root: PathBuf,
    pub cwd: PathBuf,
    pub vcs: Option<String>,
    pub cache_dir: PathBuf,
    pub config_path: Option<String>,
    pub config_errors: Vec<ConfigLoadError>,
    pub nearest_agents: Option<String>,
    pub files: BTreeMap<String, FileInfo>,
    pub reverse_imports: BTreeMap<String, BTreeSet<String>>,
    pub packages: Vec<PackageInfo>,
    pub package_edges: Vec<PackageDependency>,
    pub domains: Vec<Domain>,
    pub package_manager: String,
    pub scripts: Vec<ScriptInfo>,
    pub languages: BTreeSet<String>,
    pub anchors: CtxConfig,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigLoadError {
    pub path: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileInfo {
    pub rel: String,
    pub ext: String,
    pub size: u64,
    pub language: String,
    pub roles: BTreeSet<String>,
    pub imports: BTreeSet<String>,
    pub resolved_imports: BTreeSet<String>,
    pub exports: BTreeSet<String>,
    pub tokens: BTreeSet<String>,
}

impl FileInfo {
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(role)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Domain {
    pub id: String,
    pub path: String,
    pub config_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScriptInfo {
    pub name: String,
    pub command: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackageInfo {
    pub name: String,
    pub path: String,
    pub manifest: String,
    pub ecosystem: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackageDependency {
    pub from: String,
    pub from_manifest: String,
    pub to: String,
    pub to_manifest: Option<String>,
    pub dependency: String,
    pub source: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CtxConfig {
    #[serde(default)]
    pub version: Option<u32>,
    #[serde(default)]
    pub domain: Option<AnchorDomain>,
    #[serde(default)]
    pub domains: BTreeMap<String, AnchorDomain>,
    #[serde(default)]
    pub owns: Vec<String>,
    #[serde(default)]
    pub does_not_own: Vec<String>,
    #[serde(default)]
    pub concepts: BTreeMap<String, AnchorConcept>,
    #[serde(default)]
    pub boundaries: AnchorBoundaries,
    #[serde(default)]
    pub task_routes: BTreeMap<String, AnchorTaskRoute>,
    #[serde(default)]
    pub verification: AnchorVerification,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AnchorDomain {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub purpose: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AnchorConcept {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub invariants: Vec<String>,
    #[serde(default)]
    pub derives_from: Vec<String>,
    #[serde(default)]
    pub reads: Vec<String>,
    #[serde(default)]
    pub writes: Vec<String>,
    #[serde(default)]
    pub consumed_by: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AnchorBoundaries {
    #[serde(default)]
    pub forbidden: Vec<BoundaryRule>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct BoundaryRule {
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub to: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub recovery: Vec<String>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AnchorTaskRoute {
    #[serde(default, rename = "match")]
    pub matches: Vec<String>,
    #[serde(default)]
    pub read_first: Vec<String>,
    #[serde(default)]
    pub verify: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AnchorVerification {
    #[serde(default)]
    pub default: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Low,
    Medium,
    High,
    Hard,
}

impl Confidence {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Hard => "hard",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Risk {
    Low,
    Medium,
    MediumHigh,
    High,
    Critical,
}

impl Risk {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::MediumHigh => "medium-high",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Candidate {
    pub path: String,
    pub score: f64,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoNotRead {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerificationPlan {
    pub minimal: Vec<String>,
    pub recommended: Vec<String>,
    pub full_only_if_triggered: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskCapsule {
    pub kind: &'static str,
    pub task: String,
    pub domain: DomainRef,
    pub task_kind: String,
    pub confidence: String,
    pub risk: String,
    pub read_first: Vec<Candidate>,
    pub related_tests: Vec<String>,
    pub source_of_truth: Vec<String>,
    pub public_boundaries: Vec<String>,
    pub do_not_read_yet: Vec<DoNotRead>,
    pub forbidden_moves: Vec<String>,
    pub invariants: Vec<String>,
    pub verification: VerificationPlan,
    pub expansion_triggers: Vec<String>,
    pub stop_conditions: Vec<String>,
    pub provenance: BTreeMap<String, String>,
    pub cache: CacheInfo,
}

#[derive(Debug, Clone, Serialize)]
pub struct DomainRef {
    pub id: String,
    pub path: String,
}

impl From<&Domain> for DomainRef {
    fn from(value: &Domain) -> Self {
        Self {
            id: value.id.clone(),
            path: value.path.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CacheInfo {
    pub path: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocateReport {
    pub kind: &'static str,
    pub task: String,
    pub candidates: Vec<LocateCandidate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocateCandidate {
    pub domain: DomainRef,
    pub score: f64,
    pub task_kind: String,
    pub confidence: String,
    pub reasons: Vec<String>,
    pub start_command: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImpactReport {
    pub changed: Vec<String>,
    pub risk: String,
    pub files: Vec<FileRisk>,
    pub impacted: Vec<String>,
    pub related_tests: Vec<String>,
    pub domains: Vec<DomainRef>,
    pub external_domains: Vec<DomainRef>,
    pub minimal_verification: Vec<String>,
    pub recommended_verification: Vec<String>,
    pub full_verification: Vec<String>,
    pub expansion_triggers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileRisk {
    pub path: String,
    pub risk: String,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExplainReport {
    pub kind: String,
    pub path: Option<String>,
    pub id: Option<String>,
    pub domain: Option<DomainRef>,
    pub roles: Vec<String>,
    pub risk: Option<String>,
    pub risk_reasons: Vec<String>,
    pub imports: Vec<String>,
    pub imported_by: Vec<String>,
    pub exports: Vec<String>,
    pub related_tests: Vec<String>,
    pub invariants: Vec<String>,
    pub files: Vec<String>,
    pub provenance: String,
    pub confidence: String,
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WidenReport {
    pub kind: &'static str,
    pub reason: String,
    pub domain: DomainRef,
    pub add: Vec<String>,
    pub still_do_not_read_yet: Vec<DoNotRead>,
    pub confidence: String,
    pub stop_rule: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoundaryFinding {
    pub from: String,
    pub to: String,
    pub status: String,
    pub reason: String,
    pub recovery: Vec<String>,
    pub provenance: String,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphLens {
    pub kind: &'static str,
    pub domain: DomainRef,
    pub lens: String,
    pub nodes: Vec<String>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    #[serde(rename = "type")]
    pub edge_type: String,
}
