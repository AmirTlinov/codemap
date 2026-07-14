// Responsibility: boundary-fact-and-report-types
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct BoundaryFacts {
    pub instruction_files: Vec<BoundaryFact>,
    pub protected_looking_paths: Vec<BoundaryFact>,
    pub repo_local_guard_files: Vec<BoundaryFact>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BoundaryFact {
    pub path: String,
    pub evidence: String,
    pub effect: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct BoundaryFinding {
    pub from: String,
    pub to: String,
    pub status: String,
    pub reason: String,
    pub recovery: Vec<String>,
    pub provenance: String,
    pub strength: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoundaryReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub findings: Vec<BoundaryFinding>,
}
