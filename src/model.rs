// Responsibility: project-inventory-and-fact-primitives
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub type ImportBindingMap = BTreeMap<String, String>;
pub type ImportBindingsBySpec = BTreeMap<String, ImportBindingMap>;

mod boundary;
mod changed_reports;
mod cone_reports;
mod config;
mod coverage;
mod coverage_ledger;
#[cfg(test)]
mod coverage_tests;
mod lens_reports;
mod prelude;
mod proof_reports;
mod structure_reports;
mod teach_reports;

pub use boundary::*;
pub use changed_reports::*;
pub use cone_reports::*;
pub use config::*;
pub use coverage::*;
pub use coverage_ledger::*;
pub use lens_reports::*;
pub use prelude::*;
pub use proof_reports::*;
pub use structure_reports::*;
pub use teach_reports::*;

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
    pub anchors: CodemapConfig,
    pub cache_state: String,
    pub cache_artifacts: Vec<CacheArtifactStatus>,
    pub cache_strategy: String,
    pub files_reused: usize,
    pub scan_stats: ScanStats,
    pub timings: ProjectTimings,
}

impl Project {
    /// Reads repository text only when the indexed file was actually scanned.
    ///
    /// `content_hash == None` is the shared unread-body boundary for symlinks,
    /// gitlinks, unavailable tracked files, oversized files, and unsupported
    /// parsers. Downstream fact builders must not turn those paths into body
    /// facts.
    pub(crate) fn read_indexed_text(&self, rel: &str) -> Option<String> {
        self.files.get(rel)?.content_hash.as_ref()?;
        let path = self.root.join(rel);
        let metadata = std::fs::symlink_metadata(&path).ok()?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return None;
        }
        std::fs::read_to_string(path).ok()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigLoadError {
    pub path: String,
    pub error: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileInfo {
    pub rel: String,
    pub ext: String,
    pub size: u64,
    /// A stable index-owned boundary which cannot be derived from the current
    /// worktree node alone. Public reports expose its consequences through
    /// coverage certificates rather than leaking scanner bookkeeping.
    #[serde(default, skip_serializing)]
    pub indexed_boundary: Option<IndexedBoundary>,
    #[serde(default, skip_serializing)]
    pub content_hash: Option<String>,
    pub line_count: usize,
    pub language: String,
    pub roles: BTreeSet<String>,
    pub imports: BTreeSet<String>,
    #[serde(default)]
    pub has_dynamic_import: bool,
    pub import_bindings: ImportBindingsBySpec,
    pub resolved_imports: BTreeSet<String>,
    pub unresolved_imports: BTreeSet<String>,
    pub resolved_import_bindings: ImportBindingsBySpec,
    pub exports: BTreeSet<String>,
    pub symbols: Vec<SymbolInfo>,
    pub tokens: BTreeSet<String>,
    pub references: BTreeSet<String>,
    pub jsx_tags: BTreeSet<String>,
    pub local_bindings: BTreeSet<String>,
    pub surface_tokens: BTreeSet<String>,
    pub surface_phrases: BTreeSet<String>,
    pub visited_route_paths: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum IndexedBoundary {
    ExternalTree,
    ExternalGitlink,
    IgnoredTrackedFile,
    TraversalError,
    UnavailableTrackedFile,
}

impl FileInfo {
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(role)
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ScanStats {
    pub files_visited: usize,
    pub files_scanned: usize,
    pub files_skipped: usize,
    pub bytes_scanned: u64,
    pub ignored: Vec<ScanGroup>,
    pub generated: Vec<ScanGroup>,
    /// Internal completeness state persisted by the cache owner separately;
    /// it is deliberately absent from stable public status/doctor schemas.
    #[serde(default, skip_serializing)]
    pub inventory_boundaries: Vec<ScanInventoryBoundary>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ScanInventoryBoundary {
    FilesystemTraversalUnavailable,
    GitIndexUnavailable,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScanGroup {
    pub reason: String,
    pub count: usize,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ProjectTimings {
    pub root_ms: u128,
    pub scan_ms: u128,
    pub facts_ms: u128,
    pub cache_artifact_ms: u128,
    pub cache_write_ms: u128,
    pub total_ms: u128,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: String,
    pub exported: bool,
    pub line_start: usize,
    pub line_end: usize,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStrength {
    Low,
    Medium,
    High,
    Hard,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EvidenceLocation {
    pub path: String,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
    pub kind: String,
}

impl EvidenceLocation {
    pub fn aggregate(kind: impl Into<String>) -> Self {
        Self {
            path: "aggregate".to_string(),
            line_start: None,
            line_end: None,
            kind: kind.into(),
        }
    }

    pub fn path(path: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            line_start: None,
            line_end: None,
            kind: kind.into(),
        }
    }

    pub fn line(path: impl Into<String>, line: usize, kind: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            line_start: Some(line),
            line_end: Some(line),
            kind: kind.into(),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StructuralEdge {
    pub from: String,
    pub to: String,
    #[serde(rename = "type")]
    pub edge_type: String,
    pub evidence: String,
    pub strength: EvidenceStrength,
    pub locations: Vec<EvidenceLocation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnvDeclaration {
    pub key: String,
    pub path: String,
    pub line_start: usize,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HiddenGroup {
    pub reason: String,
    pub count: usize,
    pub expand: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Unknown {
    pub kind: String,
    pub path: Option<String>,
    pub line_start: Option<usize>,
    pub reason: String,
    pub effect: String,
    pub expand: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileSummary {
    pub path: String,
    pub kind: String,
    pub package: Option<String>,
    pub language: String,
    pub lines: usize,
    pub roles: Vec<String>,
    pub symbols: Vec<SymbolInfo>,
    pub exports: Vec<String>,
    pub imports: Vec<String>,
    pub imported_by: CountFact,
}

#[derive(Debug, Clone, Serialize)]
pub struct Domain {
    pub id: String,
    pub path: String,
    pub config_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScriptInfo {
    pub name: String,
    pub command: String,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_start: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackageInfo {
    pub name: String,
    pub path: String,
    pub manifest: String,
    pub ecosystem: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackageDependency {
    pub from: String,
    pub from_manifest: String,
    pub to: String,
    pub to_manifest: Option<String>,
    pub workspace_manifest: Option<String>,
    pub dependency: String,
    pub dependency_kind: String,
    pub source: String,
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
pub struct VerificationPlan {
    pub minimal: Vec<String>,
    pub supplemental: Vec<String>,
    pub full_only_if_triggered: Vec<String>,
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
pub struct CacheArtifactStatus {
    pub name: String,
    pub path: String,
    pub exists: bool,
    pub bytes: Option<u64>,
    pub fingerprint_match: Option<bool>,
}
