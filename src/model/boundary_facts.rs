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
