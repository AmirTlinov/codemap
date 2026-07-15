// Responsibility: verification-surface-report-types
use serde::{Deserialize, Serialize};

use super::{EvidenceLocation, EvidenceStrength, HiddenGroup, Unknown};

#[derive(Debug, Clone, Serialize)]
pub struct ProofReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub target: Option<String>,
    pub changed: Vec<String>,
    #[serde(skip_serializing)]
    pub selector: String,
    #[serde(skip_serializing)]
    pub risk: String,
    pub proofs: Vec<ProofSurface>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage: Option<ProofCoverageSummary>,
    pub wiring: Vec<ProofWiringFact>,
    pub fallback: Vec<String>,
    pub unknowns: Vec<Unknown>,
    pub hidden: Vec<HiddenGroup>,
    pub expand: Vec<String>,
    pub run_hint: String,
}

impl ProofReport {
    pub const SCHEMA_VERSION: &'static str = "10";
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProofSurface {
    pub command: Option<String>,
    pub path: Option<String>,
    pub target_anchor: Option<String>,
    pub evidence: String,
    pub strength: EvidenceStrength,
    pub reason: String,
    pub locations: Vec<EvidenceLocation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProofCoverageSummary {
    pub changed_count: usize,
    pub runnable_deterministic: Vec<ProofCoveredPath>,
    pub evidence_only: Vec<ProofCoveredPath>,
    pub setup_support_only: Vec<ProofCoveredPath>,
    pub soft_only: Vec<ProofCoveredPath>,
    pub missing: Vec<ProofGap>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProofCoveredPath {
    pub path: String,
    pub sensor_count: usize,
    pub evidence: Vec<String>,
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProofGap {
    pub path: String,
    pub kind: String,
    pub effect: String,
    pub expand: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProofWiringFact {
    pub stage: String,
    pub status: String,
    pub subject: String,
    pub path: Option<String>,
    pub evidence: String,
    pub effect: String,
    pub locations: Vec<EvidenceLocation>,
    pub expand: Option<String>,
}
