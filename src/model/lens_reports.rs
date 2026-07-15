// Responsibility: focused-lens-report-types
use serde::{Deserialize, Serialize};

use super::Surface;

use super::{
    BoundaryFinding, DomainRef, EvidenceLocation, EvidenceStrength, FileSummary, HiddenGroup,
    ObservationLedger, PackageDependency, ProofSurface, ProofWiringFact, RuntimeRoute,
    StructuralEdge, Unknown, VerificationTopology,
};

#[derive(Debug, Clone, Serialize)]
pub struct ContractReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub anchor: FileSummary,
    pub contract_kind: String,
    pub public_surface: bool,
    pub declarations: Vec<Surface>,
    pub lineage: Vec<StructuralEdge>,
    pub exported_contracts: Vec<Surface>,
    pub package_exports: Vec<StructuralEdge>,
    pub producers: Vec<StructuralEdge>,
    pub consumers: Vec<StructuralEdge>,
    pub cross_package_consumers: Vec<StructuralEdge>,
    pub proof: Vec<StructuralEdge>,
    pub unknowns: Vec<Unknown>,
    pub hidden: Vec<HiddenGroup>,
    pub expand: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub scope: String,
    pub entrypoints: Vec<Surface>,
    pub routes: Vec<RuntimeRoute>,
    pub paths: Vec<StructuralEdge>,
    pub scripts: Vec<Surface>,
    pub env: Vec<EnvSurface>,
    pub workers: Vec<Surface>,
    pub ci: Vec<Surface>,
    pub proof: Vec<StructuralEdge>,
    pub unknowns: Vec<Unknown>,
    pub observations: ObservationLedger,
    pub hidden: Vec<HiddenGroup>,
    pub expand: Vec<String>,
}

impl RuntimeReport {
    pub const SCHEMA_VERSION: &'static str = "7";
    pub(crate) const ROOT_RECURSIVE_HIDDEN_REASON: &'static str =
        "recursive runtime files hidden at root scope";
    pub(crate) const ROOT_RECURSIVE_HIDDEN_EXPAND: &'static str = "codemap runtime . --all";

    pub(crate) const OBSERVATION_GROUPS: [(&'static str, &'static str); 9] = [
        ("entrypoints", "runtime_entrypoint_surfaces"),
        ("routes", "supported_static_runtime_routes"),
        ("paths", "runtime_route_boundary_relations"),
        ("scripts", "runtime_script_catalog"),
        ("env", "static_runtime_environment_references"),
        ("workers", "runtime_worker_job_path_conventions"),
        ("ci", "indexed_build_ci_role_surfaces"),
        ("proof", "runtime_route_verification_relations"),
        ("unknowns", "runtime_unknown_detector_surface"),
    ];

    pub(crate) fn observation_query_kind(group: &str) -> Option<&'static str> {
        Self::OBSERVATION_GROUPS
            .iter()
            .find_map(|(candidate, query)| (*candidate == group).then_some(*query))
    }

    pub fn validate_observations(&self) -> Result<(), super::ObservationLedgerError> {
        self.observations.validate()?;
        if self
            .entrypoints
            .iter()
            .chain(&self.scripts)
            .chain(&self.workers)
            .chain(&self.ci)
            .any(|surface| surface.count == Some(0))
        {
            return Err(super::ObservationLedgerError::InvalidFactMultiplicity);
        }
        if self.observations.horizons.len() != Self::OBSERVATION_GROUPS.len() {
            return Err(
                if self.observations.horizons.len() < Self::OBSERVATION_GROUPS.len() {
                    super::ObservationLedgerError::MissingRequiredHorizon
                } else {
                    super::ObservationLedgerError::UnexpectedHorizon
                },
            );
        }
        let shown_counts = [
            self.entrypoints
                .iter()
                .map(|surface| surface.count.unwrap_or(1))
                .sum(),
            self.routes.len(),
            self.paths.len(),
            self.scripts.len(),
            self.env.len(),
            self.workers.len(),
            self.ci.len(),
            self.proof.len(),
            self.unknowns.len(),
        ];
        let mut certificate_ids = std::collections::BTreeSet::new();
        for ((group, query_kind), shown) in Self::OBSERVATION_GROUPS.iter().zip(shown_counts) {
            let mut matches = self
                .observations
                .horizons
                .iter()
                .filter(|horizon| horizon.group == *group);
            let horizon = matches
                .next()
                .ok_or(super::ObservationLedgerError::MissingRequiredHorizon)?;
            if matches.next().is_some() {
                return Err(super::ObservationLedgerError::DuplicateHorizon);
            }
            if horizon.scope != self.scope {
                return Err(super::ObservationLedgerError::ScopeMismatch);
            }
            if horizon.shown != shown as u64 {
                return Err(super::ObservationLedgerError::ShownFactCountMismatch);
            }
            if !certificate_ids.insert(&horizon.count.certificate_id) {
                return Err(super::ObservationLedgerError::ReusedCertificate);
            }
            let certificate = self
                .observations
                .certificates
                .get(&horizon.count.certificate_id)
                .ok_or(super::ObservationLedgerError::DanglingCertificate)?;
            if certificate.query_kind != *query_kind {
                return Err(super::ObservationLedgerError::CertificateQueryMismatch);
            }
            if horizon.count.observed > 0 && certificate.eligible_files == 0 {
                return Err(super::ObservationLedgerError::ObservedWithoutEligibleCandidate);
            }
        }
        const LEGACY_VISIBILITY_REASONS: [&str; 9] = [
            "runtime entrypoints hidden by limit",
            "runtime routes hidden by limit",
            "runtime path relations hidden by limit",
            "runtime scripts hidden by limit",
            "environment surfaces hidden by limit",
            "worker/job surfaces hidden by limit",
            "ci surfaces hidden by limit",
            "runtime verification edges hidden by limit",
            "runtime unknowns hidden by limit",
        ];
        if self
            .hidden
            .iter()
            .any(|hidden| LEGACY_VISIBILITY_REASONS.contains(&hidden.reason.as_str()))
        {
            return Err(super::ObservationLedgerError::DuplicateVisibilityAccounting);
        }
        Ok(())
    }

    pub fn validate_bounded_projection(
        &self,
        limit: usize,
    ) -> Result<(), super::ObservationLedgerError> {
        self.validate_observations()?;
        let row_counts = [
            self.entrypoints.len(),
            self.routes.len(),
            self.paths.len(),
            self.scripts.len(),
            self.env.len(),
            self.workers.len(),
            self.ci.len(),
            self.proof.len(),
            self.unknowns.len(),
        ];
        if row_counts.into_iter().any(|count| count > limit) {
            return Err(super::ObservationLedgerError::ProjectionMismatch);
        }
        Ok(())
    }

    pub(crate) fn validate_current_level_root_projection(
        &self,
        limit: usize,
    ) -> Result<(), super::ObservationLedgerError> {
        self.validate_bounded_projection(limit)?;
        if self.scope != "."
            || self
                .entrypoints
                .iter()
                .any(|surface| !runtime_root_entrypoint_is_current_level(surface))
            || self.scripts.iter().any(|surface| {
                surface
                    .path
                    .as_deref()
                    .is_none_or(|path| path.contains('/'))
            })
            || self.workers.iter().chain(&self.ci).any(|surface| {
                surface
                    .path
                    .as_deref()
                    .is_none_or(|path| !runtime_root_path_is_current_level(path))
            })
            || self
                .env
                .iter()
                .any(|surface| !runtime_root_path_is_current_level(&surface.used_by))
            || self.unknowns.iter().any(|unknown| {
                unknown
                    .path
                    .as_deref()
                    .is_some_and(|path| !runtime_root_path_is_current_level(path))
            })
        {
            return Err(super::ObservationLedgerError::ProjectionMismatch);
        }
        Ok(())
    }

    pub fn validate_full_projection(&self) -> Result<(), super::ObservationLedgerError> {
        self.validate_observations()?;
        if self.observations.horizons.iter().any(|horizon| {
            horizon.shown != horizon.count.observed
                || horizon.hidden != 0
                || horizon.expand.is_some()
        }) {
            return Err(super::ObservationLedgerError::ProjectionMismatch);
        }
        Ok(())
    }
}

fn runtime_root_path_is_current_level(path: &str) -> bool {
    !path.contains('/') || path.starts_with(".github/")
}

fn runtime_root_entrypoint_is_current_level(surface: &Surface) -> bool {
    if surface.kind == "runtime_container" {
        return surface.path.as_ref().is_some_and(|path| {
            surface.id == format!("surface:runtime_container:{path}")
                && surface.evidence == "current_level_runtime_container"
        });
    }
    if surface.kind == "cli_entrypoint" {
        let manifest = surface
            .id
            .strip_prefix("surface:cli_entrypoint:")
            .and_then(runtime_cli_entrypoint_manifest);
        return manifest.is_some_and(runtime_root_path_is_current_level);
    }
    surface
        .path
        .as_deref()
        .is_some_and(runtime_root_path_is_current_level)
}

fn runtime_cli_entrypoint_manifest(value: &str) -> Option<&str> {
    ["package.json", "Cargo.toml", "pyproject.toml"]
        .into_iter()
        .find_map(|name| {
            let end = value.find(&format!("{name}:"))? + name.len();
            Some(&value[..end])
        })
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnvSurface {
    pub name: String,
    pub used_by: String,
    pub declaration: Option<String>,
    pub evidence: String,
    pub strength: EvidenceStrength,
    pub locations: Vec<EvidenceLocation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProofMapReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub selector: String,
    pub scope: Option<String>,
    pub changed: Vec<String>,
    pub hard: Vec<ProofSurface>,
    pub direct_evidence: Vec<ProofSurface>,
    pub mediated_evidence: Vec<ProofSurface>,
    pub soft_evidence: Vec<ProofSurface>,
    pub setup_support: Vec<ProofSurface>,
    pub missing_direct: Vec<Surface>,
    pub commands: Vec<ProofSurface>,
    pub wiring: Vec<ProofWiringFact>,
    pub verification_topology: VerificationTopology,
    pub fallback: Vec<String>,
    pub unknowns: Vec<Unknown>,
    pub hidden: Vec<HiddenGroup>,
    pub expand: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeleteReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub anchor: FileSummary,
    pub direct_users: Vec<StructuralEdge>,
    pub symbol_users: Vec<StructuralEdge>,
    pub reexports: Vec<StructuralEdge>,
    pub package_exports: Vec<StructuralEdge>,
    pub tests: Vec<StructuralEdge>,
    pub runtime_refs: Vec<StructuralEdge>,
    pub unknowns: Vec<Unknown>,
    pub checklist: Vec<String>,
    pub hidden: Vec<HiddenGroup>,
    pub expand: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoundaryMapReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub scope: String,
    pub domains: Vec<DomainRef>,
    pub actual_cross_edges: Vec<StructuralEdge>,
    pub public_boundary_files: Vec<FileSummary>,
    pub test_only_crossings: Vec<StructuralEdge>,
    pub package_edges: Vec<PackageDependency>,
    pub explicit_forbidden_findings: Vec<BoundaryFinding>,
    pub unknowns: Vec<Unknown>,
    pub hidden: Vec<HiddenGroup>,
    pub expand: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FlowReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub anchor: String,
    pub flow_kind: String,
    pub precision: String,
    pub entry: Option<FileSummary>,
    pub steps: Vec<FlowStep>,
    pub side_effects: Vec<Surface>,
    pub contracts: Vec<StructuralEdge>,
    pub proof: Vec<StructuralEdge>,
    pub unknown_breaks: Vec<Unknown>,
    pub hidden: Vec<HiddenGroup>,
    pub expand: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FlowStep {
    pub index: usize,
    pub anchor: String,
    pub kind: String,
    pub evidence: String,
    pub locations: Vec<EvidenceLocation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SiblingsReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub scope: String,
    pub same_kind: Vec<Surface>,
    pub route_service_test_triplets: Vec<Surface>,
    pub shared_helpers: Vec<StructuralEdge>,
    pub shared_contracts: Vec<StructuralEdge>,
    pub proof_pattern: Vec<ProofSurface>,
    pub unknowns: Vec<Unknown>,
    pub hidden: Vec<HiddenGroup>,
    pub expand: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlaceReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub scope: String,
    pub requested_kind: String,
    pub existing_surfaces: Vec<Surface>,
    pub local_conventions: Vec<String>,
    pub paired_proof_pattern: Vec<ProofSurface>,
    pub shared_contracts: Vec<StructuralEdge>,
    pub unknowns: Vec<Unknown>,
    pub hidden: Vec<HiddenGroup>,
    pub expand: Vec<String>,
}
