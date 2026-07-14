// Responsibility: map-proof-wiring-commands
use crate::map::{
    ParsedProofCommand, package_for_parsed_command, package_json_scripts,
    package_script_name_from_command, parse_static_command, proof_wiring_expand_for_proof,
    proof_wiring_fact, shell_quote, unquote_shell_token,
};
use crate::model::{EvidenceLocation, Project, ProofSurface, ProofWiringFact};

pub(crate) fn resolve_proof_command_fact(
    project: &Project,
    command: &str,
    proof: &ProofSurface,
    selector: &str,
) -> ProofWiringFact {
    let parsed = parse_static_command(command);
    let Some(parsed) = parsed else {
        return proof_wiring_fact(
            ("runner", "unknown"),
            command.to_string(),
            proof.path.clone(),
            "command shape was not statically resolved",
            "runner resolution is unknown until the command is inspected or executed explicitly",
            proof.locations.clone(),
            proof_wiring_expand_for_proof(selector, proof),
        );
    };
    if matches!(parsed.runner.as_str(), "make" | "just") {
        return resolve_make_like_command(project, &parsed, command, proof, selector);
    }
    if matches!(parsed.runner.as_str(), "pnpm" | "npm" | "yarn" | "bun") {
        return resolve_package_command(project, &parsed, command, proof, selector);
    }
    if matches!(
        parsed.runner.as_str(),
        "cargo"
            | "swift"
            | "go"
            | "pytest"
            | "python"
            | "python3"
            | "node"
            | "npx"
            | "tsx"
            | "vitest"
            | "jest"
            | "playwright"
    ) {
        return proof_wiring_fact(
            ("runner", "wired"),
            command.to_string(),
            proof.path.clone(),
            format!(
                "runner `{}` is a known static verification runner",
                parsed.runner
            ),
            "runner is structurally recognized; codemap did not execute it",
            proof.locations.clone(),
            proof_wiring_expand_for_proof(selector, proof),
        );
    }
    proof_wiring_fact(
        ("runner", "unknown"),
        command.to_string(),
        proof.path.clone(),
        format!(
            "runner `{}` is not in the static runner table",
            parsed.runner
        ),
        "runner resolution is unknown; codemap did not guess",
        proof.locations.clone(),
        proof_wiring_expand_for_proof(selector, proof),
    )
}

fn resolve_make_like_command(
    project: &Project,
    parsed: &ParsedProofCommand,
    command: &str,
    proof: &ProofSurface,
    selector: &str,
) -> ProofWiringFact {
    let Some(target) = parsed.args.first() else {
        return proof_wiring_fact(
            ("runner", "missing"),
            command.to_string(),
            proof.path.clone(),
            "make/just command has no target token",
            "declared verification command cannot resolve to a target",
            proof.locations.clone(),
            proof_wiring_expand_for_proof(selector, proof),
        );
    };
    let target = unquote_shell_token(target);
    let matched = project
        .scripts
        .iter()
        .find(|script| script.name == target && script.command.starts_with(parsed.runner.as_str()));
    if let Some(script) = matched {
        return proof_wiring_fact(
            ("runner", "wired"),
            command.to_string(),
            script.path.clone(),
            format!("{} target `{target}` is declared", parsed.runner),
            "declared verification command resolves to a local target; codemap did not run it",
            script
                .path
                .as_ref()
                .map(|path| {
                    EvidenceLocation::line(path, script.line_start.unwrap_or(1), "script_target")
                })
                .into_iter()
                .collect(),
            Some(format!(
                "codemap cone {} --depth 2",
                shell_quote(script.path.as_deref().unwrap_or("."))
            )),
        );
    }
    proof_wiring_fact(
        ("runner", "missing"),
        command.to_string(),
        proof.path.clone(),
        format!("{} target `{target}` was not found", parsed.runner),
        "declared verification command references a missing local target",
        proof.locations.clone(),
        Some("codemap ls . --section links".to_string()),
    )
}

fn resolve_package_command(
    project: &Project,
    parsed: &ParsedProofCommand,
    command: &str,
    proof: &ProofSurface,
    selector: &str,
) -> ProofWiringFact {
    let package = package_for_parsed_command(project, parsed, proof);
    let script_name = package_script_name_from_command(parsed);
    if let Some(script_name) = script_name
        && let Some(package) = package
    {
        let scripts = package_json_scripts(project, &package.manifest);
        if let Some((name, body, line)) = scripts
            .iter()
            .find(|(name, _, _)| name == &script_name)
            .cloned()
        {
            return proof_wiring_fact(
                ("runner", "wired"),
                command.to_string(),
                Some(package.manifest.clone()),
                format!("package script `{name}` resolves to `{body}`"),
                "declared verification command resolves through the package manifest; codemap did not run it",
                vec![EvidenceLocation::line(
                    &package.manifest,
                    line,
                    "package_script",
                )],
                Some(format!(
                    "codemap cone {} --depth 2",
                    shell_quote(&package.manifest)
                )),
            );
        }
        return proof_wiring_fact(
            ("runner", "missing"),
            command.to_string(),
            Some(package.manifest.clone()),
            format!("package script `{script_name}` was not found"),
            "declared verification command references a missing package script",
            vec![EvidenceLocation::path(
                &package.manifest,
                "package_manifest",
            )],
            Some(format!(
                "codemap cone {} --depth 2",
                shell_quote(&package.manifest)
            )),
        );
    }
    if parsed.args.iter().any(|arg| arg.starts_with("--filter")) && package.is_none() {
        return proof_wiring_fact(
            ("runner", "unknown"),
            command.to_string(),
            proof.path.clone(),
            "package filter did not resolve to an indexed package",
            "package-scoped runner wiring is unknown",
            proof.locations.clone(),
            proof_wiring_expand_for_proof(selector, proof),
        );
    }
    proof_wiring_fact(
        ("runner", "wired"),
        command.to_string(),
        proof.path.clone(),
        format!("runner `{}` is structurally recognized", parsed.runner),
        "runner is recognized, but no package script hop was required or detected",
        proof.locations.clone(),
        proof_wiring_expand_for_proof(selector, proof),
    )
}
