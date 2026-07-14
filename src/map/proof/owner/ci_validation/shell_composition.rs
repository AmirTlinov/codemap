// Responsibility: map-proof-owner-ci-validation-shell-composition

pub(crate) fn ci_owner_command_is_shell_syntax_only(command: &str) -> bool {
    let command = command.trim();
    matches!(
        command,
        ")" | ";;" | ";&" | ";;&" | "then" | "do" | "else" | "fi" | "done" | "esac"
    ) || command.starts_with("--")
        || (command.ends_with(')') && !command.chars().any(char::is_whitespace))
}

pub(crate) fn ci_owner_command_has_unsupported_shell_control(command: &str) -> bool {
    let mut chars = command.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;
    let mut previous_was_escape = false;
    while let Some(ch) = chars.next() {
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
        if in_single || in_double {
            continue;
        }
        match ch {
            '\n' | '\r' | ';' | '|' | '`' | '$' | '>' | '<' => return true,
            '&' if chars.peek() == Some(&'&') => {
                chars.next();
            }
            '&' => return true,
            _ => {}
        }
    }
    false
}

pub(crate) fn ci_owner_command_has_unsupported_shell_composition(command: &str) -> bool {
    let and_count = command.match_indices("&&").count();
    if and_count == 0 {
        return false;
    }
    and_count != 1 || !ci_owner_safe_scoped_cd_composition(command)
}

fn ci_owner_safe_scoped_cd_composition(command: &str) -> bool {
    let Some((prefix, tail)) = command.split_once("&&") else {
        return false;
    };
    !tail.trim().is_empty() && ci_owner_safe_cd_prefix(prefix)
}

fn ci_owner_safe_cd_prefix(prefix: &str) -> bool {
    let prefix = prefix.trim();
    let Some(rest) = prefix.strip_prefix("cd ") else {
        return false;
    };
    let rest = rest.trim();
    let Some(path) = ci_owner_cd_path(rest) else {
        return false;
    };
    !path.is_empty()
        && path != "~"
        && !path.starts_with('/')
        && !path.starts_with("~/")
        && !path.starts_with('-')
        && !path.split('/').any(|part| part == "..")
}

fn ci_owner_cd_path(rest: &str) -> Option<String> {
    if let Some(quote) = rest.chars().next().filter(|ch| *ch == '\'' || *ch == '"') {
        return Some(rest.strip_prefix(quote)?.strip_suffix(quote)?.to_string());
    }
    if rest.chars().any(char::is_whitespace) {
        return None;
    }
    Some(rest.to_string())
}
