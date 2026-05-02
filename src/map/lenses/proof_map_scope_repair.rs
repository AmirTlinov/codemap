fn proof_map_changed_scope_repair_unknown(
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

fn proof_map_exact_scope_repair(
    project: &Project,
    scope: Option<&str>,
    direct: &[ProofSurface],
    indirect: &[ProofSurface],
    e2e: &[ProofSurface],
    contract: &[ProofSurface],
) -> Option<(Unknown, String)> {
    let target = scope?;
    if !direct.is_empty()
        || !indirect.is_empty()
        || !e2e.is_empty()
        || !contract.is_empty()
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
