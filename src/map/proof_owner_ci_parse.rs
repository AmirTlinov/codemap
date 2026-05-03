fn shell_heredoc_delimiter(command: &str) -> Option<String> {
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

fn push_ci_logical_command(
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
            out.push(CiRunStep { command, line });
        }
        None => {}
    }
}

fn flush_ci_pending_command(out: &mut Vec<CiRunStep>, pending: &mut Option<CiRunStep>) {
    if let Some(step) = pending.take()
        && !step.command.trim().is_empty()
    {
        out.push(step);
    }
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
