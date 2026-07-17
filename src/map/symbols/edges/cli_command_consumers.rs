// Responsibility: map-symbols-static-cli-command-consumers
use crate::map::{command_tokens, strip_inline_shell_comment, structural_edge_with_locations};
use crate::model::{EvidenceLocation, EvidenceStrength, FileInfo, Project, StructuralEdge};

pub(crate) fn static_cli_command_consumer_edges(
    project: &Project,
    source: &FileInfo,
    symbol_name: &str,
) -> Vec<StructuralEdge> {
    if !source.has_role("cli_surface") || generic_cli_symbol(symbol_name) {
        return Vec::new();
    }
    let command = symbol_name.replace('_', "-").to_ascii_lowercase();
    let mut matches = project
        .files
        .values()
        .filter(|file| file.has_role("proof_runner") && file.rel != source.rel)
        .filter_map(|file| static_cli_command_match(project, file, &command))
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    matches
        .into_iter()
        .take(5)
        .map(|(_, rel, line)| {
            structural_edge_with_locations(
                rel.clone(),
                format!("{}#{symbol_name}", source.rel),
                "invokes_cli_command",
                "static_cli_command_consumer",
                EvidenceStrength::Medium,
                vec![EvidenceLocation::line(
                    rel,
                    line,
                    "static_cli_command_consumer",
                )],
            )
        })
        .collect()
}

fn static_cli_command_match(
    project: &Project,
    candidate: &FileInfo,
    command: &str,
) -> Option<(usize, String, usize)> {
    if !matches!(candidate.ext.as_str(), "sh" | "bash" | "zsh" | "py") {
        return None;
    }
    let text = project.read_indexed_text(&candidate.rel)?;
    text.lines().enumerate().find_map(|(offset, line)| {
        let code = if matches!(candidate.ext.as_str(), "sh" | "bash" | "zsh") {
            strip_inline_shell_comment(line)
        } else {
            line.to_string()
        };
        let tokens = command_tokens(&code);
        let command_index = tokens.iter().position(|token| token == command)?;
        let invocation = tokens.iter().any(|token| {
            token == "codemap"
                || token.ends_with("/codemap")
                || token == "run_probe_command"
                || token.starts_with("subprocess.")
        });
        if !invocation {
            return None;
        }
        let score = if tokens
            .get(command_index + 1)
            .is_some_and(|next| next == "changed")
        {
            100
        } else if command_index > 0
            && tokens
                .get(command_index - 1)
                .is_some_and(|previous| previous == "codemap" || previous.ends_with("/codemap"))
        {
            90
        } else {
            70
        };
        Some((score, candidate.rel.clone(), offset + 1))
    })
}

fn generic_cli_symbol(symbol_name: &str) -> bool {
    matches!(
        symbol_name,
        "run" | "main" | "output" | "render" | "execute" | "dispatch" | "parse"
    )
}
