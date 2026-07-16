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
    pub const SCHEMA_VERSION: &'static str = "18";
    pub const RELATIONSHIP_GROUPS: [&'static str; 5] = [
        "outgoing",
        "incoming",
        "verification",
        "contracts",
        "boundary",
    ];
    pub const XRAY_GROUPS: [(&'static str, &'static str); 7] = [
        ("xray_roles", "file_xray_role_surfaces"),
        ("xray_outputs", "file_xray_output_surfaces"),
        ("xray_state", "file_xray_state_surfaces"),
        ("xray_side_effects", "file_xray_side_effect_surfaces"),
        ("xray_flow", "file_xray_flow_steps"),
        ("xray_nearby", "file_xray_nearby_surfaces"),
        ("xray_unknowns", "file_xray_unknown_surfaces"),
    ];
    pub const XRAY_DISPLAY_GROUPS: [&'static str; 7] = [
        "xray_roles",
        "xray_outputs",
        "xray_state",
        "xray_side_effects",
        "xray_flow",
        "xray_nearby",
        "xray_unknowns",
    ];
    pub const EXACT_FILE_GROUPS: [&'static str; 13] = [
        "outgoing",
        "incoming",
        "verification",
        "contracts",
        "boundary",
        "symbols",
        "xray_roles",
        "xray_outputs",
        "xray_state",
        "xray_side_effects",
        "xray_flow",
        "xray_nearby",
        "xray_unknowns",
    ];
    pub const EXACT_FILE_DISPLAY_GROUPS: [&'static str; 13] = [
        "outgoing",
        "incoming",
        "verification",
        "contracts",
        "boundary",
        "symbols",
        "xray_roles",
        "xray_outputs",
        "xray_state",
        "xray_side_effects",
        "xray_flow",
        "xray_nearby",
        "xray_unknowns",
    ];

    pub fn validate_observations(&self) -> Result<(), super::ObservationLedgerError> {
        self.observations.validate()?;
        if self.anchor.kind.starts_with("symbol") || self.anchor.kind == "missing_symbol" {
            self.validate_shown_facts("incoming", self.incoming.len())?;
            self.validate_shown_facts("verification", self.proof.len())?;
        } else if self.anchor.kind == "directory" {
            self.validate_relationship_observations(true)?;
        } else if self.anchor.kind != "missing" {
            self.validate_relationship_observations(false)?;
        }
        Ok(())
    }

    fn validate_relationship_observations(
        &self,
        directory: bool,
    ) -> Result<(), super::ObservationLedgerError> {
        for (group, shown) in [
            ("outgoing", self.outgoing.len()),
            ("incoming", self.incoming.len()),
            ("verification", self.proof.len()),
            ("contracts", self.contracts.len()),
            ("boundary", self.boundary.len()),
        ] {
            self.validate_shown_facts(group, shown)?;
        }
        if !directory {
            self.validate_shown_facts("symbols", self.anchor.symbols.len())?;
            let mut certificate_ids = std::collections::BTreeSet::new();
            for (group, shown) in self.xray.group_fact_counts() {
                self.validate_shown_facts(group, shown)?;
                let query_kind = Self::XRAY_GROUPS
                    .iter()
                    .find_map(|(candidate, query)| (*candidate == group).then_some(*query))
                    .ok_or(super::ObservationLedgerError::UnexpectedHorizon)?;
                let horizon = self
                    .observations
                    .horizons
                    .iter()
                    .find(|horizon| horizon.group == group)
                    .ok_or(super::ObservationLedgerError::MissingRequiredHorizon)?;
                let certificate = self
                    .observations
                    .certificates
                    .get(&horizon.count.certificate_id)
                    .ok_or(super::ObservationLedgerError::DanglingCertificate)?;
                if !certificate_ids.insert(&horizon.count.certificate_id) {
                    return Err(super::ObservationLedgerError::ReusedCertificate);
                }
                let expected_query = if group == "xray_outputs" {
                    query_kind.to_string()
                } else {
                    format!("{query_kind}_depth_{}", self.depth)
                };
                if certificate.query_kind != expected_query {
                    return Err(super::ObservationLedgerError::CertificateQueryMismatch);
                }
            }
        }
        let expected_groups = if directory {
            Self::RELATIONSHIP_GROUPS.len()
        } else {
            Self::EXACT_FILE_GROUPS.len()
        };
        if self.observations.horizons.len() != expected_groups
            || self.hidden.iter().any(|hidden| {
                let reason = hidden.reason.as_str();
                if directory {
                    matches!(
                        reason,
                        "directory outgoing edges hidden by limit"
                            | "directory incoming edges hidden by limit"
                            | "directory verification edges hidden by limit"
                            | "directory contract edges hidden by limit"
                            | "directory boundary edges hidden by limit"
                    )
                } else {
                    matches!(
                        reason,
                        "outgoing edges hidden by limit"
                            | "incoming edges hidden by limit"
                            | "verification edges hidden by limit"
                            | "contract edges hidden by limit"
                            | "boundary edges hidden by limit"
                            | "nested symbols hidden by default"
                            | "symbols hidden by limit"
                    )
                }
            })
        {
            return Err(super::ObservationLedgerError::DuplicateVisibilityAccounting);
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
    pub outputs: Vec<Surface>,
    pub state: Vec<Surface>,
    pub side_effects: Vec<Surface>,
    pub flow: Vec<FlowStep>,
    pub nearby: Vec<Surface>,
    pub unknowns: Vec<Unknown>,
}

impl XrayCard {
    pub(crate) fn group_fact_counts(&self) -> [(&'static str, usize); 7] {
        [
            ("xray_roles", self.roles.len()),
            ("xray_outputs", self.outputs.len()),
            ("xray_state", self.state.len()),
            ("xray_side_effects", self.side_effects.len()),
            ("xray_flow", self.flow.len()),
            ("xray_nearby", self.nearby.len()),
            ("xray_unknowns", self.unknowns.len()),
        ]
    }
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
    pub const SCHEMA_VERSION: &'static str = "6";

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

#[derive(Debug, Clone, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WhereSuggestion {
    pub name: String,
    pub defined_in: String,
    pub definition_count: usize,
    pub expand: String,
}
