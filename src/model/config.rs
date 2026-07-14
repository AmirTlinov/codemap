use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CodemapConfig {
    #[serde(default)]
    pub version: Option<u32>,
    #[serde(default)]
    pub domain: Option<AnchorDomain>,
    #[serde(default)]
    pub domains: BTreeMap<String, AnchorDomain>,
    #[serde(default)]
    pub owns: Vec<String>,
    #[serde(default)]
    pub does_not_own: Vec<String>,
    #[serde(default)]
    pub concepts: BTreeMap<String, AnchorConcept>,
    #[serde(default)]
    pub roles: BTreeMap<String, String>,
    #[serde(default)]
    pub boundaries: AnchorBoundaries,
    #[serde(default)]
    pub verification: AnchorVerification,
    #[serde(default)]
    pub proof: AnchorProof,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnchorDomain {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub purpose: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnchorConcept {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub invariants: Vec<String>,
    #[serde(default)]
    pub derives_from: Vec<String>,
    #[serde(default)]
    pub reads: Vec<String>,
    #[serde(default)]
    pub writes: Vec<String>,
    #[serde(default)]
    pub consumed_by: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnchorBoundaries {
    #[serde(default)]
    pub forbidden: Vec<BoundaryRule>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryRule {
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub to: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub recovery: Vec<String>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnchorVerification {
    #[serde(default)]
    pub default: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnchorProof {
    #[serde(default)]
    pub changed: Vec<String>,
}
