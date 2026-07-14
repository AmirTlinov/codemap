// Responsibility: diff-map-surface-dedupe
use crate::model::{EnvSurface, ProofSurface, RuntimeRoute};
use std::collections::BTreeSet;

pub(crate) fn dedupe_runtime_routes(routes: &mut Vec<RuntimeRoute>) {
    let mut seen = BTreeSet::new();
    routes.retain(|route| {
        seen.insert((
            route.method.clone(),
            route.path.clone(),
            route.file.clone(),
            route.evidence.clone(),
        ))
    });
}

pub(crate) fn dedupe_env_surfaces(env: &mut Vec<EnvSurface>) {
    let mut seen = BTreeSet::new();
    env.retain(|surface| {
        seen.insert((
            surface.name.clone(),
            surface.used_by.clone(),
            surface.evidence.clone(),
        ))
    });
}

pub(crate) fn dedupe_proof_surfaces(proofs: &mut Vec<ProofSurface>) {
    let mut seen = BTreeSet::new();
    proofs.retain(|proof| {
        seen.insert((
            proof.path.clone(),
            proof.evidence.clone(),
            proof.reason.clone(),
        ))
    });
}
