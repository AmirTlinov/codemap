use serde::{Deserialize, Serialize};

use super::EvidenceLocation;

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
