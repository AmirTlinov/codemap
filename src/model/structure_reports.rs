// Responsibility: structural-listing-and-graph-report-types
use serde::{Deserialize, Serialize};

use super::{
    BoundaryFacts, DomainRef, EvidenceLocation, EvidenceStrength, FileSummary, HiddenGroup,
    ObservationLedger, ObservationLedgerError, StructuralEdge,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LsReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub path: String,
    pub mode: String,
    pub anchor: Option<FileSummary>,
    pub directory: Vec<DirectorySurface>,
    pub boundary_facts: BoundaryFacts,
    pub edges: Vec<StructuralEdge>,
    #[serde(default)]
    pub observations: ObservationLedger,
    pub hidden: Vec<HiddenGroup>,
    pub next: Vec<String>,
}

impl LsReport {
    pub const SCHEMA_VERSION: &'static str = "10";

    /// Root inventory groups certified by the S03.d observation ledger.
    pub const ROOT_INVENTORY_GROUPS: [&'static str; 4] =
        ["directory_surfaces", "packages", "scripts", "test_surfaces"];

    /// Exact-symbol `ls` groups certified by the S03.e observation ledger.
    pub const SYMBOL_GROUPS: [&'static str; 2] = ["consumers", "verification"];

    /// Exact-file relationship groups certified by the S03.f ledger.
    pub const FILE_RELATIONSHIP_GROUPS: [&'static str; 3] =
        ["imports", "consumers", "verification"];

    /// Legacy detached hidden reasons whose truth now lives in the horizons.
    const LEGACY_ROOT_HIDDEN_REASONS: [&'static str; 2] = [
        "directory surfaces hidden by limit",
        "support packages hidden below support scopes",
    ];

    /// Surface kinds counted by the `test_surfaces` horizon at the root level.
    pub const TEST_SURFACE_KINDS: [&'static str; 3] = ["test", "e2e_test", "test_support"];

    pub fn validate_observations(&self) -> Result<(), ObservationLedgerError> {
        self.observations.validate()?;
        if self.is_exact_symbol_scope() {
            return self.validate_symbol_observations();
        }
        if self.mode == "file" {
            return self.validate_file_observations();
        }
        if self.path != "." || self.mode != "directory" {
            if self.observations.horizons.is_empty() {
                return Ok(());
            }
            return Err(ObservationLedgerError::DuplicateVisibilityAccounting);
        }
        for group in Self::ROOT_INVENTORY_GROUPS {
            let mut horizons = self
                .observations
                .horizons
                .iter()
                .filter(|horizon| horizon.group == group);
            let horizon = horizons
                .next()
                .ok_or(ObservationLedgerError::MissingRequiredHorizon)?;
            if horizons.next().is_some() {
                return Err(ObservationLedgerError::DuplicateHorizon);
            }
            if horizon.scope != self.path {
                return Err(ObservationLedgerError::ScopeMismatch);
            }
            if horizon.shown != self.shown_root_facts(group) {
                return Err(ObservationLedgerError::ShownFactCountMismatch);
            }
        }
        if self.observations.horizons.len() != Self::ROOT_INVENTORY_GROUPS.len() {
            return Err(ObservationLedgerError::DuplicateVisibilityAccounting);
        }
        if self
            .hidden
            .iter()
            .any(|group| Self::LEGACY_ROOT_HIDDEN_REASONS.contains(&group.reason.as_str()))
        {
            return Err(ObservationLedgerError::DuplicateVisibilityAccounting);
        }
        Ok(())
    }

    fn validate_file_observations(&self) -> Result<(), ObservationLedgerError> {
        let mut shown_total = 0_u64;
        for group in Self::FILE_RELATIONSHIP_GROUPS {
            let mut horizons = self
                .observations
                .horizons
                .iter()
                .filter(|horizon| horizon.group == group);
            let horizon = horizons
                .next()
                .ok_or(ObservationLedgerError::MissingRequiredHorizon)?;
            if horizons.next().is_some() {
                return Err(ObservationLedgerError::DuplicateHorizon);
            }
            if horizon.scope != self.path {
                return Err(ObservationLedgerError::ScopeMismatch);
            }
            let shown = self.shown_file_relationships(group);
            if horizon.shown != shown {
                return Err(ObservationLedgerError::ShownFactCountMismatch);
            }
            shown_total += shown;
        }
        if self.observations.horizons.len() != Self::FILE_RELATIONSHIP_GROUPS.len()
            || shown_total != self.edges.len() as u64
            || self
                .hidden
                .iter()
                .any(|group| group.reason == "edges hidden by limit")
        {
            return Err(ObservationLedgerError::DuplicateVisibilityAccounting);
        }
        Ok(())
    }

    fn validate_symbol_observations(&self) -> Result<(), ObservationLedgerError> {
        let mut shown_total = 0_u64;
        for group in Self::SYMBOL_GROUPS {
            let mut horizons = self
                .observations
                .horizons
                .iter()
                .filter(|horizon| horizon.group == group);
            let horizon = horizons
                .next()
                .ok_or(ObservationLedgerError::MissingRequiredHorizon)?;
            if horizons.next().is_some() {
                return Err(ObservationLedgerError::DuplicateHorizon);
            }
            if horizon.scope != self.path {
                return Err(ObservationLedgerError::ScopeMismatch);
            }
            let shown = self.shown_symbol_facts(group);
            if horizon.shown != shown {
                return Err(ObservationLedgerError::ShownFactCountMismatch);
            }
            shown_total += shown;
        }
        if self.observations.horizons.len() != Self::SYMBOL_GROUPS.len()
            || shown_total != self.edges.len() as u64
            || self
                .hidden
                .iter()
                .any(|group| group.reason == "symbol edges hidden by limit")
        {
            return Err(ObservationLedgerError::DuplicateVisibilityAccounting);
        }
        Ok(())
    }

    fn is_exact_symbol_scope(&self) -> bool {
        self.path
            .rsplit_once('#')
            .is_some_and(|(file, symbol)| !file.is_empty() && !symbol.is_empty())
    }

    fn shown_symbol_facts(&self, group: &str) -> u64 {
        self.edges
            .iter()
            .filter(|edge| match group {
                "consumers" => edge.edge_type == "symbol_reference",
                _ => edge.edge_type == "tests",
            })
            .count() as u64
    }

    fn shown_file_relationships(&self, group: &str) -> u64 {
        self.edges
            .iter()
            .filter(|edge| match group {
                "imports" => edge.edge_type == "imports",
                "consumers" => edge.edge_type == "imported_by",
                _ => edge.edge_type == "tests",
            })
            .count() as u64
    }

    fn shown_root_facts(&self, group: &str) -> u64 {
        if group == "directory_surfaces" {
            return self.directory.len() as u64;
        }
        self.directory
            .iter()
            .filter(|surface| match group {
                "packages" => {
                    surface.kind.starts_with("package:")
                        || surface.kind.starts_with("support_package:")
                }
                "scripts" => surface.kind == "script",
                _ => Self::TEST_SURFACE_KINDS.contains(&surface.kind.as_str()),
            })
            .map(|surface| surface.examples.len() as u64)
            .sum()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DirectorySurface {
    pub id: String,
    pub kind: String,
    pub path: Option<String>,
    pub role: Option<String>,
    pub evidence: String,
    pub strength: EvidenceStrength,
    pub count: usize,
    pub examples: Vec<String>,
    pub hidden_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphLens {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub domain: DomainRef,
    pub lens: String,
    pub nodes: Vec<String>,
    pub edges: Vec<GraphEdge>,
    pub hidden: Vec<HiddenGroup>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    #[serde(rename = "type")]
    pub edge_type: String,
    pub evidence: String,
    pub strength: EvidenceStrength,
    pub locations: Vec<EvidenceLocation>,
}
