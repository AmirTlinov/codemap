// Responsibility: proof-map-lens-scope-repair
use crate::map::{
    directory_has_files, nearest_proof_scope, nearest_proof_scope_unknown, shell_quote,
};
use crate::model::{Project, ProofSurface, Unknown};

pub(crate) fn proof_map_changed_scope_repair_unknown(
    project: &Project,
    seed: &str,
    changed: &[String],
    proofs: &[ProofSurface],
) -> Option<Unknown> {
    if !proofs.is_empty()
        || !changed.iter().any(|path| path == seed)
        || (!project.files.contains_key(seed) && !directory_has_files(project, seed))
    {
        return None;
    }
    let nearest = nearest_proof_scope(project, seed)?;
    Some(nearest_proof_scope_unknown(
        seed,
        &nearest,
        format!("codemap proof {}", shell_quote(&nearest)),
    ))
}

pub(crate) fn proof_map_exact_scope_repair(
    project: &Project,
    scope: Option<&str>,
    surfaces: &[ProofSurface],
) -> Option<(Unknown, String)> {
    let target = scope?;
    if !surfaces.is_empty()
        || (!project.files.contains_key(target) && !directory_has_files(project, target))
    {
        return None;
    }
    let nearest = nearest_proof_scope(project, target)?;
    Some((
        nearest_proof_scope_unknown(
            target,
            &nearest,
            format!("codemap proof {}", shell_quote(&nearest)),
        ),
        format!("codemap proof-map {}", shell_quote(&nearest)),
    ))
}
