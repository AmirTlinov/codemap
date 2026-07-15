// Responsibility: cone-and-where-report-types
use serde::{Deserialize, Serialize};

use super::FlowStep;

use super::{
    EnvDeclaration, EvidenceStrength, FileSummary, HiddenGroup, ObservationLedger, ObservedCount,
    StructuralEdge, Unknown,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Surface {
    pub id: String,
    pub kind: String,
    pub path: Option<String>,
    pub role: Option<String>,
    pub evidence: String,
    pub strength: EvidenceStrength,
    pub count: Option<usize>,
    pub examples: Vec<String>,
    pub hidden_count: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConeReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub anchor: FileSummary,
    pub depth: usize,
    pub xray: XrayCard,
    pub declared_env: Vec<EnvDeclaration>,
    pub outgoing: Vec<StructuralEdge>,
    pub incoming: Vec<StructuralEdge>,
    pub proof: Vec<StructuralEdge>,
    pub contracts: Vec<StructuralEdge>,
    pub boundary: Vec<StructuralEdge>,
    pub observations: ObservationLedger,
    pub hidden: Vec<HiddenGroup>,
    pub unknowns: Vec<Unknown>,
    pub expand: Vec<String>,
}

impl ConeReport {
    pub const SCHEMA_VERSION: &'static str = "12";

    pub fn validate_observations(&self) -> Result<(), super::ObservationLedgerError> {
        self.observations.validate()?;
        if self.anchor.kind.starts_with("symbol") || self.anchor.kind == "missing_symbol" {
            self.validate_shown_facts("incoming", self.incoming.len())?;
            self.validate_shown_facts("verification", self.proof.len())?;
        }
        Ok(())
    }

    fn validate_shown_facts(
        &self,
        group: &str,
        fact_count: usize,
    ) -> Result<(), super::ObservationLedgerError> {
        let horizon = self
            .observations
            .horizons
            .iter()
            .find(|horizon| horizon.group == group)
            .ok_or(super::ObservationLedgerError::MissingRequiredHorizon)?;
        if horizon.shown != fact_count as u64 {
            return Err(super::ObservationLedgerError::ShownFactCountMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct XrayCard {
    pub roles: Vec<Surface>,
    pub inputs: Vec<StructuralEdge>,
    pub outputs: Vec<Surface>,
    pub state: Vec<Surface>,
    pub side_effects: Vec<Surface>,
    pub direct_consumers: Vec<StructuralEdge>,
    pub mediated_consumers: Vec<StructuralEdge>,
    pub flow: Vec<FlowStep>,
    pub nearby: Vec<Surface>,
    pub proof_hard: Vec<StructuralEdge>,
    pub proof_direct: Vec<StructuralEdge>,
    pub proof_mediated: Vec<StructuralEdge>,
    pub proof_soft: Vec<StructuralEdge>,
    pub unknowns: Vec<Unknown>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WhereReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub query: String,
    pub kind_filter: Option<String>,
    pub total_matches: usize,
    pub observations: ObservationLedger,
    pub definitions: Vec<WhereDefinition>,
    pub soft_suggestions: Vec<WhereSuggestion>,
    pub unknowns: Vec<Unknown>,
    pub hidden: Vec<HiddenGroup>,
    pub expand: Vec<String>,
    // Rich single-definition cone map, rendered only (kept out of JSON to keep the
    // where contract flat). When there is exactly one match, `where` is structurally
    // identical to `cone file#symbol`.
    #[serde(skip)]
    pub detail: Option<Box<ConeReport>>,
}

impl WhereReport {
    pub fn validate_observations(&self) -> Result<(), super::ObservationLedgerError> {
        self.observations.validate()?;
        validate_horizon_shown(
            &self.observations,
            "definition_matches",
            self.definitions.len(),
        )?;
        for definition in &self.definitions {
            definition.observations.validate()?;
            for (group, shown) in [
                ("consumers", definition.consumers.len()),
                ("incoming", definition.incoming.len()),
                ("verification", definition.verification.len()),
            ] {
                validate_horizon_shown(&definition.observations, group, shown)?;
            }
        }
        Ok(())
    }
}

fn validate_horizon_shown(
    ledger: &ObservationLedger,
    group: &str,
    fact_count: usize,
) -> Result<(), super::ObservationLedgerError> {
    let horizon = ledger
        .horizons
        .iter()
        .find(|horizon| horizon.group == group)
        .ok_or(super::ObservationLedgerError::MissingRequiredHorizon)?;
    if horizon.shown != fact_count as u64 {
        return Err(super::ObservationLedgerError::ShownFactCountMismatch);
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct WhereDefinition {
    pub anchor: FileSummary,
    pub consumers: Vec<StructuralEdge>,
    pub consumers_total: ObservedCount,
    pub incoming: Vec<StructuralEdge>,
    pub verification: Vec<StructuralEdge>,
    pub observations: ObservationLedger,
    pub hidden: Vec<HiddenGroup>,
    pub expand: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WhereSuggestion {
    pub name: String,
    pub defined_in: String,
    pub definition_count: usize,
    pub expand: String,
}
