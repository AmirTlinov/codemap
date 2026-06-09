enum CiOwnerPackageScriptBody {
    Safe(String),
    Setup(String),
    Unresolved,
}

fn ci_owner_proof_surface_for_step(
    project: &Project,
    ci_rel: &str,
    step: CiRunStep,
) -> Option<ProofSurface> {
    let (evidence, reason) = match ci_owner_package_script_body(project, &step.command) {
        Some(CiOwnerPackageScriptBody::Safe(reason)) => ("ci_run_step", reason),
        Some(CiOwnerPackageScriptBody::Setup(reason)) => ("ci_run_setup", reason),
        Some(CiOwnerPackageScriptBody::Unresolved) => return None,
        None => (
            "ci_run_step",
            ci_owner_validation_step_reason(&step.command)?,
        ),
    };
    Some(ProofSurface {
        command: Some(step.command),
        path: Some(ci_rel.to_string()),
        target_anchor: Some(ci_rel.to_string()),
        evidence: evidence.to_string(),
        strength: EvidenceStrength::Hard,
        reason,
        locations: vec![EvidenceLocation::line(ci_rel, step.line, "ci_step")],
    })
}

fn ci_owner_step_kind_for_project(project: &Project, command: &str) -> Option<CiOwnerStepKind> {
    match ci_owner_package_script_body(project, command) {
        Some(CiOwnerPackageScriptBody::Safe(_)) => Some(CiOwnerStepKind::Validation),
        Some(CiOwnerPackageScriptBody::Setup(_)) => Some(CiOwnerStepKind::Setup),
        Some(CiOwnerPackageScriptBody::Unresolved) => Some(CiOwnerStepKind::Control),
        None => ci_owner_step_kind(command),
    }
}

fn ci_owner_package_script_body(
    project: &Project,
    command: &str,
) -> Option<CiOwnerPackageScriptBody> {
    let command = strip_inline_shell_comment(command);
    let command = command.trim();
    if command.is_empty()
        || ci_owner_command_has_unsupported_shell_control(command)
        || ci_owner_command_has_unsupported_shell_composition(command)
        || ci_owner_command_is_non_validation(command)
    {
        return None;
    }
    let invoked = ci_owner_validation_script_names(command);
    if invoked.is_empty() {
        return None;
    }
    let matches = ci_owner_matching_package_scripts(project, command, &invoked);
    if matches.is_empty() {
        return Some(CiOwnerPackageScriptBody::Unresolved);
    }
    let unsafe_match = matches
        .iter()
        .find(|(_, _, body, _)| !manifest_script_command_body_is_run_safe(body));
    if let Some((package, name, body, _line)) = unsafe_match {
        return Some(CiOwnerPackageScriptBody::Setup(format!(
            "CI workflow run step invokes package script `{name}` in {}, but its manifest body is setup/support: {body}",
            package.manifest
        )));
    }
    let (package, name, body, line) = &matches[0];
    Some(CiOwnerPackageScriptBody::Safe(format!(
        "CI workflow run step invokes package validation script `{name}` from {} line {line}: {body}",
        package.manifest
    )))
}

fn ci_owner_validation_script_names(command: &str) -> Vec<String> {
    let tokens = command_tokens(command);
    unique(
        tokens
            .iter()
            .enumerate()
            .filter(|(index, token)| {
                ci_owner_script_name_is_validation(token)
                    && token_invokes_package_script(&tokens, *index)
            })
            .map(|(_, token)| token.to_string())
            .collect(),
    )
}

fn ci_owner_matching_package_scripts<'a>(
    project: &'a Project,
    command: &str,
    invoked: &[String],
) -> Vec<(&'a crate::model::PackageInfo, String, String, usize)> {
    let mut out = Vec::new();
    for package in project
        .packages
        .iter()
        .filter(|package| package.ecosystem == "javascript")
    {
        if !ci_owner_command_targets_package(project, package, command) {
            continue;
        }
        for (name, body, line) in package_json_scripts(project, &package.manifest) {
            if invoked.iter().any(|script| script == &name.to_ascii_lowercase()) {
                out.push((package, name, body, line));
            }
        }
    }
    out
}

fn ci_owner_command_targets_package(
    project: &Project,
    package: &crate::model::PackageInfo,
    command: &str,
) -> bool {
    if package.path == "." {
        return !project.packages.iter().any(|candidate| {
            candidate.ecosystem == "javascript"
                && candidate.path != "."
                && command_references_package(candidate, command)
        });
    }
    command_references_package(package, command)
}
