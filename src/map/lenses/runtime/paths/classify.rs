// Responsibility: runtime-path-call-and-boundary-classification
use crate::map::{code_shape_without_literal_content, runtime_code_lines, unknown};
use crate::model::{MiddlewareOrGuardKind, Unknown};

pub(crate) fn invoked_target(body: &str, name: &str) -> bool {
    runtime_code_lines(body).into_iter().any(|(_, line)| {
        let code = code_shape_without_literal_content(&line);
        identifier_followed_by(&code, name, '(') || identifier_followed_by(&code, name, '.')
    })
}

pub(crate) fn middleware_or_guard_kind(body: &str, name: &str) -> Option<MiddlewareOrGuardKind> {
    let lower = name.to_ascii_lowercase();
    if validation_call(body, name) || lower.ends_with("schema") || lower.contains("validator") {
        return Some(MiddlewareOrGuardKind::Validation);
    }
    if lower.contains("middleware") || lower.starts_with("with") && lower.contains("context") {
        return Some(MiddlewareOrGuardKind::Middleware);
    }
    if lower.contains("guard")
        || lower.contains("permission")
        || lower.contains("authorize")
        || lower.contains("authenticate")
        || lower.starts_with("requiresession")
        || lower.starts_with("getsession")
        || lower.starts_with("assert")
        || lower.starts_with("ensuretenant")
    {
        return Some(MiddlewareOrGuardKind::Guard);
    }
    None
}

pub(crate) fn transformation_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [
        "respond",
        "response",
        "serialize",
        "project",
        "sanitize",
        "strip",
        "redact",
        "omit",
        "pick",
        "wrap",
        "normalize",
    ]
    .iter()
    .any(|part| lower.contains(part))
}

pub(crate) fn response_projection_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [
        "serialize",
        "project",
        "sanitize",
        "strip",
        "redact",
        "omit",
        "pick",
    ]
    .iter()
    .any(|part| lower.contains(part))
}

pub(crate) fn response_constructors(body: &str, line_offset: usize) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    for (line_number, line) in runtime_code_lines(body) {
        let code = code_shape_without_literal_content(&line);
        for constructor in [
            "NextResponse.json",
            "Response.json",
            "res.json",
            "reply.send",
            "ctx.json",
            "c.json",
        ] {
            if qualified_call(&code, constructor) {
                out.push((constructor.to_string(), line_offset + line_number));
            }
        }
    }
    out
}

fn qualified_call(code: &str, name: &str) -> bool {
    let needle = format!("{name}(");
    code.match_indices(&needle).any(|(start, _)| {
        start == 0
            || code[..start]
                .chars()
                .next_back()
                .is_some_and(|ch| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '.'))
    })
}

pub(crate) fn runtime_path_unknowns(rel: &str, body: &str, line_offset: usize) -> Vec<Unknown> {
    let mut out = Vec::new();
    for (line_number, line) in runtime_code_lines(body) {
        let code = code_shape_without_literal_content(&line);
        let line_number = line_offset + line_number;
        let boundary = if ["container.resolve(", "injector.resolve(", "container.get("]
            .iter()
            .any(|pattern| code.contains(pattern))
        {
            Some((
                "runtime_di_boundary",
                "dependency injection target is selected by a runtime container",
            ))
        } else if code.contains("Reflect.") || code.contains("reflect(") {
            Some((
                "runtime_reflection_boundary",
                "reflection crosses the static runtime path boundary",
            ))
        } else if code.contains("](") || code.contains("]?.(") {
            Some((
                "runtime_dynamic_dispatch",
                "computed callable target crosses the static runtime path boundary",
            ))
        } else {
            None
        };
        if let Some((kind, reason)) = boundary {
            out.push(unknown(
                kind,
                Some(rel),
                Some(line_number),
                reason,
                "runtime path stops instead of choosing a handler or transformation",
                Some(format!("codemap cone {rel}")),
            ));
        }
    }
    out
}

pub(crate) fn explicitly_omitted_fields(body: &str) -> Vec<String> {
    let code = runtime_code_lines(body)
        .into_iter()
        .map(|(_, line)| code_shape_without_literal_content(&line))
        .collect::<Vec<_>>()
        .join("\n");
    let Some(parameter) = primary_parameter_name(&code) else {
        return Vec::new();
    };
    let referenced = owned_dotted_fields(&code, parameter);
    let returned = returned_primary_fields(&code, parameter);
    if returned.is_empty() {
        return Vec::new();
    }
    referenced
        .into_iter()
        .filter(|field| !returned.contains(field))
        .collect()
}

fn validation_call(body: &str, name: &str) -> bool {
    runtime_code_lines(body).into_iter().any(|(_, line)| {
        let code = code_shape_without_literal_content(&line);
        code.contains(&format!("{name}.safeParse(")) || code.contains(&format!("{name}.parse("))
    })
}

fn identifier_followed_by(code: &str, name: &str, expected: char) -> bool {
    crate::map::identifier_ranges(code, name)
        .any(|range| code[range.1..].trim_start().starts_with(expected))
}

fn primary_parameter_name(code: &str) -> Option<&str> {
    let start = code.find('(')? + 1;
    let tail = code[start..].trim_start();
    let parameter_len = tail
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '$')
        .count();
    (parameter_len > 0).then(|| &tail[..parameter_len])
}

fn owned_dotted_fields(code: &str, owner: &str) -> std::collections::BTreeSet<String> {
    let mut out = std::collections::BTreeSet::new();
    for (_, end) in crate::map::identifier_ranges(code, owner) {
        let tail = code[end..].trim_start();
        let Some(tail) = tail.strip_prefix("?.").or_else(|| tail.strip_prefix('.')) else {
            continue;
        };
        let field = tail
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
            .collect::<String>();
        if !field.is_empty() {
            out.insert(field);
        }
    }
    out
}

fn returned_primary_fields(code: &str, parameter: &str) -> std::collections::BTreeSet<String> {
    let aliases = primary_field_aliases(code, parameter);
    let mut returned = std::collections::BTreeSet::new();
    for (_, return_end) in crate::map::identifier_ranges(code, "return") {
        let open = return_end
            + code[return_end..]
                .chars()
                .take_while(|ch| ch.is_whitespace())
                .map(char::len_utf8)
                .sum::<usize>();
        if !code[open..].starts_with('{') {
            continue;
        }
        let Some(close) = matching_brace(code, open) else {
            continue;
        };
        for segment in top_level_comma_segments(&code[open + 1..close]) {
            returned.extend(owned_dotted_fields(segment, parameter));
            let value = top_level_value(segment).unwrap_or(segment).trim();
            for (alias, fields) in &aliases {
                if crate::map::identifier_ranges(value, alias).next().is_some() {
                    returned.extend(fields.iter().cloned());
                }
            }
        }
    }
    returned
}

fn primary_field_aliases(
    code: &str,
    parameter: &str,
) -> std::collections::BTreeMap<String, std::collections::BTreeSet<String>> {
    let mut aliases = std::collections::BTreeMap::new();
    for line in code.lines() {
        let Some((left, right)) = line.split_once('=') else {
            continue;
        };
        if right.starts_with('=') || left.ends_with(['!', '<', '>']) {
            continue;
        }
        let fields = owned_dotted_fields(right, parameter);
        if fields.is_empty() {
            continue;
        }
        let Some(name) = left
            .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'))
            .rfind(|part| !part.is_empty())
        else {
            continue;
        };
        aliases.insert(name.to_string(), fields);
    }
    aliases
}

fn top_level_value(segment: &str) -> Option<&str> {
    let mut round = 0usize;
    let mut square = 0usize;
    let mut curly = 0usize;
    for (offset, ch) in segment.char_indices() {
        match ch {
            '(' => round += 1,
            ')' => round = round.saturating_sub(1),
            '[' => square += 1,
            ']' => square = square.saturating_sub(1),
            '{' => curly += 1,
            '}' => curly = curly.saturating_sub(1),
            ':' if round == 0 && square == 0 && curly == 0 => {
                return Some(&segment[offset + 1..]);
            }
            _ => {}
        }
    }
    None
}

fn matching_brace(code: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, ch) in code[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(open + offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn top_level_comma_segments(value: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut round = 0usize;
    let mut square = 0usize;
    let mut curly = 0usize;
    for (offset, ch) in value.char_indices() {
        match ch {
            '(' => round += 1,
            ')' => round = round.saturating_sub(1),
            '[' => square += 1,
            ']' => square = square.saturating_sub(1),
            '{' => curly += 1,
            '}' => curly = curly.saturating_sub(1),
            ',' if round == 0 && square == 0 && curly == 0 => {
                segments.push(&value[start..offset]);
                start = offset + 1;
            }
            _ => {}
        }
    }
    segments.push(&value[start..]);
    segments
}
