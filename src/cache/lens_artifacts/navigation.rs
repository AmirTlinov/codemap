use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model::{
    BoundaryFacts, ConeReport, DirectorySurface, EnvDeclaration, FileSummary, HiddenGroup,
    LsReport, ObservationLedger, StructuralEdge, Unknown, XrayCard,
};

use super::{
    LensArtifact, current_status_fingerprint, format_version, read_lens_artifact,
    write_lens_artifact,
};

pub struct LsLensKey<'a> {
    pub cache_dir: &'a Path,
    pub version: &'a str,
    pub root: &'a Path,
    pub path: &'a str,
    pub include_hidden: bool,
    pub limit: usize,
}

pub struct ConeLensKey<'a> {
    pub cache_dir: &'a Path,
    pub version: &'a str,
    pub root: &'a Path,
    pub path: &'a str,
    pub depth: usize,
    pub include_hidden: bool,
    pub limit: usize,
}

pub fn read_ls_report(key: LsLensKey<'_>) -> Option<LsReport> {
    let cached: CachedLsLens =
        read_lens_artifact(key.cache_dir, "ls-current.json", key.version, key.root)?;
    if cached.path != key.path
        || cached.include_hidden != key.include_hidden
        || cached.limit != key.limit
    {
        return None;
    }
    if cached.report_sha256 != ls_report_sha256(&cached.report) {
        return None;
    }
    let report = cached.report.into_report();
    report.validate_observations().ok()?;
    Some(report)
}

pub fn write_ls_report(key: LsLensKey<'_>, report: &LsReport) -> Result<()> {
    let report = CachedLsReport::from_report(report);
    let cached = CachedLsLens {
        format_version: format_version(),
        version: key.version.to_string(),
        root: key.root.to_string_lossy().to_string(),
        fingerprint: current_status_fingerprint(key.cache_dir).unwrap_or_default(),
        path: key.path.to_string(),
        include_hidden: key.include_hidden,
        limit: key.limit,
        report_sha256: ls_report_sha256(&report),
        report,
    };
    write_lens_artifact(key.cache_dir, "ls-current.json", &cached)
}

pub fn read_cone_report(key: ConeLensKey<'_>) -> Option<ConeReport> {
    let cached: CachedConeLens =
        read_lens_artifact(key.cache_dir, "cone-current.json", key.version, key.root)?;
    if cached.path != key.path
        || cached.depth != key.depth
        || cached.include_hidden != key.include_hidden
        || cached.limit != key.limit
    {
        return None;
    }
    if cached.report_sha256 != cone_report_sha256(&cached.report) {
        return None;
    }
    let report = cached.report.into_report();
    report.validate_observations().ok()?;
    Some(report)
}

pub fn write_cone_report(key: ConeLensKey<'_>, report: &ConeReport) -> Result<()> {
    let report = CachedConeReport::from_report(report);
    let cached = CachedConeLens {
        format_version: format_version(),
        version: key.version.to_string(),
        root: key.root.to_string_lossy().to_string(),
        fingerprint: current_status_fingerprint(key.cache_dir).unwrap_or_default(),
        path: key.path.to_string(),
        depth: key.depth,
        include_hidden: key.include_hidden,
        limit: key.limit,
        report_sha256: cone_report_sha256(&report),
        report,
    };
    write_lens_artifact(key.cache_dir, "cone-current.json", &cached)
}

#[derive(Deserialize, Serialize)]
struct CachedLsLens {
    format_version: u64,
    version: String,
    root: String,
    fingerprint: String,
    path: String,
    include_hidden: bool,
    limit: usize,
    report_sha256: String,
    report: CachedLsReport,
}

impl LensArtifact for CachedLsLens {
    fn format_version(&self) -> u64 {
        self.format_version
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn root(&self) -> &str {
        &self.root
    }

    fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
}

#[derive(Deserialize, Serialize)]
struct CachedConeLens {
    format_version: u64,
    version: String,
    root: String,
    fingerprint: String,
    path: String,
    depth: usize,
    include_hidden: bool,
    limit: usize,
    report_sha256: String,
    report: CachedConeReport,
}

impl LensArtifact for CachedConeLens {
    fn format_version(&self) -> u64 {
        self.format_version
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn root(&self) -> &str {
        &self.root
    }

    fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
}

#[derive(Deserialize, Serialize)]
struct CachedLsReport {
    kind: String,
    schema_version: String,
    path: String,
    mode: String,
    anchor: Option<FileSummary>,
    directory: Vec<DirectorySurface>,
    #[serde(default)]
    boundary_facts: BoundaryFacts,
    edges: Vec<StructuralEdge>,
    #[serde(default)]
    observations: ObservationLedger,
    hidden: Vec<HiddenGroup>,
    next: Vec<String>,
}

impl CachedLsReport {
    fn from_report(report: &LsReport) -> Self {
        Self {
            kind: report.kind.to_string(),
            schema_version: report.schema_version.to_string(),
            path: report.path.clone(),
            mode: report.mode.clone(),
            anchor: report.anchor.clone(),
            directory: report.directory.clone(),
            boundary_facts: report.boundary_facts.clone(),
            edges: report.edges.clone(),
            observations: report.observations.clone(),
            hidden: report.hidden.clone(),
            next: report.next.clone(),
        }
    }

    fn into_report(self) -> LsReport {
        LsReport {
            kind: "ls_report",
            schema_version: crate::model::LsReport::SCHEMA_VERSION,
            path: self.path,
            mode: self.mode,
            anchor: self.anchor,
            directory: self.directory,
            boundary_facts: self.boundary_facts,
            edges: self.edges,
            observations: self.observations,
            hidden: self.hidden,
            next: self.next,
        }
    }
}

#[derive(Deserialize, Serialize)]
struct CachedConeReport {
    kind: String,
    schema_version: String,
    anchor: FileSummary,
    depth: usize,
    #[serde(default)]
    xray: XrayCard,
    #[serde(default)]
    declared_env: Vec<EnvDeclaration>,
    outgoing: Vec<StructuralEdge>,
    incoming: Vec<StructuralEdge>,
    proof: Vec<StructuralEdge>,
    contracts: Vec<StructuralEdge>,
    boundary: Vec<StructuralEdge>,
    observations: ObservationLedger,
    hidden: Vec<HiddenGroup>,
    unknowns: Vec<Unknown>,
    expand: Vec<String>,
}

impl CachedConeReport {
    fn from_report(report: &ConeReport) -> Self {
        Self {
            kind: report.kind.to_string(),
            schema_version: report.schema_version.to_string(),
            anchor: report.anchor.clone(),
            depth: report.depth,
            xray: report.xray.clone(),
            declared_env: report.declared_env.clone(),
            outgoing: report.outgoing.clone(),
            incoming: report.incoming.clone(),
            proof: report.proof.clone(),
            contracts: report.contracts.clone(),
            boundary: report.boundary.clone(),
            observations: report.observations.clone(),
            hidden: report.hidden.clone(),
            unknowns: report.unknowns.clone(),
            expand: report.expand.clone(),
        }
    }

    fn into_report(self) -> ConeReport {
        ConeReport {
            kind: "cone_report",
            schema_version: crate::model::ConeReport::SCHEMA_VERSION,
            anchor: self.anchor,
            depth: self.depth,
            xray: self.xray,
            declared_env: self.declared_env,
            outgoing: self.outgoing,
            incoming: self.incoming,
            proof: self.proof,
            contracts: self.contracts,
            boundary: self.boundary,
            observations: self.observations,
            hidden: self.hidden,
            unknowns: self.unknowns,
            expand: self.expand,
        }
    }
}

fn cone_report_sha256(report: &CachedConeReport) -> String {
    let body = serde_json::to_vec(report).expect("cached cone report should serialize");
    format!("{:x}", Sha256::digest(body))
}

fn ls_report_sha256(report: &CachedLsReport) -> String {
    let body = serde_json::to_vec(report).expect("cached ls report should serialize");
    format!("{:x}", Sha256::digest(body))
}
