use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::model::{HiddenGroup, ProofMapReport, ProofSurface, Surface, Unknown};

use super::{LensArtifact, current_status_fingerprint, read_lens_artifact, write_lens_artifact};

pub fn read_proof_map_report(
    cache_dir: &Path,
    version: &str,
    root: &Path,
    scope: Option<&str>,
    selector: &str,
    limit: usize,
    raw_sensors: bool,
) -> Option<ProofMapReport> {
    let cached: CachedProofMapLens =
        read_lens_artifact(cache_dir, "proof-map-current.json", version, root)?;
    if cached.scope.as_deref() != scope
        || cached.selector != selector
        || cached.limit != limit
        || cached.raw_sensors != raw_sensors
    {
        return None;
    }
    Some(cached.report.into_report())
}

pub fn write_proof_map_report(
    cache_dir: &Path,
    version: &str,
    root: &Path,
    selector: &str,
    limit: usize,
    raw_sensors: bool,
    report: &ProofMapReport,
) -> Result<()> {
    let cached = CachedProofMapLens {
        version: version.to_string(),
        root: root.to_string_lossy().to_string(),
        fingerprint: current_status_fingerprint(cache_dir).unwrap_or_default(),
        scope: report.scope.clone(),
        selector: selector.to_string(),
        limit,
        raw_sensors,
        report: CachedProofMapReport::from_report(report),
    };
    write_lens_artifact(cache_dir, "proof-map-current.json", &cached)
}

#[derive(Deserialize, Serialize)]
struct CachedProofMapLens {
    version: String,
    root: String,
    fingerprint: String,
    scope: Option<String>,
    selector: String,
    limit: usize,
    raw_sensors: bool,
    report: CachedProofMapReport,
}

impl LensArtifact for CachedProofMapLens {
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
struct CachedProofMapReport {
    kind: String,
    schema_version: String,
    scope: Option<String>,
    changed: Vec<String>,
    direct: Vec<ProofSurface>,
    indirect: Vec<ProofSurface>,
    e2e: Vec<ProofSurface>,
    contract: Vec<ProofSurface>,
    missing_direct: Vec<Surface>,
    commands: Vec<ProofSurface>,
    fallback: Vec<String>,
    unknowns: Vec<Unknown>,
    hidden: Vec<HiddenGroup>,
    expand: Vec<String>,
}

impl CachedProofMapReport {
    fn from_report(report: &ProofMapReport) -> Self {
        Self {
            kind: report.kind.to_string(),
            schema_version: report.schema_version.to_string(),
            scope: report.scope.clone(),
            changed: report.changed.clone(),
            direct: report.direct.clone(),
            indirect: report.indirect.clone(),
            e2e: report.e2e.clone(),
            contract: report.contract.clone(),
            missing_direct: report.missing_direct.clone(),
            commands: report.commands.clone(),
            fallback: report.fallback.clone(),
            unknowns: report.unknowns.clone(),
            hidden: report.hidden.clone(),
            expand: report.expand.clone(),
        }
    }

    fn into_report(self) -> ProofMapReport {
        ProofMapReport {
            kind: "proof_map_report",
            schema_version: "2",
            scope: self.scope,
            changed: self.changed,
            direct: self.direct,
            indirect: self.indirect,
            e2e: self.e2e,
            contract: self.contract,
            missing_direct: self.missing_direct,
            commands: self.commands,
            fallback: self.fallback,
            unknowns: self.unknowns,
            hidden: self.hidden,
            expand: self.expand,
        }
    }
}
