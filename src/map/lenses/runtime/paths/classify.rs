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
    let referenced = dotted_fields(&code);
    let returned = returned_object_fields(&code);
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

fn dotted_fields(code: &str) -> std::collections::BTreeSet<String> {
    let mut out = std::collections::BTreeSet::new();
    for part in code.split('.').skip(1) {
        let field = part
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
            .collect::<String>();
        if !field.is_empty() {
            out.insert(field);
        }
    }
    out
}

fn returned_object_fields(code: &str) -> std::collections::BTreeSet<String> {
    let Some(start) = code.find("return {") else {
        return std::collections::BTreeSet::new();
    };
    let tail = &code[start + "return {".len()..];
    let Some(end) = tail.find('}') else {
        return std::collections::BTreeSet::new();
    };
    tail[..end]
        .split(',')
        .filter_map(|part| {
            let name = part
                .trim()
                .trim_start_matches("...")
                .split([':', ' ', '\n'])
                .next()?;
            (!name.is_empty()
                && name
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_'))
            .then(|| name.to_string())
        })
        .collect()
}
