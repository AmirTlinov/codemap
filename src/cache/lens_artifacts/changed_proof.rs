use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::model::{
    BoundaryFacts, ChangedCouplingFact, ChangedMapDelta, ChangedProofSummary, ChangedReport,
    ChangedRisk, ChangedStructuralEvent, FileSummary, GitChange, HiddenGroup, ImpactCluster,
    ProofCoverageSummary, ProofReport, ProofSurface, ProofWiringFact, Unknown,
};

use super::{
    LensArtifact, current_status_fingerprint, format_version, read_lens_artifact,
    write_lens_artifact,
};

pub fn read_changed_report(
    cache_dir: &Path,
    version: &str,
    root: &Path,
    selector: &str,
    limit: usize,
) -> Option<ChangedReport> {
    let cached: CachedChangedLens =
        read_lens_artifact(cache_dir, "changed-current.json", version, root)?;
    if cached.selector != selector || cached.limit != limit {
        return None;
    }
    Some(cached.report.into_report(limit))
}

pub fn write_changed_report(
    cache_dir: &Path,
    version: &str,
    root: &Path,
    selector: &str,
    limit: usize,
    report: &ChangedReport,
) -> Result<()> {
    let cached = CachedChangedLens {
        format_version: format_version(),
        version: version.to_string(),
        root: root.to_string_lossy().to_string(),
        fingerprint: current_status_fingerprint(cache_dir).unwrap_or_default(),
        selector: selector.to_string(),
        limit,
        report: CachedChangedReport::from_report(report),
    };
    write_lens_artifact(cache_dir, "changed-current.json", &cached)
}

pub fn read_proof_changed_report(
    cache_dir: &Path,
    version: &str,
    root: &Path,
    selector: &str,
    depth: usize,
    limit: usize,
) -> Option<ProofReport> {
    let cached: CachedProofLens =
        read_lens_artifact(cache_dir, "proof-changed.json", version, root)?;
    if cached.selector != selector || cached.depth != depth || cached.limit != limit {
        return None;
    }
    Some(cached.report.into_report())
}

pub fn write_proof_changed_report(
    cache_dir: &Path,
    version: &str,
    root: &Path,
    selector: &str,
    depth: usize,
    limit: usize,
    report: &ProofReport,
) -> Result<()> {
    let cached = CachedProofLens {
        format_version: format_version(),
        version: version.to_string(),
        root: root.to_string_lossy().to_string(),
        fingerprint: current_status_fingerprint(cache_dir).unwrap_or_default(),
        selector: selector.to_string(),
        depth,
        limit,
        report: CachedProofReport::from_report(report),
    };
    write_lens_artifact(cache_dir, "proof-changed.json", &cached)
}

#[derive(Deserialize, Serialize)]
struct CachedChangedLens {
    format_version: u64,
    version: String,
    root: String,
    fingerprint: String,
    selector: String,
    limit: usize,
    report: CachedChangedReport,
}

impl LensArtifact for CachedChangedLens {
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
struct CachedProofLens {
    format_version: u64,
    version: String,
    root: String,
    fingerprint: String,
    selector: String,
    depth: usize,
    limit: usize,
    report: CachedProofReport,
}

impl LensArtifact for CachedProofLens {
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
struct CachedChangedReport {
    kind: String,
    schema_version: String,
    selector: String,
    total_changed_count: usize,
    changed: Vec<FileSummary>,
    git_state: Vec<GitChange>,
    structural_events: Vec<ChangedStructuralEvent>,
    map_delta: ChangedMapDelta,
    risks: Vec<ChangedRisk>,
    coupling: Vec<ChangedCouplingFact>,
    #[serde(default)]
    boundary_facts: BoundaryFacts,
    impact: Vec<ImpactCluster>,
    proof: ChangedProofSummary,
    unknowns: Vec<Unknown>,
    hidden: Vec<HiddenGroup>,
    expand: Vec<String>,
}

impl CachedChangedReport {
    fn from_report(report: &ChangedReport) -> Self {
        Self {
            kind: report.kind.to_string(),
            schema_version: report.schema_version.to_string(),
            selector: report.selector.clone(),
            total_changed_count: report.total_changed_count,
            changed: report.changed.clone(),
            git_state: report.git_state.clone(),
            structural_events: report.structural_events.clone(),
            map_delta: report.map_delta.clone(),
            risks: report.risks.clone(),
            coupling: report.coupling.clone(),
            boundary_facts: report.boundary_facts.clone(),
            impact: report.impact.clone(),
            proof: report.proof.clone(),
            unknowns: report.unknowns.clone(),
            hidden: report.hidden.clone(),
            expand: report.expand.clone(),
        }
    }

    fn into_report(self, limit: usize) -> ChangedReport {
        ChangedReport {
            kind: "changed_report",
            schema_version: "8",
            selector: self.selector,
            display_limit: limit,
            proof_plan_cache: None,
            proof_map_cache: None,
            total_changed_count: self.total_changed_count,
            changed: self.changed,
            git_state: self.git_state,
            structural_events: self.structural_events,
            map_delta: self.map_delta,
            risks: self.risks,
            coupling: self.coupling,
            boundary_facts: self.boundary_facts,
            impact: self.impact,
            proof: self.proof,
            unknowns: self.unknowns,
            hidden: self.hidden,
            expand: self.expand,
        }
    }
}

#[derive(Deserialize, Serialize)]
struct CachedProofReport {
    kind: String,
    schema_version: String,
    target: Option<String>,
    changed: Vec<String>,
    #[serde(default)]
    selector: String,
    risk: String,
    proofs: Vec<ProofSurface>,
    coverage: Option<ProofCoverageSummary>,
    #[serde(default)]
    wiring: Vec<ProofWiringFact>,
    fallback: Vec<String>,
    unknowns: Vec<Unknown>,
    hidden: Vec<HiddenGroup>,
    expand: Vec<String>,
    run_hint: String,
}

impl CachedProofReport {
    fn from_report(report: &ProofReport) -> Self {
        Self {
            kind: report.kind.to_string(),
            schema_version: report.schema_version.to_string(),
            target: report.target.clone(),
            changed: report.changed.clone(),
            selector: report.selector.clone(),
            risk: report.risk.clone(),
            proofs: report.proofs.clone(),
            coverage: report.coverage.clone(),
            wiring: report.wiring.clone(),
            fallback: report.fallback.clone(),
            unknowns: report.unknowns.clone(),
            hidden: report.hidden.clone(),
            expand: report.expand.clone(),
            run_hint: report.run_hint.clone(),
        }
    }

    fn into_report(self) -> ProofReport {
        ProofReport {
            kind: "proof_plan",
            schema_version: "9",
            target: self.target,
            changed: self.changed,
            selector: self.selector,
            risk: self.risk,
            proofs: self.proofs,
            coverage: self.coverage,
            wiring: self.wiring,
            fallback: self.fallback,
            unknowns: self.unknowns,
            hidden: self.hidden,
            expand: self.expand,
            run_hint: self.run_hint,
        }
    }
}
