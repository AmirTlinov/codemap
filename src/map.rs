use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

use serde::Serialize;

use crate::cache;
use crate::evidence::{import_statement_locations, line_looks_like_import_or_reexport};
use crate::model::{
    BoundaryFact, BoundaryFacts, BoundaryFinding, BoundaryMapReport, BoundaryReport,
    ChangedCouplingFact, ChangedMapDelta, ChangedProofCommand, ChangedProofSummary, ChangedReport,
    ChangedRisk, ChangedSymbol, ConeReport, ContractReport, DeleteReport, DiffMapReport,
    DirectorySurface, Domain, DomainRef, EnvDeclaration, EnvSurface, EvidenceLocation,
    EvidenceStrength, FileInfo, FileSummary, FlowReport, FlowStep, GitChange, GraphEdge, GraphLens,
    HiddenGroup, ImpactCluster, ImpactReport, LsReport, PackageDependency, PlaceReport, Project,
    ProofCoverageSummary, ProofCoveredPath, ProofGap, ProofMapReport, ProofReport, ProofSurface,
    ProofWiringFact, Risk, RuntimeReport, RuntimeRoute, SiblingsReport, StructuralEdge, Surface,
    TeachProofCommand, TeachReport, TeachRolePattern, Unknown, VerificationPlan, XrayCard,
};
use crate::repo;

mod graph_lens;
pub use graph_lens::graph_lens;

include!("map/edges.rs");
include!("map/facts.rs");
include!("map/unknowns.rs");
include!("map/status.rs");
include!("map/teach.rs");
include!("map/boundary_facts.rs");
include!("map/entry.rs");
include!("map/ls.rs");
include!("map/root_inventory_ls.rs");
include!("map/root_inventory_proof_map.rs");
include!("map/root_inventory_ls_helpers.rs");
include!("map/directory_edges.rs");
include!("map/directory_owner_edges.rs");
include!("map/cone_directory.rs");
include!("map/cone_traversal.rs");
include!("map/cone_owner_surfaces.rs");
include!("map/proof_runner_neighbors.rs");
include!("map/cone_env_surfaces.rs");
include!("map/cone_xray.rs");
include!("map/proof_edges.rs");
include!("map/directory_helpers.rs");
include!("map/symbol_summary.rs");
include!("map/symbol_edges.rs");
include!("map/symbol_body_refs.rs");
include!("map/jsx_and_exports.rs");
include!("map/js_export_scan.rs");
include!("map/js_identifier_refs.rs");
include!("map/file_metadata.rs");
include!("map/test_edges.rs");
include!("map/test_surface.rs");
include!("map/proof_precedence.rs");
include!("map/manifest_owner_surfaces.rs");
include!("map/scope_repair.rs");
include!("map/proof_entry.rs");
include!("map/proof_coverage.rs");
include!("map/proof_owner_surfaces.rs");
include!("map/proof_owner_ci_parse.rs");
include!("map/proof_owner_ci.rs");
include!("map/proof_owner_ci_validation.rs");
include!("map/proof_owner_ci_script_body.rs");
include!("map/proof_surfaces.rs");
include!("map/proof_locations.rs");
include!("map/proof_commands.rs");
include!("map/proof_wiring.rs");
include!("map/impact.rs");
include!("map/lenses.rs");
include!("map/boundary.rs");
include!("map/package_consumers.rs");
include!("map/command_inference_roles.rs");
include!("map/command_inference.rs");
include!("map/resolve.rs");
include!("map/tests.rs");
