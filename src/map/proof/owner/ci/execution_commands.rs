// Responsibility: exact-command-owner-crossings
use crate::map::{
    CiWorkflowStep, ci_command_workflow_dispatch_edges, command_tokens,
    package_script_body_for_target, repo_script_targets, script_target_for_path,
    structural_edge_with_locations,
};
use crate::model::{EvidenceLocation, EvidenceStrength, Project, StructuralEdge};
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) fn ci_command_execution_edges(
    project: &Project,
    rel: &str,
    from: &str,
    command: &str,
    line: usize,
) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    let location = || vec![EvidenceLocation::line(rel, line, "ci_command")];
    let scripts = repo_script_targets(project, command);
    edges.extend(ci_command_workflow_dispatch_edges(
        project, rel, from, command, line,
    ));
    for script in &scripts {
        edges.push(structural_edge_with_locations(
            from,
            script,
            "invokes_script",
            "static_command_path",
            EvidenceStrength::Hard,
            location(),
        ));
    }
    if let Some(process) = command_process(command) {
        edges.push(structural_edge_with_locations(
            from,
            format!("process:{process}"),
            "invokes_process",
            "static_command_head",
            EvidenceStrength::Hard,
            location(),
        ));
    }
    if let Some(target) = deployment_target(command, scripts.first().map(String::as_str)) {
        let owner = scripts.first().map(String::as_str).unwrap_or(from);
        edges.push(structural_edge_with_locations(
            owner,
            target,
            "deploys",
            "exact_mutation_syntax",
            EvidenceStrength::Hard,
            location(),
        ));
    }
    if let Some(target) = command_smoke_target(command, scripts.first().map(String::as_str)) {
        let owner = scripts.first().map(String::as_str).unwrap_or(from);
        edges.push(structural_edge_with_locations(
            owner,
            target,
            "smoke_checks",
            "exact_smoke_syntax",
            EvidenceStrength::Hard,
            location(),
        ));
    }
    edges
}

pub(crate) fn ci_script_execution_edges(project: &Project, script: &str) -> Vec<StructuralEdge> {
    if script.starts_with("script:") {
        let surface = project.scripts.iter().find(|surface| {
            script_target_for_path(surface.path.as_deref().unwrap_or("script"), &surface.name)
                == script
        });
        let (path, command, line) = if let Some(surface) = surface {
            (
                surface.path.clone().unwrap_or_else(|| "script".to_string()),
                surface.command.clone(),
                surface.line_start.unwrap_or(1),
            )
        } else if let Some(package) = package_script_body_for_target(project, script) {
            package
        } else {
            return Vec::new();
        };
        return ci_command_execution_edges(project, &path, script, &command, line)
            .into_iter()
            .filter(|edge| !(edge.edge_type == "invokes_script" && edge.to == script))
            .collect();
    }
    let Some(text) = project.read_indexed_text(script) else {
        return Vec::new();
    };
    let mut edges = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let process = command_process(trimmed).or_else(|| embedded_process(trimmed));
        if let Some(process) = process {
            edges.push(structural_edge_with_locations(
                script,
                format!("process:{process}"),
                "invokes_process",
                "script_process_syntax",
                EvidenceStrength::Hard,
                vec![EvidenceLocation::line(script, index + 1, "process_call")],
            ));
        }
        if let Some(target) = deployment_target(trimmed, Some(script)) {
            edges.push(structural_edge_with_locations(
                script,
                target,
                "deploys",
                "script_mutation_syntax",
                EvidenceStrength::Hard,
                vec![EvidenceLocation::line(script, index + 1, "deployment_call")],
            ));
        }
        if let Some(target) = command_smoke_target(trimmed, Some(script)) {
            edges.push(structural_edge_with_locations(
                script,
                target,
                "smoke_checks",
                "script_smoke_syntax",
                EvidenceStrength::Hard,
                vec![EvidenceLocation::line(script, index + 1, "smoke_call")],
            ));
        }
    }
    edges
}

pub(crate) fn ci_step_smoke_edge(
    rel: &str,
    step_id: &str,
    step: &CiWorkflowStep,
) -> Option<StructuralEdge> {
    let lower = step.name.to_ascii_lowercase();
    let declares_check = ["verify", "smoke", "prove", "wait for"]
        .iter()
        .any(|term| lower.contains(term))
        || lower.starts_with("check ");
    declares_check.then(|| {
        structural_edge_with_locations(
            step_id,
            format!("check:{}", step.name),
            "smoke_checks",
            "workflow_step_name",
            EvidenceStrength::High,
            vec![EvidenceLocation::line(rel, step.line, "ci_step")],
        )
    })
}

pub(crate) fn ci_receipt_edges(
    rel: &str,
    step_id: &str,
    step: &CiWorkflowStep,
) -> Vec<StructuralEdge> {
    let mut receipts = BTreeSet::new();
    for line in step.body.lines() {
        let lower = line.to_ascii_lowercase();
        let output_syntax = lower.contains("writefilesync(")
            || lower.contains("--out ")
            || lower.contains("--output ")
            || lower.contains("> ")
            || (step
                .uses
                .as_deref()
                .is_some_and(|uses| uses.contains("upload-artifact"))
                && lower.trim_start().starts_with("path:"));
        if !output_syntax {
            continue;
        }
        for token in command_tokens(line) {
            let token = clean_path_token(&token);
            if receipt_path(&token) {
                receipts.insert(token);
            }
        }
    }
    receipts
        .into_iter()
        .map(|receipt| {
            structural_edge_with_locations(
                step_id,
                format!("receipt:{receipt}"),
                "produces_receipt",
                "exact_output_path",
                EvidenceStrength::Hard,
                vec![EvidenceLocation::line(rel, step.line, "ci_step")],
            )
        })
        .collect()
}

fn command_process(command: &str) -> Option<String> {
    let words = shell_words(command);
    let mut index = 0;
    while index < words.len() {
        let token = clean_path_token(&words[index]);
        if token.is_empty()
            || token.contains('=')
            || matches!(token.as_str(), "env" | "sudo" | "command")
        {
            index += 1;
            continue;
        }
        if token.starts_with(['$', '(', '[', '{']) || shell_control_word(&token) {
            return None;
        }
        let executable = Path::new(&token)
            .file_name()?
            .to_str()?
            .to_ascii_lowercase();
        return known_process(&executable).then_some(executable);
    }
    None
}

fn embedded_process(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let call = ["spawn(", "spawnsync(", "execfile(", "execfilesync("]
        .iter()
        .find_map(|marker| lower.find(marker).map(|index| index + marker.len()))?;
    let rest = line[call..].trim_start();
    let quote = rest.chars().next().filter(|ch| matches!(ch, '\'' | '"'))?;
    let executable = rest[1..].split(quote).next()?.to_ascii_lowercase();
    known_process(&executable).then_some(executable)
}

fn deployment_target(command: &str, _script: Option<&str>) -> Option<String> {
    let lower = command.to_ascii_lowercase();
    let tokens = command_tokens(command);
    let process = command_process(command).or_else(|| embedded_process(command));
    if process.as_deref() == Some("git") && tokens.iter().any(|token| token == "push") {
        return Some("deployment:git-remote".to_string());
    }
    if process.as_deref() == Some("kubectl")
        && tokens.iter().any(|token| {
            matches!(
                token.as_str(),
                "apply" | "patch" | "delete" | "set" | "rollout"
            )
        })
    {
        return Some("deployment:kubernetes".to_string());
    }
    if process.as_deref() == Some("docker")
        && (tokens.iter().any(|token| token == "push")
            || (tokens.iter().any(|token| token == "buildx") && lower.contains("--push")))
    {
        return Some("deployment:registry".to_string());
    }
    None
}

fn command_smoke_target(command: &str, script: Option<&str>) -> Option<String> {
    let lower = command.to_ascii_lowercase();
    if lower.contains("curl -f") {
        let url = shell_words(command)
            .into_iter()
            .map(|word| clean_path_token(&word))
            .find(|word| word.starts_with("http") && !word.contains('$'))
            .unwrap_or_else(|| "http-surface".to_string());
        return Some(format!("smoke:{url}"));
    }
    let script = script?.to_ascii_lowercase();
    (script.contains("/smoke/") || script.contains("smoke") || script.contains("health"))
        .then(|| format!("smoke:{script}"))
}

pub(crate) fn shell_words(command: &str) -> Vec<String> {
    command
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|ch: char| matches!(ch, '\'' | '"' | ',' | ';'))
                .to_string()
        })
        .collect()
}

pub(crate) fn clean_path_token(token: &str) -> String {
    token
        .trim_matches(|ch: char| {
            matches!(
                ch,
                '\'' | '"' | '`' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';'
            )
        })
        .trim_end_matches('\\')
        .to_string()
}

fn receipt_path(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    (lower.ends_with(".json") || lower.ends_with(".jsonl"))
        && ["receipt", "artifact", "manifest", "proof", "report"]
            .iter()
            .any(|term| lower.contains(term))
}

fn shell_control_word(token: &str) -> bool {
    matches!(
        token,
        "if" | "then"
            | "else"
            | "fi"
            | "for"
            | "while"
            | "do"
            | "done"
            | "case"
            | "esac"
            | "set"
            | "echo"
            | "exit"
            | "test"
    )
}

fn known_process(token: &str) -> bool {
    matches!(
        token,
        "git"
            | "kubectl"
            | "docker"
            | "curl"
            | "node"
            | "pnpm"
            | "npm"
            | "yarn"
            | "make"
            | "just"
            | "cargo"
            | "go"
            | "python"
            | "python3"
            | "bash"
            | "sh"
            | "helm"
            | "gh"
            | "jq"
            | "aws"
            | "rsync"
            | "scp"
            | "ssh"
    )
}
