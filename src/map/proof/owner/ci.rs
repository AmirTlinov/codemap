// Responsibility: map-proof-owner-ci
use crate::map::{
    flush_ci_pending_command, push_ci_logical_command, push_ci_run_steps, shell_heredoc_delimiter,
};

#[derive(Debug, Clone)]
pub(crate) struct CiRunStep {
    pub(crate) command: String,
    pub(crate) line: usize,
}

pub(crate) fn ci_run_steps(text: &str) -> Vec<CiRunStep> {
    ci_run_steps_with_offset(text, 0)
}

pub(crate) fn ci_run_steps_with_offset(text: &str, line_offset: usize) -> Vec<CiRunStep> {
    let lines = text.lines().collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut index = 0usize;
    while index < lines.len() {
        let Some(spec) = ci_run_spec(lines[index]) else {
            index += 1;
            continue;
        };
        if ci_value_is_block_scalar(&spec.value) {
            index += 1;
            let mut pending: Option<CiRunStep> = None;
            let mut heredoc_until: Option<String> = None;
            while index < lines.len() {
                let line = lines[index];
                if line.trim().is_empty() {
                    index += 1;
                    continue;
                }
                if let Some(delimiter) = heredoc_until.as_deref() {
                    if line.trim() == delimiter {
                        heredoc_until = None;
                    }
                    index += 1;
                    continue;
                }
                let indent = leading_whitespace_count(line);
                if indent <= spec.indent {
                    break;
                }
                let command = trim_yaml_scalar(line.trim());
                if !command.is_empty() && !command.starts_with('#') {
                    let heredoc_delimiter = shell_heredoc_delimiter(&command);
                    push_ci_logical_command(
                        &mut out,
                        &mut pending,
                        command,
                        index + 1 + line_offset,
                    );
                    if pending.is_none() {
                        heredoc_until = heredoc_delimiter;
                    }
                }
                index += 1;
            }
            flush_ci_pending_command(&mut out, &mut pending);
            continue;
        }
        let command = trim_yaml_scalar(&spec.value);
        if !command.is_empty() {
            push_ci_run_steps(&mut out, command, index + 1 + line_offset);
        }
        index += 1;
    }
    out
}

#[derive(Debug, Clone)]
struct CiRunSpec {
    value: String,
    indent: usize,
}

fn ci_run_spec(line: &str) -> Option<CiRunSpec> {
    let indent = leading_whitespace_count(line);
    let trimmed = line.trim_start();
    let value = trimmed
        .strip_prefix("- run:")
        .or_else(|| trimmed.strip_prefix("run:"))?
        .trim()
        .to_string();
    Some(CiRunSpec { value, indent })
}

pub(crate) fn ci_inline_run_command(line: &str) -> Option<String> {
    let spec = ci_run_spec(line)?;
    if ci_value_is_block_scalar(&spec.value) {
        return None;
    }
    let command = trim_yaml_scalar(&spec.value);
    (!command.is_empty()).then_some(command)
}

fn ci_value_is_block_scalar(value: &str) -> bool {
    value
        .split_whitespace()
        .next()
        .is_some_and(|token| token.starts_with('|') || token.starts_with('>'))
}

fn trim_yaml_scalar(value: &str) -> String {
    let mut value = value.trim().to_string();
    if value.len() >= 2 {
        let first = value.as_bytes()[0] as char;
        let last = value.as_bytes()[value.len() - 1] as char;
        if matches!(first, '"' | '\'') && first == last {
            value = value[1..value.len() - 1].to_string();
        }
    }
    value.trim().to_string()
}

fn leading_whitespace_count(line: &str) -> usize {
    line.chars().take_while(|ch| ch.is_whitespace()).count()
}

mod execution;
mod execution_commands;
mod execution_projection;
mod execution_targets;
mod match_reasons;
mod package_reference;
mod shell_tokens;
mod workflow;

pub(crate) use execution::*;
pub(crate) use execution_commands::*;
pub(crate) use execution_projection::*;
pub(crate) use execution_targets::*;
pub(crate) use match_reasons::*;
pub(crate) use package_reference::*;
pub(crate) use shell_tokens::*;
pub(crate) use workflow::*;
