// Responsibility: cache-runtime-root
use std::fs;
use std::path::Path;

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{cache_enabled, cached_status_fingerprint, fingerprint};
use crate::model::{
    EnvSurface, HiddenGroup, ObservationLedger, Project, RuntimeReport, RuntimeRoute,
    StructuralEdge, Surface, Unknown,
};

pub fn read_runtime_root_report(
    cache_dir: &Path,
    version: &str,
    root: &Path,
) -> Option<RuntimeReport> {
    let text = fs::read_to_string(cache_dir.join("runtime-root.json")).ok()?;
    let cached: CachedRuntimeRoot = serde_json::from_str(&text).ok()?;
    if cached.version != version {
        return None;
    }
    if cached.root != root.to_string_lossy() {
        return None;
    }
    if cached.fingerprint != cached_status_fingerprint(cache_dir)? {
        return None;
    }
    if !cached.projection.is_bounded_root()
        || !cached
            .projection
            .matches_recursive_hidden_boundary(&cached.report.hidden)
    {
        return None;
    }
    if cached.report_sha256 != cached_runtime_report_sha256(&cached.report)? {
        return None;
    }
    if cached.report.kind != "runtime_report"
        || cached.report.schema_version != RuntimeReport::SCHEMA_VERSION
        || cached.report.scope != "."
    {
        return None;
    }
    let observation_snapshot = super::identity::runtime_scope_fingerprint_from_project_snapshot(
        root,
        ".",
        &cached.fingerprint,
    );
    if cached
        .report
        .observations
        .certificates
        .values()
        .any(|certificate| certificate.snapshot != observation_snapshot)
    {
        return None;
    }
    let report = cached.report.into_report();
    report.validate_current_level_root_projection(20).ok()?;
    Some(report)
}

pub(crate) fn write_runtime_root(project: &Project, version: &str) -> Result<()> {
    if !cache_enabled() {
        return Ok(());
    }
    fs::create_dir_all(&project.cache_dir)?;
    let report = crate::map::runtime_report(project, ".", false, 20);
    report
        .validate_current_level_root_projection(20)
        .map_err(|error| anyhow!("invalid runtime observation ledger: {error:?}"))?;
    let recursive_hidden_count = canonical_recursive_hidden_count(&report.hidden)
        .ok_or_else(|| anyhow!("invalid runtime recursive hidden boundary"))?;
    let report = CachedRuntimeReport::from_report(report);
    let cached = CachedRuntimeRoot {
        version: version.to_string(),
        root: project.root.to_string_lossy().to_string(),
        fingerprint: fingerprint(project, None),
        projection: CachedRuntimeProjection::bounded_root(recursive_hidden_count),
        report_sha256: cached_runtime_report_sha256(&report)
            .ok_or_else(|| anyhow!("runtime root report should serialize"))?,
        report,
    };
    let body = serde_json::to_string_pretty(&cached)?;
    fs::write(
        project.cache_dir.join("runtime-root.json"),
        format!("{body}\n"),
    )?;
    Ok(())
}

#[derive(Deserialize, Serialize)]
struct CachedRuntimeRoot {
    version: String,
    root: String,
    fingerprint: String,
    projection: CachedRuntimeProjection,
    report_sha256: String,
    report: CachedRuntimeReport,
}

#[derive(Deserialize, PartialEq, Eq, Serialize)]
struct CachedRuntimeProjection {
    kind: String,
    limit: usize,
    current_level_root: bool,
    recursive_hidden_count: usize,
}

impl CachedRuntimeProjection {
    fn bounded_root(recursive_hidden_count: usize) -> Self {
        Self {
            kind: "bounded".to_string(),
            limit: 20,
            current_level_root: true,
            recursive_hidden_count,
        }
    }

    fn is_bounded_root(&self) -> bool {
        self.kind == "bounded" && self.limit == 20 && self.current_level_root
    }

    fn matches_recursive_hidden_boundary(&self, hidden: &[HiddenGroup]) -> bool {
        canonical_recursive_hidden_count(hidden) == Some(self.recursive_hidden_count)
    }
}

fn canonical_recursive_hidden_count(hidden: &[HiddenGroup]) -> Option<usize> {
    match hidden {
        [] => Some(0),
        [group]
            if group.count > 0
                && group.reason == RuntimeReport::ROOT_RECURSIVE_HIDDEN_REASON
                && group.expand == RuntimeReport::ROOT_RECURSIVE_HIDDEN_EXPAND =>
        {
            Some(group.count)
        }
        _ => None,
    }
}

#[derive(Deserialize, Serialize)]
struct CachedRuntimeReport {
    kind: String,
    schema_version: String,
    scope: String,
    entrypoints: Vec<Surface>,
    routes: Vec<RuntimeRoute>,
    paths: Vec<StructuralEdge>,
    scripts: Vec<Surface>,
    env: Vec<EnvSurface>,
    workers: Vec<Surface>,
    ci: Vec<Surface>,
    proof: Vec<StructuralEdge>,
    unknowns: Vec<Unknown>,
    observations: ObservationLedger,
    hidden: Vec<HiddenGroup>,
    expand: Vec<String>,
}

impl CachedRuntimeReport {
    fn from_report(report: RuntimeReport) -> Self {
        Self {
            kind: report.kind.to_string(),
            schema_version: report.schema_version.to_string(),
            scope: report.scope,
            entrypoints: report.entrypoints,
            routes: report.routes,
            paths: report.paths,
            scripts: report.scripts,
            env: report.env,
            workers: report.workers,
            ci: report.ci,
            proof: report.proof,
            unknowns: report.unknowns,
            observations: report.observations,
            hidden: report.hidden,
            expand: report.expand,
        }
    }

    fn into_report(self) -> RuntimeReport {
        RuntimeReport {
            kind: "runtime_report",
            schema_version: RuntimeReport::SCHEMA_VERSION,
            scope: self.scope,
            entrypoints: self.entrypoints,
            routes: self.routes,
            paths: self.paths,
            scripts: self.scripts,
            env: self.env,
            workers: self.workers,
            ci: self.ci,
            proof: self.proof,
            unknowns: self.unknowns,
            observations: self.observations,
            hidden: self.hidden,
            expand: self.expand,
        }
    }
}

fn cached_runtime_report_sha256(report: &CachedRuntimeReport) -> Option<String> {
    let canonical = serde_json::to_value(report).ok()?;
    let body = serde_json::to_vec(&canonical).ok()?;
    Some(format!("{:x}", Sha256::digest(body)))
}
