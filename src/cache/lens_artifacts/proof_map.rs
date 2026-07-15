use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model::{HiddenGroup, ProofMapReport, ProofSurface, ProofWiringFact, Surface, Unknown};

use super::{
    LensArtifact, current_status_fingerprint, format_version, read_lens_artifact,
    write_lens_artifact,
};

pub fn read_proof_map_report(
    cache_dir: &Path,
    version: &str,
    root: &Path,
    scope: Option<&str>,
    selector: &str,
    limit: usize,
    raw_sensors: bool,
) -> Option<ProofMapReport> {
    let artifact = proof_map_artifact_name(scope, selector, limit, raw_sensors);
    if let Some(cached) = read_matching_proof_map_artifact(
        cache_dir,
        &artifact,
        version,
        root,
        scope,
        selector,
        limit,
        raw_sensors,
    ) {
        return Some(cached.report.into_report());
    }
    read_matching_proof_map_artifact(
        cache_dir,
        "proof-map-current.json",
        version,
        root,
        scope,
        selector,
        limit,
        raw_sensors,
    )
    .map(|cached| cached.report.into_report())
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
        format_version: format_version(),
        version: version.to_string(),
        root: root.to_string_lossy().to_string(),
        fingerprint: current_status_fingerprint(cache_dir).unwrap_or_default(),
        scope: report.scope.clone(),
        selector: selector.to_string(),
        limit,
        raw_sensors,
        report: CachedProofMapReport::from_report(report),
    };
    let artifact = proof_map_artifact_name(report.scope.as_deref(), selector, limit, raw_sensors);
    write_lens_artifact(cache_dir, &artifact, &cached)?;
    write_lens_artifact(cache_dir, "proof-map-current.json", &cached)
}

#[allow(clippy::too_many_arguments)]
fn read_matching_proof_map_artifact(
    cache_dir: &Path,
    artifact: &str,
    version: &str,
    root: &Path,
    scope: Option<&str>,
    selector: &str,
    limit: usize,
    raw_sensors: bool,
) -> Option<CachedProofMapLens> {
    let cached: CachedProofMapLens = read_lens_artifact(cache_dir, artifact, version, root)?;
    if cached.scope.as_deref() != scope
        || cached.selector != selector
        || cached.limit != limit
        || cached.raw_sensors != raw_sensors
    {
        return None;
    }
    Some(cached)
}

fn proof_map_artifact_name(
    scope: Option<&str>,
    selector: &str,
    limit: usize,
    raw_sensors: bool,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(scope.unwrap_or("").as_bytes());
    hasher.update([0]);
    hasher.update(selector.as_bytes());
    hasher.update([0]);
    hasher.update(limit.to_string().as_bytes());
    hasher.update([0]);
    hasher.update(if raw_sensors {
        b"raw".as_slice()
    } else {
        b"compact".as_slice()
    });
    let digest = hasher.finalize();
    let suffix = digest
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0f])
        .take(16)
        .map(|n| char::from_digit(n as u32, 16).expect("hex digit"))
        .collect::<String>();
    format!("proof-map-{suffix}.json")
}

#[derive(Deserialize, Serialize)]
struct CachedProofMapLens {
    format_version: u64,
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
struct CachedProofMapReport {
    kind: String,
    schema_version: String,
    #[serde(default)]
    selector: String,
    scope: Option<String>,
    changed: Vec<String>,
    hard: Vec<ProofSurface>,
    direct_evidence: Vec<ProofSurface>,
    mediated_evidence: Vec<ProofSurface>,
    soft_evidence: Vec<ProofSurface>,
    setup_support: Vec<ProofSurface>,
    missing_direct: Vec<Surface>,
    commands: Vec<ProofSurface>,
    #[serde(default)]
    wiring: Vec<ProofWiringFact>,
    #[serde(default)]
    verification_topology: crate::model::VerificationTopology,
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
            selector: report.selector.clone(),
            scope: report.scope.clone(),
            changed: report.changed.clone(),
            hard: report.hard.clone(),
            direct_evidence: report.direct_evidence.clone(),
            mediated_evidence: report.mediated_evidence.clone(),
            soft_evidence: report.soft_evidence.clone(),
            setup_support: report.setup_support.clone(),
            missing_direct: report.missing_direct.clone(),
            commands: report.commands.clone(),
            wiring: report.wiring.clone(),
            verification_topology: report.verification_topology.clone(),
            fallback: report.fallback.clone(),
            unknowns: report.unknowns.clone(),
            hidden: report.hidden.clone(),
            expand: report.expand.clone(),
        }
    }

    fn into_report(self) -> ProofMapReport {
        ProofMapReport {
            kind: "proof_map_report",
            schema_version: "7",
            selector: if self.selector.is_empty() {
                self.scope
                    .clone()
                    .unwrap_or_else(|| "--changed".to_string())
            } else {
                self.selector
            },
            scope: self.scope,
            changed: self.changed,
            hard: self.hard,
            direct_evidence: self.direct_evidence,
            mediated_evidence: self.mediated_evidence,
            soft_evidence: self.soft_evidence,
            setup_support: self.setup_support,
            missing_direct: self.missing_direct,
            commands: self.commands,
            wiring: self.wiring,
            verification_topology: self.verification_topology,
            fallback: self.fallback,
            unknowns: self.unknowns,
            hidden: self.hidden,
            expand: self.expand,
        }
    }
}
