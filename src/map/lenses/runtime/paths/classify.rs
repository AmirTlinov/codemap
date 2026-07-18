// Responsibility: runtime-path-call-and-boundary-classification
use crate::map::{code_shape_without_literal_content, runtime_code_lines, unknown};
use crate::model::{MiddlewareOrGuardKind, Unknown};

pub(crate) fn invoked_target(body: &str, name: &str) -> bool {
    runtime_code_lines(body).into_iter().any(|(_, line)| {
        let code = code_shape_without_literal_content(&line);
        identifier_followed_by(&code, name, '(') || identifier_followed_by(&code, name, '.')
    })
}

pub(crate) fn middleware_or_guard_kind(
    body: &str,
    name: &str,
    on_guard_chain: bool,
) -> Option<MiddlewareOrGuardKind> {
    let lower = name.to_ascii_lowercase();
    if validation_call(body, name) || lower.ends_with("schema") || lower.contains("validator") {
        return Some(MiddlewareOrGuardKind::Validation);
    }
    let kind =
        if lower.contains("middleware") || lower.starts_with("with") && lower.contains("context") {
            MiddlewareOrGuardKind::Middleware
        } else if lower.contains("guard")
            || lower.contains("csrf")
            || lower.contains("permission")
            || lower.contains("authorize")
            || lower.contains("authenticate")
            || lower.contains("security")
            || lower.contains("session")
            || lower.starts_with("requiresession")
            || lower.starts_with("getsession")
            || lower.starts_with("assert")
            || lower.starts_with("ensuretenant")
        {
            MiddlewareOrGuardKind::Guard
        } else {
            return None;
        };
    (call_wraps_protected_handler(body, name) || call_is_awaited(body, name, on_guard_chain))
        .then_some(kind)
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

fn validation_call(body: &str, name: &str) -> bool {
    runtime_code_lines(body).into_iter().any(|(_, line)| {
        let code = code_shape_without_literal_content(&line);
        code.contains(&format!("{name}.safeParse(")) || code.contains(&format!("{name}.parse("))
    })
}

fn call_wraps_protected_handler(body: &str, name: &str) -> bool {
    call_sources(body, name).into_iter().any(|(source, start)| {
        let before = &body[..start];
        let returned = before
            .rsplit([';', '\n', '{', '}'])
            .next()
            .is_some_and(|prefix| prefix.trim_start().starts_with("return "));
        let callback_argument = crate::repo::js_top_level_arguments(source)
            .into_iter()
            .skip(1)
            .any(|argument| {
                matches!(
                    argument.trim().to_ascii_lowercase().as_str(),
                    "handler" | "next" | "callback"
                )
            });
        returned && (source.contains("=>") || source.contains("function") || callback_argument)
    })
}

fn call_is_awaited(body: &str, name: &str, on_guard_chain: bool) -> bool {
    call_sources(body, name).into_iter().any(|(_, start)| {
        let prefix = body[..start]
            .rsplit([';', '\n', '{', '}'])
            .next()
            .unwrap_or_default();
        prefix.split_whitespace().any(|part| part == "await")
            && (on_guard_chain || prefix.contains("return") || prefix.contains('='))
    })
}

fn call_sources<'a>(body: &'a str, name: &str) -> Vec<(&'a str, usize)> {
    let mut out = Vec::new();
    for (start, end) in crate::map::identifier_ranges(body, name) {
        let after = end
            + body[end..]
                .len()
                .saturating_sub(body[end..].trim_start().len());
        if body.as_bytes().get(after) != Some(&b'(') {
            continue;
        }
        if let Some(call_end) = crate::repo::js_balanced_call_end(body, after) {
            out.push((&body[start..call_end], start));
        }
    }
    out
}

fn identifier_followed_by(code: &str, name: &str, expected: char) -> bool {
    crate::map::identifier_ranges(code, name)
        .any(|range| code[range.1..].trim_start().starts_with(expected))
}
