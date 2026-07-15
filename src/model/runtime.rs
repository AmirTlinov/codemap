// Responsibility: runtime-map-entity-types
use serde::{Deserialize, Serialize};

use super::{EvidenceLocation, EvidenceStrength};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuntimeRoute {
    pub method: Option<String>,
    pub path: String,
    pub file: String,
    pub handler_symbol: Option<String>,
    #[serde(default)]
    pub middleware_or_guards: Vec<MiddlewareOrGuard>,
    pub evidence: String,
    pub strength: EvidenceStrength,
    pub locations: Vec<EvidenceLocation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MiddlewareOrGuard {
    pub name: String,
    pub kind: MiddlewareOrGuardKind,
    pub owner: String,
    pub evidence: String,
    pub strength: EvidenceStrength,
    pub locations: Vec<EvidenceLocation>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum MiddlewareOrGuardKind {
    Middleware,
    Guard,
    Validation,
}
