fn changed_risks(
    project: &Project,
    changed_paths: &[String],
    git_state: &[GitChange],
    proof_map: &ProofMapReport,
    selector: &str,
) -> Vec<ChangedRisk> {
    let changed_set = changed_paths.iter().cloned().collect::<BTreeSet<_>>();
    let mut risks = Vec::new();
    push_changed_risk(
        &mut risks,
        "untracked_files_present",
        "low",
        git_state
            .iter()
            .filter(|change| change.status == "untracked")
            .map(|change| change.path.clone())
            .collect(),
        "untracked paths are present in the selected git state",
        "git_status",
        Some(format!(
            "codemap changed{} --section observed",
            changed_self_selector_suffix(selector)
        )),
    );
    push_changed_risk(
        &mut risks,
        "conflicts_present",
        "high",
        git_state
            .iter()
            .filter(|change| change.status == "conflicted")
            .map(|change| change.path.clone())
            .collect(),
        "conflicted paths are present in the selected git state",
        "git_status",
        Some(format!(
            "codemap changed{} --section observed",
            changed_self_selector_suffix(selector)
        )),
    );
    push_changed_risk(
        &mut risks,
        "generated_changed",
        "medium",
        changed_paths
            .iter()
            .filter(|path| changed_path_is_generated(project, path))
            .cloned()
            .collect(),
        "generated-looking changed path observed; source provenance is not inferred by codemap",
        "path_or_role",
        None,
    );
    push_changed_risk(
        &mut risks,
        "large_binary_changed",
        "high",
        changed_paths
            .iter()
            .filter(|path| changed_path_is_large_binary(project, path))
            .cloned()
            .collect(),
        "large binary-like changed paths are visible by metadata/path only",
        "metadata_or_extension",
        None,
    );
    push_changed_risk(
        &mut risks,
        "model_weight_like_changed",
        "high",
        changed_paths
            .iter()
            .filter(|path| changed_path_is_model_weight_like(path))
            .cloned()
            .collect(),
        "model/checkpoint/weight-like changed paths are visible by deterministic path patterns",
        "path_pattern",
        None,
    );
    push_changed_risk(
        &mut risks,
        "lockfile_without_manifest_change",
        "medium",
        changed_paths
            .iter()
            .filter(|path| {
                changed_manifest_for_lockfile(path)
                    .is_some_and(|manifest| !changed_set.contains(&manifest))
            })
            .cloned()
            .collect(),
        "lockfile changed without its paired manifest in the selected changes",
        "manifest_lock_pair",
        None,
    );
    push_changed_risk(
        &mut risks,
        "manifest_without_lockfile_change",
        "medium",
        changed_paths
            .iter()
            .filter(|path| {
                changed_lockfiles_for_manifest(project, path)
                    .iter()
                    .any(|lockfile| !changed_set.contains(lockfile))
            })
            .cloned()
            .collect(),
        "manifest changed while an existing paired lockfile was not selected",
        "manifest_lock_pair",
        None,
    );
    push_changed_risk(
        &mut risks,
        "protected_looking_path_changed",
        "medium",
        changed_paths
            .iter()
            .filter(|path| changed_path_is_protected_looking(path))
            .cloned()
            .collect(),
        "protected-looking generated/vendor/build/model path changed",
        "path_pattern",
        None,
    );
    push_changed_risk(
        &mut risks,
        "instruction_file_changed",
        "medium",
        changed_paths
            .iter()
            .filter(|path| changed_path_is_instruction_file(path))
            .cloned()
            .collect(),
        "repo instruction or guard file changed",
        "path_pattern",
        None,
    );
    push_changed_risk(
        &mut risks,
        "unknown_direct_proof",
        "medium",
        changed_missing_direct_paths(proof_map).into_iter().collect(),
        "changed paths still have missing direct deterministic proof in proof-map",
        "proof_map_unknown",
        Some(format!("codemap proof-map {selector} --raw-sensors")),
    );
    risks.sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.paths.cmp(&b.paths)));
    risks
}

fn push_changed_risk(
    risks: &mut Vec<ChangedRisk>,
    kind: &str,
    severity: &str,
    mut paths: Vec<String>,
    effect: &str,
    evidence_kind: &str,
    expand: Option<String>,
) {
    paths.sort();
    paths.dedup();
    if paths.is_empty() {
        return;
    }
    let evidence = paths
        .iter()
        .take(10)
        .map(|path| EvidenceLocation::path(path, evidence_kind))
        .collect();
    risks.push(ChangedRisk {
        kind: kind.to_string(),
        severity: severity.to_string(),
        count: paths.len(),
        paths,
        evidence,
        effect: effect.to_string(),
        expand,
    });
}

fn changed_coupling(
    project: &Project,
    changed_paths: &[String],
    proof_map: &ProofMapReport,
    selector: &str,
) -> Vec<ChangedCouplingFact> {
    let changed_set = changed_paths.iter().cloned().collect::<BTreeSet<_>>();
    let mut facts = Vec::new();
    let pair_paths = changed_paths
        .iter()
        .filter(|path| {
            changed_manifest_for_lockfile(path)
                .is_some_and(|manifest| changed_set.contains(&manifest))
                || changed_lockfiles_for_manifest(project, path)
                    .into_iter()
                    .any(|lockfile| changed_set.contains(&lockfile))
        })
        .cloned()
        .collect::<Vec<_>>();
    push_changed_coupling(
        &mut facts,
        "lockfile_manifest_pair",
        if changed_paths.iter().any(|path| {
            changed_manifest_for_lockfile(path).is_some()
                || !changed_lockfiles_for_manifest(project, path).is_empty()
        }) {
            if pair_paths.is_empty() { "no" } else { "yes" }
        } else {
            "not_applicable"
        },
        pair_paths,
        "changed manifest/lockfile pair relationship from known package manager conventions",
        "manifest_lock_pair",
        None,
    );
    let runner_paths = changed_paths
        .iter()
        .filter(|path| changed_path_is_runner_like(path))
        .cloned()
        .collect::<Vec<_>>();
    let runner_script_paths = runner_paths
        .iter()
        .filter(|path| changed_runner_has_package_script(project, path))
        .cloned()
        .collect::<Vec<_>>();
    if !runner_paths.is_empty() {
        push_changed_coupling(
            &mut facts,
            "runner_has_package_script",
            if runner_script_paths.is_empty() { "no" } else { "yes" },
            if runner_script_paths.is_empty() {
                runner_paths
            } else {
                runner_script_paths
            },
            "changed runner/script-like paths checked against package script catalog",
            "script_catalog",
            None,
        );
    }
    let source_paths = changed_paths
        .iter()
        .filter(|path| changed_lens_path_looks_like_source(&path.to_ascii_lowercase()))
        .cloned()
        .collect::<Vec<_>>();
    if !source_paths.is_empty() {
        let missing_direct_paths = changed_missing_direct_paths(proof_map);
        let missing = source_paths
            .iter()
            .filter(|path| missing_direct_paths.contains(*path))
            .cloned()
            .collect::<Vec<_>>();
        push_changed_coupling(
            &mut facts,
            "source_has_direct_or_declared_proof_surface",
            if missing.is_empty() { "yes" } else { "no" },
            if missing.is_empty() {
                source_paths
            } else {
                missing
            },
            "changed source paths compared with deterministic proof-map direct/missing surfaces",
            "proof_map",
            Some(format!("codemap proof-map {selector} --raw-sensors")),
        );
    }
    facts
}

fn changed_missing_direct_paths(proof_map: &ProofMapReport) -> BTreeSet<String> {
    proof_map
        .missing_direct
        .iter()
        .filter_map(changed_surface_path)
        .collect()
}

fn changed_surface_path(surface: &Surface) -> Option<String> {
    surface
        .path
        .clone()
        .or_else(|| surface.examples.first().cloned())
        .or_else(|| {
            surface
                .id
                .strip_prefix("surface:missing_direct_proof:")
                .map(str::to_string)
        })
        .map(|path| repo::normalize_rel_path(&path))
}

fn push_changed_coupling(
    facts: &mut Vec<ChangedCouplingFact>,
    kind: &str,
    status: &str,
    mut paths: Vec<String>,
    effect: &str,
    evidence_kind: &str,
    expand: Option<String>,
) {
    if status == "not_applicable" {
        return;
    }
    paths.sort();
    paths.dedup();
    let evidence = paths
        .iter()
        .take(10)
        .map(|path| EvidenceLocation::path(path, evidence_kind))
        .collect();
    facts.push(ChangedCouplingFact {
        kind: kind.to_string(),
        status: status.to_string(),
        paths,
        evidence,
        effect: effect.to_string(),
        expand,
    });
}
