// Responsibility: map-cone-owner-surfaces
mod ci_edges;
mod manifest_edges;
mod runtime_edges;
mod schema_edges;

pub(crate) use ci_edges::*;
pub(crate) use manifest_edges::*;
pub(crate) use runtime_edges::*;
pub(crate) use schema_edges::*;

use crate::map::{
    ci_execution_unknowns, owner_env_edges, owner_env_unknowns, owner_surface_proof_surfaces,
    proof_runner_neighbor_edges, schema_owner_path, shell_quote, sort_edges,
    structural_edge_with_locations, unknown, workspace_manifest_file,
    workspace_manifest_member_packages,
};
use crate::model::{Project, ProofSurface, StructuralEdge, Unknown};

pub(crate) fn cone_owner_outgoing_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let Some(file) = project.files.get(rel) else {
        return Vec::new();
    };
    let mut edges = Vec::new();
    if file.has_role("manifest") {
        edges.extend(owner_manifest_edges(project, rel));
    }
    if file.has_role("schema_contract") || schema_owner_path(rel) {
        edges.extend(owner_schema_edges(project, rel));
    }
    if file.has_role("env_config") {
        edges.extend(owner_env_edges(project, rel));
    }
    if file.has_role("build_ci") {
        edges.extend(owner_ci_edges(project, rel));
    }
    edges.extend(owner_runtime_edges(project, rel));
    sort_edges(&mut edges);
    edges
}

pub(crate) fn cone_owner_incoming_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let Some(file) = project.files.get(rel) else {
        return Vec::new();
    };
    let mut edges = Vec::new();
    if file.has_role("manifest") {
        edges.extend(owner_manifest_incoming_edges(project, rel));
    }
    if file.has_role("schema_contract") || schema_owner_path(rel) {
        edges.extend(owner_schema_incoming_edges(project, rel));
    }
    sort_edges(&mut edges);
    edges
}

pub(crate) fn cone_owner_proof_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let mut edges = owner_surface_proof_surfaces(project, rel)
        .into_iter()
        .map(|proof| owner_proof_surface_edge(rel, proof))
        .collect::<Vec<_>>();
    if project
        .files
        .get(rel)
        .is_some_and(|file| file.has_role("proof_runner") || file.has_role("doctor"))
    {
        edges.extend(proof_runner_neighbor_edges(project, rel));
    }
    sort_edges(&mut edges);
    edges
}

pub(crate) fn owner_proof_surface_edge(rel: &str, proof: ProofSurface) -> StructuralEdge {
    let from = proof.path.clone().unwrap_or_else(|| rel.to_string());
    let to = proof
        .command
        .clone()
        .or_else(|| proof.path.clone())
        .unwrap_or_else(|| rel.to_string());
    let edge_type = if crate::proof_classification::proof_surface_is_runnable_validation(&proof) {
        "proof_surface"
    } else if crate::proof_classification::proof_surface_is_setup_or_support(&proof) {
        "setup_support_surface"
    } else if crate::proof_classification::proof_surface_is_evidence_only(&proof) {
        "evidence_surface"
    } else {
        "soft_evidence_surface"
    };
    structural_edge_with_locations(
        from,
        to,
        edge_type,
        proof.evidence,
        proof.strength,
        proof.locations,
    )
}

pub(crate) fn cone_owner_unknowns(project: &Project, rel: &str) -> Vec<Unknown> {
    let Some(file) = project.files.get(rel) else {
        return Vec::new();
    };
    let mut unknowns = Vec::new();
    if file.has_role("env_config") {
        unknowns.extend(owner_env_unknowns(project, rel));
    }
    if file.has_role("build_ci")
        && let Some(text) = project.read_indexed_text(rel)
    {
        unknowns.extend(ci_execution_unknowns(rel, &text));
    }
    unknowns.extend(owner_runtime_unknowns(project, rel));
    if file.has_role("manifest") && workspace_manifest_file(rel) {
        if workspace_manifest_member_packages(project, rel).is_empty() {
            unknowns.push(unknown(
                "workspace_members_not_found",
                Some(rel),
                None,
                "no workspace member package manifests matched this workspace manifest",
                "workspace manifest cone cannot show package membership edges",
                Some(format!("codemap ls {}", shell_quote(rel))),
            ));
        }
        if owner_surface_proof_surfaces(project, rel).is_empty() {
            unknowns.push(unknown(
                "workspace_proof_not_found",
                Some(rel),
                None,
                "no workspace root script or CI run step was found for this workspace manifest",
                "proof may fall back to broader repo commands",
                Some(format!("codemap proof {}", shell_quote(rel))),
            ));
        }
    }
    if (file.has_role("schema_contract") || schema_owner_path(rel))
        && owner_schema_edges(project, rel).is_empty()
        && owner_surface_proof_surfaces(project, rel).is_empty()
    {
        unknowns.push(unknown(
            "schema_owner_neighborhood_missing",
            Some(rel),
            None,
            "no migrations, schema scripts, CI references, or schema env links were found",
            "schema cone cannot show producer/proof rails for this owner surface",
            Some(format!("codemap proof-map {}", shell_quote(rel))),
        ));
    }
    unknowns
}
