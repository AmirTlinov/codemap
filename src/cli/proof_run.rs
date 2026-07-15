// Responsibility: proof-command-execution
mod safety;

pub(crate) use safety::*;

use crate::cli::{
    ProofArgs, ensure_valid_config, output_format_with_json_alias, output_with_prelude,
    proof_inputs, proof_section_name, shell_quote_arg,
};
use crate::{map, render, repo};
use anyhow::Result;
use anyhow::bail;
use std::collections::BTreeSet;
use std::env;
use std::path::Path;
use std::process::Command;

pub(crate) fn proof(project: &crate::model::Project, args: ProofArgs) -> Result<()> {
    ensure_valid_config(project)?;
    if args.run && args.section.is_some() {
        bail!("--section is display-only and cannot be combined with --run");
    }
    let (target, changed, selector, since_notice) = proof_inputs(project, &args)?;
    let limit = if args.include_hidden {
        usize::MAX / 2
    } else {
        args.limit
    };
    let mut report = map::proof_report(project, target, changed, selector, args.depth, limit);
    maybe_write_proof_changed_lens_cache(project, &args, &report);
    // Inject the fail-open snapshot notice after cache writes so it never pollutes
    // the cached report.
    if let Some(notice) = since_notice {
        report.unknowns.push(notice);
    }
    let prelude = repo::map_prelude(&project.root);
    if args.run {
        render::set_map_prelude(prelude, crate::cli::build_identity(false));
        render::proof(&report, proof_section_name(args.section));
        return run_proof_plan(project, &report);
    }
    output_with_prelude(
        output_format_with_json_alias(args.format, args.json),
        &report,
        &prelude,
        || render::proof(&report, proof_section_name(args.section)),
    )
}

fn maybe_write_proof_changed_lens_cache(
    project: &crate::model::Project,
    args: &ProofArgs,
    report: &crate::model::ProofReport,
) {
    if args.run
        || args.include_hidden
        || args
            .target
            .as_deref()
            .is_some_and(|target| target != "changed")
        || args
            .files
            .as_deref()
            .is_some_and(|files| !files.trim().is_empty())
    {
        return;
    }
    if args.target.as_deref() != Some("changed") && !args.staged && args.since.is_none() {
        return;
    }
    let selector = if args.target.as_deref() == Some("changed") {
        "changed".to_string()
    } else if args.staged {
        "--staged".to_string()
    } else if let Some(since) = args.since.as_deref() {
        format!("--since {}", shell_quote_arg(since))
    } else {
        "changed".to_string()
    };
    let _ = crate::cache::write_proof_changed_report(
        &project.cache_dir,
        repo::VERSION,
        &project.root,
        &selector,
        args.depth,
        args.limit,
        report,
    );
}

fn run_proof_plan(
    project: &crate::model::Project,
    report: &crate::model::ProofReport,
) -> Result<()> {
    let commands = proof_plan_commands_for_run(report);
    let plan = crate::model::VerificationPlan {
        minimal: commands,
        supplemental: Vec::new(),
        full_only_if_triggered: Vec::new(),
    };
    run_plan(project, &plan, false)
}

pub(crate) fn proof_plan_commands_for_run(report: &crate::model::ProofReport) -> Vec<String> {
    let proof_commands = proof_run_commands(report);
    if proof_commands.is_empty() {
        report.fallback.clone()
    } else {
        proof_commands
    }
}

fn proof_run_commands(report: &crate::model::ProofReport) -> Vec<String> {
    report
        .proofs
        .iter()
        .filter(|proof| proof_surface_command_closes_run(proof))
        .filter_map(|proof| proof.command.clone())
        .collect()
}

fn proof_surface_command_closes_run(proof: &crate::model::ProofSurface) -> bool {
    crate::proof_classification::proof_surface_is_runnable_validation(proof)
}

fn run_plan(
    project: &crate::model::Project,
    plan: &crate::model::VerificationPlan,
    include_supplemental: bool,
) -> Result<()> {
    for command in planned_run_commands(plan, include_supplemental)? {
        let command = resolve_run_command(&command)?;
        println!("\n$ {command}");
        let status = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .current_dir(&project.root)
            .status()?;
        if !status.success() {
            bail!("verification command failed: {command}");
        }
    }
    Ok(())
}

pub(crate) fn planned_run_commands(
    plan: &crate::model::VerificationPlan,
    include_supplemental: bool,
) -> Result<Vec<String>> {
    let mut commands = plan.minimal.clone();
    if include_supplemental {
        commands.extend(plan.supplemental.clone());
    }
    commands = commands
        .into_iter()
        .map(|command| command.trim().to_string())
        .filter(|command| !command.is_empty())
        .collect();
    if commands.is_empty() {
        bail!("no verification commands inferred; refusing to treat --run as successful");
    }
    let rejected: Vec<(String, ProofCommandRejection)> = commands
        .iter()
        .filter_map(|command| {
            proof_command_rejection(command).map(|reason| (command.clone(), reason))
        })
        .collect();
    if !rejected.is_empty() {
        for (command, reason) in &rejected {
            match reason {
                ProofCommandRejection::Placeholder => {
                    eprintln!("codemap: cannot run placeholder verification command: {command}");
                }
                ProofCommandRejection::Unsafe(reason) => {
                    eprintln!(
                        "codemap: refusing unsafe verification command: {command} ({reason})"
                    );
                }
                ProofCommandRejection::Unknown => {
                    eprintln!("codemap: refusing unknown verification command: {command}");
                }
            }
        }
        if rejected
            .iter()
            .all(|(_, reason)| matches!(reason, ProofCommandRejection::Placeholder))
        {
            bail!(
                "verification plan contains non-runnable placeholder commands for the selected scope"
            );
        }
        bail!("verification plan contains rendered commands that codemap will not run by default");
    }
    Ok(unique_preserve_order(commands))
}

pub(crate) fn resolve_run_command(command: &str) -> Result<String> {
    let trimmed = command.trim();
    if trimmed == "codemap" || trimmed.starts_with("codemap ") {
        let exe = env::current_exe()?;
        let exe = exe.canonicalize().unwrap_or(exe);
        let suffix = trimmed.strip_prefix("codemap").unwrap_or_default();
        return Ok(format!("{}{}", shell_quote_path(&exe), suffix));
    }
    Ok(trimmed.to_string())
}

fn unique_preserve_order(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            out.push(value);
        }
    }
    out
}

fn shell_quote_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-' | ':'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProofCommandRejection {
    Placeholder,
    Unsafe(&'static str),
    Unknown,
}

fn proof_command_rejection(command: &str) -> Option<ProofCommandRejection> {
    let command = command.trim();
    if command.is_empty() || command.contains("nearest domain tests") {
        return Some(ProofCommandRejection::Placeholder);
    }
    if let Some(reason) = unsafe_shell_syntax_reason(command) {
        return Some(ProofCommandRejection::Unsafe(reason));
    }
    if safe_proof_command(command) {
        return None;
    }
    if let Some(reason) = unsafe_proof_command_reason(command) {
        return Some(ProofCommandRejection::Unsafe(reason));
    }
    Some(ProofCommandRejection::Unknown)
}
