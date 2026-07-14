// Responsibility: route-shape-scanning
use crate::map::quoted_literal_at;

pub(crate) fn route_chain_segment(after_route_call: &str) -> &str {
    let start = after_route_call
        .chars()
        .take_while(|ch| ch.is_whitespace())
        .map(char::len_utf8)
        .sum::<usize>();
    if !after_route_call[start..].starts_with('.') {
        return "";
    }
    let mut depth = 0usize;
    let mut index = start;
    while index < after_route_call.len() {
        if depth == 0
            && (after_route_call[index..].starts_with(';')
                || after_route_call[index..].starts_with("&&")
                || after_route_call[index..].starts_with("||")
                || after_route_call[index..].starts_with(',')
                || after_route_call[index..].starts_with('?')
                || after_route_call[index..].starts_with(':')
                || (index > start && after_route_call[index..].starts_with(".route(")))
        {
            break;
        }
        let Some(ch) = after_route_call[index..].chars().next() else {
            break;
        };
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
        index += ch.len_utf8();
    }
    &after_route_call[start..index]
}

pub(crate) fn route_chain_has_top_level_method(chain: &str, method: &str) -> bool {
    let method_call = format!(".{method}(");
    top_level_chain_call_offset(chain, &method_call).is_some()
}

pub(crate) fn top_level_chain_call_offset(chain: &str, call: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut index = 0;
    while index < chain.len() {
        if depth == 0 && chain[index..].starts_with(call) {
            return Some(index);
        }
        let Some(ch) = chain[index..].chars().next() else {
            break;
        };
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
        index += ch.len_utf8();
    }
    None
}

pub(crate) fn object_field_literal(line: &str, code: &str, field: &str) -> Option<String> {
    let value_start = object_field_value_start(line, code, field)?;
    quoted_literal_at(&line[value_start..])
}

pub(crate) fn object_field_value_start(line: &str, code: &str, field: &str) -> Option<usize> {
    let keys = [
        field.to_string(),
        format!("\"{field}\""),
        format!("'{field}'"),
    ];
    let mut depth = 0usize;
    let mut index = 0;
    while index < code.len() {
        let Some(ch) = code[index..].chars().next() else {
            break;
        };
        if ch == '{' {
            depth += 1;
            index += ch.len_utf8();
            continue;
        }
        if ch == '}' {
            depth = depth.saturating_sub(1);
            index += ch.len_utf8();
            continue;
        }
        if depth == 1 {
            for key in &keys {
                if let Some(value_start) =
                    object_field_value_start_after_key(line, code, index, key)
                {
                    return Some(value_start);
                }
            }
        }
        index += ch.len_utf8();
    }
    None
}

fn object_field_value_start_after_key(
    line: &str,
    code: &str,
    index: usize,
    key: &str,
) -> Option<usize> {
    if !line[index..].starts_with(key) {
        return None;
    }
    let after_key = index + key.len();
    if !key.starts_with('"')
        && !key.starts_with('\'')
        && index > 0
        && line[..index]
            .chars()
            .next_back()
            .is_some_and(is_identifier_char)
    {
        return None;
    }
    if !key.starts_with('"')
        && !key.starts_with('\'')
        && line[after_key..]
            .chars()
            .next()
            .is_some_and(is_identifier_char)
    {
        return None;
    }
    let after_spaces = after_key
        + code[after_key..]
            .chars()
            .take_while(|ch| ch.is_whitespace())
            .map(char::len_utf8)
            .sum::<usize>();
    code[after_spaces..]
        .starts_with(':')
        .then_some(after_spaces + 1)
}

pub(crate) fn is_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'
}

pub(crate) fn object_argument_range(code: &str, arg_start: usize) -> Option<(usize, usize)> {
    let leading_spaces = code[arg_start..]
        .chars()
        .take_while(|ch| ch.is_whitespace())
        .map(char::len_utf8)
        .sum::<usize>();
    let object_start = arg_start + leading_spaces;
    if !code[object_start..].starts_with('{') {
        return None;
    }
    let mut depth = 0usize;
    for (relative, ch) in code[object_start..].char_indices() {
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some((object_start, object_start + relative + ch.len_utf8()));
            }
        }
    }
    None
}

pub(crate) fn matching_close_paren(code: &str, open_paren: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (relative, ch) in code[open_paren..].char_indices() {
        if ch == '(' {
            depth += 1;
        } else if ch == ')' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(open_paren + relative);
            }
        }
    }
    None
}

pub(crate) fn static_route_methods() -> &'static [&'static str] {
    &[
        "get", "post", "put", "patch", "delete", "all", "head", "options",
    ]
}

pub(crate) fn route_like_receiver(prefix: &str) -> bool {
    let receiver = prefix
        .trim_end()
        .chars()
        .rev()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '$')
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    let receiver = receiver.to_ascii_lowercase();
    receiver == "app"
        || receiver == "api"
        || receiver == "router"
        || receiver == "server"
        || receiver == "fastify"
        || receiver.ends_with("router")
        || receiver.ends_with("server")
}
