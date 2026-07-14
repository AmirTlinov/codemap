// Responsibility: map-proof-owner-ci-parse
use crate::map::CiRunStep;

pub(crate) fn shell_heredoc_delimiter(command: &str) -> Option<String> {
    command.split_whitespace().find_map(|token| {
        let delimiter = token.strip_prefix("<<")?.trim_start_matches('-');
        if delimiter.is_empty() {
            return None;
        }
        Some(
            delimiter
                .trim_matches(|ch| matches!(ch, '"' | '\''))
                .to_string(),
        )
    })
}

pub(crate) fn push_ci_logical_command(
    out: &mut Vec<CiRunStep>,
    pending: &mut Option<CiRunStep>,
    command: String,
    line: usize,
) {
    let continues = command_has_shell_continuation(&command);
    let command = command_without_shell_continuation(&command);
    match pending {
        Some(current) => {
            if !command.is_empty() {
                if !current.command.is_empty() {
                    current.command.push(' ');
                }
                current.command.push_str(&command);
            }
            if !continues {
                flush_ci_pending_command(out, pending);
            }
        }
        None if continues => {
            *pending = Some(CiRunStep { command, line });
        }
        None if !command.is_empty() => {
            push_ci_run_steps(out, command, line);
        }
        None => {}
    }
}

pub(crate) fn flush_ci_pending_command(out: &mut Vec<CiRunStep>, pending: &mut Option<CiRunStep>) {
    if let Some(step) = pending.take()
        && !step.command.trim().is_empty()
    {
        push_ci_run_steps(out, step.command, step.line);
    }
}

pub(crate) fn push_ci_run_steps(out: &mut Vec<CiRunStep>, command: String, line: usize) {
    let Some(parts) = split_ci_shell_and_commands(&command) else {
        out.push(CiRunStep { command, line });
        return;
    };
    for part in parts {
        out.push(CiRunStep {
            command: part,
            line,
        });
    }
}

fn split_ci_shell_and_commands(command: &str) -> Option<Vec<String>> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut previous_was_escape = false;
    let mut chars = command.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        if previous_was_escape {
            previous_was_escape = false;
            continue;
        }
        if ch == '\\' && !in_single {
            previous_was_escape = true;
            continue;
        }
        if ch == '\'' && !in_double {
            in_single = !in_single;
            continue;
        }
        if ch == '"' && !in_single {
            in_double = !in_double;
            continue;
        }
        if ch == '&'
            && !in_single
            && !in_double
            && chars.peek().is_some_and(|(_, next)| *next == '&')
        {
            chars.next();
            let part = command[start..index].trim();
            if part.is_empty() {
                return None;
            }
            parts.push(part.to_string());
            start = index + 2;
        }
    }
    if parts.is_empty() {
        return None;
    }
    let tail = command[start..].trim();
    if tail.is_empty() {
        return None;
    }
    parts.push(tail.to_string());
    if let Some(cd_prefix) = ci_safe_cd_prefix(&parts[0]) {
        if parts.len() == 2 {
            return None;
        }
        return Some(
            parts
                .iter()
                .skip(1)
                .map(|part| format!("{cd_prefix} && {part}"))
                .collect(),
        );
    }
    if parts[0].trim_start().starts_with("cd ") {
        return None;
    }
    Some(parts)
}

fn ci_safe_cd_prefix(command: &str) -> Option<String> {
    let command = command.trim();
    let rest = command.strip_prefix("cd ")?.trim();
    let path = if let Some(quote) = rest.chars().next().filter(|ch| *ch == '\'' || *ch == '"') {
        rest.strip_prefix(quote)?.strip_suffix(quote)?.to_string()
    } else {
        if rest.chars().any(char::is_whitespace) {
            return None;
        }
        rest.to_string()
    };
    if path.is_empty()
        || path == "~"
        || path.starts_with('/')
        || path.starts_with("~/")
        || path.starts_with('-')
        || path.split('/').any(|part| part == "..")
    {
        return None;
    }
    Some(command.to_string())
}

fn command_has_shell_continuation(command: &str) -> bool {
    command.trim_end().ends_with('\\')
}

fn command_without_shell_continuation(command: &str) -> String {
    command
        .trim_end()
        .strip_suffix('\\')
        .unwrap_or_else(|| command.trim_end())
        .trim()
        .to_string()
}
