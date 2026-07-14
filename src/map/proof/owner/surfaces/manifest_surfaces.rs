// Responsibility: map-proof-owner-surfaces-manifest
use crate::map::{
    cargo_manifest_builtin_proof_surfaces, ci_run_reference_proof_surfaces, command_tokens,
    manifest_ci_run_match_reason, manifest_file_name, package_json_scripts, package_script_command,
    swift_manifest_builtin_proof_surfaces, workspace_manifest_ci_reference_proof_surfaces,
    workspace_manifest_file, workspace_manifest_script_proof_surfaces,
};
use crate::model::{EvidenceLocation, EvidenceStrength, FileInfo, Project, ProofSurface};

pub(crate) fn manifest_script_proof_surfaces(
    project: &Project,
    file: &FileInfo,
) -> Vec<ProofSurface> {
    if workspace_manifest_file(&file.rel) {
        return workspace_manifest_script_proof_surfaces(project, file);
    }
    if manifest_file_name(&file.rel) == "Cargo.toml" {
        return cargo_manifest_builtin_proof_surfaces(project, file);
    }
    if manifest_file_name(&file.rel) == "Package.swift" {
        return swift_manifest_builtin_proof_surfaces(project, file);
    }
    let Some(package) = project
        .packages
        .iter()
        .find(|package| package.manifest == file.rel)
    else {
        return Vec::new();
    };
    if package.ecosystem != "javascript" {
        return Vec::new();
    }
    package_json_scripts(project, &package.manifest)
        .into_iter()
        .filter(|(name, command, _)| manifest_script_is_proof_or_support_relevant(name, command))
        .map(|(name, command, line)| ProofSurface {
            command: package_script_command(project, package, &name),
            path: Some(package.manifest.clone()),
            target_anchor: Some(file.rel.clone()),
            evidence: manifest_script_evidence(&name, &command).to_string(),
            strength: EvidenceStrength::Hard,
            reason: format!("package manifest defines `{name}` script: {command}"),
            locations: vec![EvidenceLocation::line(
                &package.manifest,
                line,
                "package_script",
            )],
        })
        .collect()
}

pub(crate) fn manifest_ci_reference_proof_surfaces(
    project: &Project,
    file: &FileInfo,
) -> Vec<ProofSurface> {
    if workspace_manifest_file(&file.rel) {
        return workspace_manifest_ci_reference_proof_surfaces(project, file);
    }
    let Some(package) = project
        .packages
        .iter()
        .find(|package| package.manifest == file.rel)
    else {
        return Vec::new();
    };
    let scripts = package_json_scripts(project, &package.manifest);
    ci_run_reference_proof_surfaces(project, file, "manifest_ci_reference", |command| {
        manifest_ci_run_match_reason(package, &scripts, command)
    })
}

pub(crate) fn manifest_script_is_proof_relevant(name: &str, command: &str) -> bool {
    manifest_script_command_body_is_run_safe(command)
        && (manifest_script_name_is_proof_relevant(name)
            || manifest_script_command_is_proof_relevant(command))
}

pub(crate) fn manifest_script_is_proof_or_support_relevant(name: &str, command: &str) -> bool {
    manifest_script_name_is_proof_relevant(name)
        || manifest_script_command_is_proof_relevant(command)
}

pub(crate) fn manifest_script_evidence(name: &str, command: &str) -> &'static str {
    if manifest_script_is_proof_relevant(name, command) {
        "manifest_script"
    } else {
        "manifest_script_setup"
    }
}

fn manifest_script_name_is_proof_relevant(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "test"
            | "tests"
            | "lint"
            | "typecheck"
            | "type-check"
            | "type_check"
            | "check"
            | "build"
            | "verify"
            | "e2e"
    ) {
        return true;
    }
    lower
        .split([':', '-', '_', '.', ' '])
        .filter(|part| !part.is_empty())
        .any(|part| {
            matches!(
                part,
                "test" | "tests" | "lint" | "typecheck" | "check" | "build" | "verify" | "e2e"
            )
        })
}

fn manifest_script_command_is_proof_relevant(command: &str) -> bool {
    let tokens = command_tokens(command);
    if tokens.iter().any(|token| {
        matches!(
            token.as_str(),
            "vitest"
                | "jest"
                | "mocha"
                | "uvu"
                | "playwright"
                | "cypress"
                | "eslint"
                | "biome"
                | "tsc"
                | "svelte-check"
        )
    }) {
        return true;
    }
    tokens.iter().any(|token| {
        matches!(
            token.as_str(),
            "test" | "lint" | "typecheck" | "check" | "build" | "verify" | "e2e"
        )
    })
}

pub(crate) fn manifest_script_command_body_is_run_safe(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    if crate::proof_classification::proof_text_is_readonly_migration_status(&lower) {
        return true;
    }
    ![
        "--watch",
        " watch",
        ":watch",
        " dev",
        " start",
        " serve",
        " preview",
        " install",
        " codegen",
        " generate",
        " seed",
        " studio",
        " deploy",
        " release",
        " publish",
        " migrate",
        " db push",
        " db:push",
        " db:normalize",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}
