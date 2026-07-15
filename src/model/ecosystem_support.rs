// Responsibility: versioned-ecosystem-support-contract-types
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct EcosystemSupportCells {
    pub inventory: String,
    pub symbols: String,
    pub imports: String,
    pub packages: String,
    pub runtime: String,
    pub contracts: String,
    pub data: String,
    pub verification: String,
    pub dynamic_unknowns: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ReleaseEcosystemSupport {
    pub ecosystem: String,
    pub tier: String,
    pub cells: EcosystemSupportCells,
    pub promise: String,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectEcosystemSupport {
    #[serde(flatten)]
    pub declaration: ReleaseEcosystemSupport,
    pub detected_files: usize,
    pub generated_files: usize,
    pub examples: Vec<String>,
}
