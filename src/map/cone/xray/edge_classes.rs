// Responsibility: map-cone-xray-edge-classes
use crate::model::{EvidenceStrength, StructuralEdge};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum XrayEvidenceBucket {
    Hard,
    Direct,
    Mediated,
    Soft,
}

pub(crate) fn xray_proof_bucket(edge: &StructuralEdge) -> XrayEvidenceBucket {
    if edge.edge_type == "setup_support_surface" {
        return XrayEvidenceBucket::Soft;
    }
    if xray_edge_is_mediated(edge) {
        return XrayEvidenceBucket::Mediated;
    }
    let base = crate::proof_classification::proof_base_evidence(&edge.evidence);
    if xray_edge_is_soft(edge) {
        return XrayEvidenceBucket::Soft;
    }
    if edge.strength == EvidenceStrength::Hard {
        return XrayEvidenceBucket::Hard;
    }
    if matches!(
        base,
        "test_import"
            | "test_imported_symbol_reference"
            | "test_reexported_symbol_reference"
            | "test_support_import"
            | "test_symbol_reference"
            | "e2e_route"
            | "manifest_script"
            | "schema_package_script"
            | "ci_run_step"
            | "ci_validation_step"
    ) {
        return XrayEvidenceBucket::Direct;
    }
    XrayEvidenceBucket::Soft
}

fn xray_edge_is_soft(edge: &StructuralEdge) -> bool {
    let base = crate::proof_classification::proof_base_evidence(&edge.evidence);
    matches!(
        base,
        "test_name"
            | "e2e_surface_phrase"
            | "e2e_path_surface"
            | "test_surface_phrase"
            | "test_surface_tokens"
            | "test_role_surface_match"
            | "script_path_token"
            | "script_surface_match"
            | "proof_neighbor_token_match"
    ) || (edge.strength < EvidenceStrength::High
        && !matches!(
            base,
            "test_import"
                | "test_imported_symbol_reference"
                | "test_reexported_symbol_reference"
                | "test_support_import"
                | "test_symbol_reference"
                | "e2e_route"
        ))
}

pub(crate) fn xray_edge_is_mediated(edge: &StructuralEdge) -> bool {
    edge.evidence.ends_with("_via_direct_consumer")
        || edge.evidence.ends_with("_via_direct_dependency")
        || edge.evidence.ends_with("_via_local_symbol_consumer")
        || edge.evidence.ends_with("_via_cone_depth")
        || edge.evidence.contains("reexport")
        || edge.evidence.contains("barrel")
        || edge.evidence.contains("module_aggregator")
}

pub(crate) fn xray_input_edge(edge: &StructuralEdge) -> bool {
    matches!(
        edge.edge_type.as_str(),
        "imports" | "direct_dependency" | "env_consumer" | "uses_lockfile" | "contract"
    ) || edge.edge_type.contains("dependency")
}
