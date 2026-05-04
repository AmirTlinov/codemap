fn changed_fail_open_unknowns(project: &Project, changed: &[String]) -> Vec<Unknown> {
    let mut unknowns = Vec::new();
    for rel in changed {
        let Some(file) = project.files.get(rel) else {
            continue;
        };
        if changed_should_check_direct_proof(file)
            && !strict_test_edges_for_file(project, rel, usize::MAX)
                .iter()
                .any(|(_, _, strength)| *strength >= EvidenceStrength::High)
        {
            unknowns.push(unknown(
                "direct_test_import_not_found",
                Some(rel),
                None,
                "no direct test import, symbol reference, support import, or e2e route visit was found for this changed anchor",
                "proof map may still show scripts or soft evidence, but no deterministic direct test sensor was found",
                Some(format!("codemap proof-map {} --raw-sensors", shell_quote(rel))),
            ));
        }
        if file.has_role("manifest") {
            unknowns.extend(changed_manifest_unknowns(project, rel));
        }
        if file.has_role("schema_contract") || schema_owner_path(rel) {
            unknowns.extend(changed_schema_unknowns(project, rel));
        }
        if file.has_role("env_config") {
            unknowns.extend(owner_env_unknowns(project, rel));
            unknowns.extend(changed_env_unknowns(project, rel));
        }
        if file_uses_ci_run_step_syntax(file) {
            if owner_ci_edges(project, rel).is_empty() {
                unknowns.push(unknown(
                    "ci_run_step_not_found",
                    Some(rel),
                    None,
                    "no deterministic CI run step was found in this CI surface",
                    "CI cone cannot connect this workflow to scripts or validation commands",
                    Some(format!("codemap cone {}", shell_quote(rel))),
                ));
            } else if !owner_surface_proof_surfaces(project, rel)
                .iter()
                .any(proof_ci_run_step_is_validation)
            {
                unknowns.push(unknown_ci_validation_step_not_found(rel));
            }
        }
    }
    unknowns
}

fn file_uses_ci_run_step_syntax(file: &FileInfo) -> bool {
    if !file.has_role("build_ci") {
        return false;
    }
    let lower = file.rel.to_ascii_lowercase();
    let name = status_file_name(&lower);
    matches!(file.ext.as_str(), "yml" | "yaml")
        && (lower.starts_with(".github/workflows/")
            || lower.starts_with(".circleci/")
            || lower.starts_with(".buildkite/")
            || lower.starts_with(".teamcity/")
            || matches!(
                name.as_str(),
                ".gitlab-ci.yml"
                    | ".gitlab-ci.yaml"
                    | ".travis.yml"
                    | "azure-pipelines.yml"
                    | "azure-pipelines.yaml"
                    | "bitbucket-pipelines.yml"
                    | "bitbucket-pipelines.yaml"
                    | "cloudbuild.yml"
                    | "cloudbuild.yaml"
                    | "codemagic.yml"
                    | "codemagic.yaml"
                    | "bitrise.yml"
                    | "bitrise.yaml"
                    | ".drone.yml"
                    | ".drone.yaml"
                    | ".woodpecker.yml"
                    | ".woodpecker.yaml"
            ))
}

fn proof_ci_run_step_is_validation(proof: &ProofSurface) -> bool {
    crate::proof_classification::proof_base_evidence(&proof.evidence) == "ci_run_step"
        && crate::proof_classification::proof_surface_is_runnable_validation(proof)
}

fn changed_should_check_direct_proof(file: &FileInfo) -> bool {
    !file.has_role("generated")
        && !file.has_role("fixture")
        && !file.has_role("archive")
        && !file.has_role("test")
        && !file.has_role("test_support")
        && repo::is_source_ext(&file.ext)
}

fn changed_manifest_unknowns(project: &Project, rel: &str) -> Vec<Unknown> {
    let proofs = owner_surface_proof_surfaces(project, rel);
    if workspace_manifest_file(rel) {
        return changed_workspace_manifest_unknowns(project, rel, &proofs);
    }
    let mut unknowns = Vec::new();
    if !proofs.iter().any(|proof| {
        matches!(
            proof.evidence.as_str(),
            "manifest_script" | "cargo_manifest_command"
        ) && crate::proof_classification::proof_surface_is_runnable_validation(proof)
    }) {
        unknowns.push(unknown(
            "package_local_script_not_found",
            Some(rel),
            None,
            "no package-local test, lint, build, check, or verify script was found",
            "manifest proof falls back to broader commands or CI references when available",
            Some(format!("codemap proof {}", shell_quote(rel))),
        ));
    }
    if !proofs
        .iter()
        .any(|proof| {
            proof.evidence == "manifest_ci_reference"
                && crate::proof_classification::proof_surface_is_runnable_validation(proof)
        })
    {
        unknowns.push(unknown(
            "ci_reference_not_found",
            Some(rel),
            None,
            "no CI reference was found for this package manifest",
            "package-level validation may exist outside static workflow/script evidence",
            Some(format!("codemap cone {}", shell_quote(rel))),
        ));
    }
    if !project
        .package_edges
        .iter()
        .any(|edge| edge.to_manifest.as_deref() == Some(rel))
    {
        unknowns.push(unknown(
            "package_consumer_not_found",
            Some(rel),
            None,
            "no workspace package consumer was found for this manifest",
            "consumer map only includes deterministic manifest dependency edges",
            Some(format!("codemap graph --lens causal {}", shell_quote(rel))),
        ));
    }
    unknowns
}

fn changed_workspace_manifest_unknowns(
    project: &Project,
    rel: &str,
    proofs: &[ProofSurface],
) -> Vec<Unknown> {
    let mut unknowns = Vec::new();
    if !proofs
        .iter()
        .any(|proof| {
            proof.evidence == "workspace_manifest_script"
                && crate::proof_classification::proof_surface_is_runnable_validation(proof)
        })
    {
        unknowns.push(unknown(
            "workspace_script_not_found",
            Some(rel),
            None,
            "no workspace root test, lint, build, check, or verify script was found",
            "workspace manifest proof falls back to broader commands or CI references when available",
            Some(format!("codemap proof {}", shell_quote(rel))),
        ));
    }
    if !proofs
        .iter()
        .any(|proof| {
            proof.evidence == "workspace_manifest_ci_reference"
                && crate::proof_classification::proof_surface_is_runnable_validation(proof)
        })
    {
        unknowns.push(unknown(
            "ci_reference_not_found",
            Some(rel),
            None,
            "no CI reference was found for this workspace manifest",
            "workspace-level validation may exist outside static workflow/script evidence",
            Some(format!("codemap cone {}", shell_quote(rel))),
        ));
    }
    if workspace_manifest_member_packages(project, rel).is_empty() {
        unknowns.push(unknown(
            "workspace_members_not_found",
            Some(rel),
            None,
            "no workspace member package manifests matched this workspace manifest",
            "workspace manifest changed map cannot show package membership impact",
            Some(format!("codemap cone {}", shell_quote(rel))),
        ));
    }
    unknowns
}

fn changed_schema_unknowns(project: &Project, rel: &str) -> Vec<Unknown> {
    let edges = owner_schema_edges(project, rel);
    let incoming = owner_schema_incoming_edges(project, rel);
    let proofs = owner_surface_proof_surfaces(project, rel);
    let mut unknowns = Vec::new();
    if !edges.iter().any(|edge| edge.edge_type == "schema_migration") {
        unknowns.push(unknown(
            "schema_migration_not_found",
            Some(rel),
            None,
            "no migration file was found for this schema owner",
            "schema cone cannot show migration rails for this anchor",
            Some(format!("codemap cone {} --all", shell_quote(rel))),
        ));
    }
    if !edges.iter().any(|edge| edge.edge_type == "reads_env") {
        unknowns.push(unknown(
            "schema_env_link_not_found",
            Some(rel),
            None,
            "no static env() reference was found in this schema",
            "runtime database/config dependency is not connected from schema evidence",
            Some(format!("codemap runtime {}", shell_quote(rel))),
        ));
    }
    if incoming.is_empty() {
        unknowns.push(unknown(
            "schema_client_consumer_not_found",
            Some(rel),
            None,
            "no source file importing the generated schema client was found",
            "schema consumer map cannot prove application code reads this generated client",
            Some(format!("codemap cone {}", shell_quote(rel))),
        ));
    }
    if !proofs.iter().any(|proof| {
        matches!(
            proof.evidence.as_str(),
            "schema_package_script" | "schema_ci_reference"
        )
    }) {
        unknowns.push(unknown(
            "schema_check_not_found",
            Some(rel),
            None,
            "no schema script or CI reference was found",
            "schema proof map cannot point to generate, migrate, seed, or contract-check rails",
            Some(format!("codemap proof {}", shell_quote(rel))),
        ));
    }
    unknowns
}

fn changed_env_unknowns(project: &Project, rel: &str) -> Vec<Unknown> {
    let proofs = owner_surface_proof_surfaces(project, rel);
    let mut unknowns = Vec::new();
    if env_declared_keys(project, rel).is_empty() {
        unknowns.push(unknown(
            "env_key_not_found",
            Some(rel),
            None,
            "no KEY=value declarations were found in this env surface",
            "runtime config cone cannot show declared knobs for this file",
            Some(format!("codemap cone {}", shell_quote(rel))),
        ));
    }
    if !proofs.iter().any(|proof| proof.evidence == "env_ci_reference") {
        unknowns.push(unknown(
            "env_ci_reference_not_found",
            Some(rel),
            None,
            "no CI or workflow reference was found for declared env keys",
            "deployment/CI config linkage is not proven by static workflow evidence",
            Some(format!("codemap cone {}", shell_quote(rel))),
        ));
    }
    unknowns
}
