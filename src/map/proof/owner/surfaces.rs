// Responsibility: map-proof-owner-surfaces
use crate::map::{ci_owner_proof_surface_for_step, ci_run_steps};
use crate::model::{FileInfo, Project, ProofSurface};

pub(crate) fn owner_surface_proof_surfaces(project: &Project, anchor: &str) -> Vec<ProofSurface> {
    let Some(file) = project.files.get(anchor) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if file.has_role("manifest") {
        out.extend(manifest_script_proof_surfaces(project, file));
        out.extend(manifest_ci_reference_proof_surfaces(project, file));
    }
    if file.has_role("schema_contract") || schema_owner_path(&file.rel) {
        out.extend(schema_script_proof_surfaces(project, file));
        out.extend(schema_ci_reference_proof_surfaces(project, file));
    }
    if file.has_role("env_config") {
        out.extend(env_consumer_proof_surfaces(project, file));
        out.extend(env_ci_reference_proof_surfaces(project, file));
    }
    if file.has_role("build_ci") {
        out.extend(ci_owner_proof_surfaces(project, file));
    }
    out
}

pub(crate) fn ci_owner_proof_surfaces(project: &Project, file: &FileInfo) -> Vec<ProofSurface> {
    let Some(text) = project.read_indexed_text(&file.rel) else {
        return Vec::new();
    };
    ci_run_steps(&text)
        .into_iter()
        .filter_map(|step| ci_owner_proof_surface_for_step(project, &file.rel, step))
        .collect()
}

mod ci_reference_walk;
mod env_surfaces;
mod manifest_surfaces;
mod package_scripts;
mod schema_surfaces;

pub(crate) use ci_reference_walk::*;
pub(crate) use env_surfaces::*;
pub(crate) use manifest_surfaces::*;
pub(crate) use package_scripts::*;
pub(crate) use schema_surfaces::*;
