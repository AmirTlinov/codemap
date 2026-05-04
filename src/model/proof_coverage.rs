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
