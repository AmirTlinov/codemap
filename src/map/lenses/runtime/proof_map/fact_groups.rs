// Responsibility: runtime-non-route-fact-group-census
use crate::map::{
    dedupe_runtime_entrypoints, env_surfaces_for_file, runtime_code_entrypoints,
    runtime_entrypoint_kind, runtime_manifest_entrypoints, surface_from_path, unknowns_for_file,
};
use crate::model::{
    EnvSurface, EvidenceStrength, FileInfo, IndexedBoundary, Project, Surface, Unknown,
};
use crate::repo::RuntimeExternalPathKind;

pub(super) struct RuntimeNonRouteFacts {
    pub entrypoints: Vec<Surface>,
    pub env: Vec<EnvSurface>,
    pub workers: Vec<Surface>,
    pub ci: Vec<Surface>,
    pub unknowns: Vec<Unknown>,
}

pub(super) fn runtime_non_route_facts(
    project: &Project,
    files: &[&FileInfo],
) -> RuntimeNonRouteFacts {
    let mut entrypoints = Vec::new();
    let mut env = Vec::new();
    let mut workers = Vec::new();
    let mut ci = Vec::new();
    let mut unknowns = Vec::new();
    for &file in files {
        match crate::repo::indexed_boundary(&project.root, file) {
            Some(IndexedBoundary::TraversalError | IndexedBoundary::UnavailableTrackedFile) => {
                continue;
            }
            Some(IndexedBoundary::ExternalGitlink) => continue,
            Some(IndexedBoundary::ExternalTree)
                if crate::repo::runtime_external_path_kind(&file.rel)
                    == RuntimeExternalPathKind::Container =>
            {
                continue;
            }
            _ => {}
        }
        if let Some(kind) = runtime_entrypoint_kind(file) {
            entrypoints.push(surface_from_path(
                kind,
                &file.rel,
                "file_convention",
                EvidenceStrength::High,
            ));
        }
        entrypoints.extend(runtime_manifest_entrypoints(project, file));
        entrypoints.extend(runtime_code_entrypoints(project, file));
        if file.has_role("build_ci") {
            ci.push(surface_from_path(
                "build_ci",
                &file.rel,
                "role:build_ci",
                EvidenceStrength::High,
            ));
        }
        if crate::repo::runtime_worker_or_job_convention(&file.rel) {
            workers.push(surface_from_path(
                "worker_or_job",
                &file.rel,
                "worker_job_path_convention",
                EvidenceStrength::Medium,
            ));
        }
        if crate::repo::is_source_ext(&file.ext) {
            env.extend(env_surfaces_for_file(project, file));
            unknowns.extend(unknowns_for_file(project, file));
        }
    }
    RuntimeNonRouteFacts {
        entrypoints: dedupe_runtime_entrypoints(entrypoints),
        env,
        workers,
        ci,
        unknowns,
    }
}
