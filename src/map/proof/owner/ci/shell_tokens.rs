// Responsibility: map-proof-owner-ci-shell-tokens
use crate::map::{code_shape_without_literal_content, find_all, quoted_literal_at};

pub(crate) fn command_tokens(command: &str) -> Vec<String> {
    command
        .split(|ch: char| {
            !(ch.is_ascii_alphanumeric()
                || matches!(ch, '_' | '-' | ':' | '@' | '/' | '.' | '=' | '*'))
        })
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

pub(crate) fn strip_inline_shell_comment(line: &str) -> String {
    let mut in_single = false;
    let mut in_double = false;
    let mut previous_was_escape = false;
    let mut previous_was_whitespace = true;
    for (index, ch) in line.char_indices() {
        if previous_was_escape {
            previous_was_escape = false;
            previous_was_whitespace = ch.is_whitespace();
            continue;
        }
        if ch == '\\' && !in_single {
            previous_was_escape = true;
            previous_was_whitespace = false;
            continue;
        }
        if ch == '\'' && !in_double {
            in_single = !in_single;
            previous_was_whitespace = false;
            continue;
        }
        if ch == '"' && !in_single {
            in_double = !in_double;
            previous_was_whitespace = false;
            continue;
        }
        if ch == '#' && !in_single && !in_double && previous_was_whitespace {
            return line[..index].trim_end().to_string();
        }
        previous_was_whitespace = ch.is_whitespace();
    }
    line.to_string()
}

pub(crate) fn prisma_env_names(line: &str) -> Vec<String> {
    let code = code_shape_without_literal_content(line);
    let mut out = Vec::new();
    for start in find_all(&code, "env(") {
        if let Some(name) = quoted_literal_at(&line[start + "env(".len()..]) {
            out.push(name);
        }
    }
    out
}
