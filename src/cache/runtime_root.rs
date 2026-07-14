// Responsibility: cache-runtime-root
use std::fs;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::{cached_status_fingerprint, fingerprint};
use crate::model::{
    EnvSurface, HiddenGroup, Project, RuntimeReport, RuntimeRoute, StructuralEdge, Surface, Unknown,
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
    Some(cached.report.into_report())
}

pub(crate) fn write_runtime_root(project: &Project, version: &str) -> Result<()> {
    let report = crate::map::runtime_report(project, ".", false, 20);
    let cached = CachedRuntimeRoot {
        version: version.to_string(),
        root: project.root.to_string_lossy().to_string(),
        fingerprint: fingerprint(project, None),
        report: CachedRuntimeReport::from_report(report),
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
    report: CachedRuntimeReport,
}

#[derive(Deserialize, Serialize)]
struct CachedRuntimeReport {
    kind: String,
    schema_version: String,
    scope: String,
    entrypoints: Vec<Surface>,
    routes: Vec<RuntimeRoute>,
    scripts: Vec<Surface>,
    env: Vec<EnvSurface>,
    workers: Vec<Surface>,
    ci: Vec<Surface>,
    proof: Vec<StructuralEdge>,
    unknowns: Vec<Unknown>,
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
            scripts: report.scripts,
            env: report.env,
            workers: report.workers,
            ci: report.ci,
            proof: report.proof,
            unknowns: report.unknowns,
            hidden: report.hidden,
            expand: report.expand,
        }
    }

    fn into_report(self) -> RuntimeReport {
        RuntimeReport {
            kind: "runtime_report",
            schema_version: "2",
            scope: self.scope,
            entrypoints: self.entrypoints,
            routes: self.routes,
            scripts: self.scripts,
            env: self.env,
            workers: self.workers,
            ci: self.ci,
            proof: self.proof,
            unknowns: self.unknowns,
            hidden: self.hidden,
            expand: self.expand,
        }
    }
}
