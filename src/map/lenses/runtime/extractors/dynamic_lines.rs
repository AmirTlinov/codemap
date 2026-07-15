// Responsibility: dynamic-runtime-line-detection
use crate::map::{
    code_shape_without_literal_content, go_route_has_methods_chain, go_route_method_in_chain,
    is_identifier_char, matching_close_paren, object_argument_range, object_field_literal,
    quoted_literal_at, route_like_receiver, static_route_methods, top_level_comma,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RoutePathGapKind {
    Concatenated,
    Dynamic,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RoutePathGap {
    pub kind: RoutePathGapKind,
    pub ordinal: usize,
}

pub(crate) fn dynamic_import_line(line: &str) -> bool {
    let code = code_shape_without_literal_content(line);
    let Some((tail, code_tail)) = named_call_tail(line, &code, "import") else {
        return false;
    };
    code_tail.starts_with('(')
        && quoted_literal_at(tail.trim_start().trim_start_matches('(')).is_none()
}

pub(crate) fn dynamic_require_line(line: &str) -> bool {
    let code = code_shape_without_literal_content(line);
    let Some((tail, code_tail)) = named_call_tail(line, &code, "require") else {
        return false;
    };
    code_tail.starts_with('(')
        && (code_tail.contains('+')
            || quoted_literal_at(tail.trim_start().trim_start_matches('(')).is_none())
}

/// Returns the opaque runtime-code construct on this line, if JavaScript can
/// compile a string/value into executable code that the static consumer scan
/// cannot inspect. This is deliberately query-independent: any symbol use can
/// be hidden behind these boundaries.
pub(crate) fn runtime_generated_code_line(line: &str) -> Option<&'static str> {
    let code = code_shape_without_literal_content(line);
    let eval_reference = crate::map::identifier_ranges(&code, "eval")
        .next()
        .is_some();
    let computed_eval_call = crate::map::quoted_literal_contents(line)
        .iter()
        .any(|literal| literal == "eval")
        && code.contains('[')
        && code.contains(']');
    if eval_reference || computed_eval_call {
        return Some("eval_generated_code");
    }
    if runtime_call_tail(line, &code, "Function")
        .is_some_and(|(_, code_tail)| code_tail.starts_with('('))
    {
        return Some("function_constructor_generated_code");
    }
    ["setTimeout", "setInterval"].into_iter().find_map(|name| {
        let (tail, code_tail) = runtime_call_tail(line, &code, name)?;
        if code_tail.starts_with('(')
            && quoted_literal_at(tail.trim_start().trim_start_matches('(')).is_some()
        {
            Some("string_timer_generated_code")
        } else {
            None
        }
    })
}

fn named_call_tail<'a>(line: &'a str, code: &'a str, name: &str) -> Option<(&'a str, &'a str)> {
    let mut offset = 0;
    while let Some(found) = code[offset..].find(name) {
        let start = offset + found;
        let end = start + name.len();
        if name_has_call_boundary(code, start, end) {
            return Some((&line[end..], code[end..].trim_start()));
        }
        offset = end;
    }
    None
}

fn runtime_call_tail<'a>(line: &'a str, code: &'a str, name: &str) -> Option<(&'a str, &'a str)> {
    let mut offset = 0;
    while let Some(found) = code[offset..].find(name) {
        let start = offset + found;
        let end = start + name.len();
        let before = code[..start].chars().next_back();
        let after = code[end..].chars().next();
        let valid_before = before.is_none_or(|ch| !is_identifier_char(ch) && ch != '$');
        let valid_after = after.is_none_or(|ch| !is_identifier_char(ch) && ch != '$');
        if valid_before && valid_after {
            return Some((&line[end..], code[end..].trim_start()));
        }
        offset = end;
    }
    None
}

fn name_has_call_boundary(code: &str, start: usize, end: usize) -> bool {
    let before = code[..start].chars().next_back();
    let after = code[end..].chars().next();
    let valid_before = before.is_none_or(|ch| !is_identifier_char(ch) && ch != '.' && ch != '$');
    let valid_after = after.is_none_or(|ch| !is_identifier_char(ch) && ch != '$');
    valid_before && valid_after
}

pub(crate) fn dynamic_env_lookup_line(line: &str) -> bool {
    let code = code_shape_without_literal_content(line);
    code.contains("process.env[")
        || code.contains("import.meta.env[")
        || dynamic_call_arg(line, &code, "Deno.env.get(")
        || dynamic_call_arg(line, &code, "std::env::var(")
        || dynamic_call_arg(line, &code, "env::var(")
        || dynamic_call_arg(line, &code, "os.getenv(")
        || dynamic_os_environ_lookup(line, &code)
}

fn dynamic_call_arg(line: &str, code: &str, call: &str) -> bool {
    let Some(start) = code.find(call) else {
        return false;
    };
    quoted_literal_at(&line[start + call.len()..]).is_none()
}

fn dynamic_os_environ_lookup(line: &str, code: &str) -> bool {
    let Some(start) = code.find("os.environ[") else {
        return false;
    };
    quoted_literal_at(&line[start + "os.environ[".len()..]).is_none()
}

pub(crate) fn route_string_concat_line(line: &str) -> bool {
    route_path_gaps(line)
        .iter()
        .any(|gap| gap.kind == RoutePathGapKind::Concatenated)
}

pub(crate) fn route_dynamic_path_line(line: &str) -> bool {
    !route_path_gaps(line).is_empty()
}

pub(crate) fn route_path_gaps(line: &str) -> Vec<RoutePathGap> {
    let code = code_shape_without_literal_content(line);
    let mut gaps = Vec::new();
    for method in static_route_methods() {
        let call = format!(".{method}(");
        let mut offset = 0;
        while let Some(found) = code[offset..].find(&call) {
            let start = offset + found;
            offset = start + call.len();
            if !route_like_receiver(&code[..start]) {
                continue;
            }
            let open_paren = start + call.len() - 1;
            let Some(close_paren) = matching_close_paren(&code, open_paren) else {
                gaps.push((start, RoutePathGapKind::Unsupported));
                continue;
            };
            let arg_start = open_paren + 1;
            let arg_end = top_level_comma(&code, arg_start, close_paren).unwrap_or(close_paren);
            let arg = line[arg_start..arg_end].trim();
            if quoted_literal_at(arg).is_some() {
                continue;
            }
            gaps.push((
                start,
                if arg.contains('+') || arg.contains("${") {
                    RoutePathGapKind::Concatenated
                } else {
                    RoutePathGapKind::Dynamic
                },
            ));
        }
    }
    let go_call = ".HandleFunc(";
    let mut offset = 0;
    while let Some(found) = code[offset..].find(go_call) {
        let start = offset + found;
        offset = start + go_call.len();
        let open_paren = start + go_call.len() - 1;
        let Some(close_paren) = matching_close_paren(&code, open_paren) else {
            gaps.push((start, RoutePathGapKind::Unsupported));
            continue;
        };
        let arg_start = open_paren + 1;
        let arg_end = top_level_comma(&code, arg_start, close_paren).unwrap_or(close_paren);
        let arg = line[arg_start..arg_end].trim();
        if quoted_literal_at(arg).is_none() {
            gaps.push((
                start,
                if arg.contains('+') || arg.contains("${") {
                    RoutePathGapKind::Concatenated
                } else {
                    RoutePathGapKind::Dynamic
                },
            ));
        }
    }
    gaps.sort_by_key(|(start, _)| *start);
    gaps.into_iter()
        .enumerate()
        .map(|(index, (_, kind))| RoutePathGap {
            kind,
            ordinal: index + 1,
        })
        .collect()
}

pub(crate) fn route_dynamic_method_line(line: &str) -> bool {
    route_dynamic_method_count(line) > 0
}

pub(crate) fn route_dynamic_method_count(line: &str) -> usize {
    let code = code_shape_without_literal_content(line);
    let mut count = go_dynamic_route_method_count(line, &code);
    let mut offset = 0;
    while let Some(found) = code[offset..].find('[') {
        let start = offset + found;
        if computed_route_method_receiver(&code[..start]) && code[start..].contains("](") {
            count += 1;
        }
        offset = start + 1;
    }
    count
}

fn computed_route_method_receiver(prefix: &str) -> bool {
    if route_like_receiver(prefix) {
        return true;
    }
    let prefix = prefix.trim_end();
    let Some(route_start) = prefix.rfind(".route(") else {
        return false;
    };
    if !route_like_receiver(&prefix[..route_start]) {
        return false;
    }
    let open_paren = route_start + ".route(".len() - 1;
    matching_close_paren(prefix, open_paren) == prefix.len().checked_sub(1)
}

fn go_dynamic_route_method_count(line: &str, code: &str) -> usize {
    let call = ".HandleFunc(";
    let mut count = 0;
    let mut offset = 0;
    while let Some(found) = code[offset..].find(call) {
        let start = offset + found;
        let open_paren = start + call.len() - 1;
        let Some(close) = matching_close_paren(code, open_paren) else {
            offset = start + call.len();
            continue;
        };
        offset = close + 1;
        if quoted_literal_at(line[open_paren + 1..close].trim_start()).is_some()
            && go_route_has_methods_chain(code, close + 1)
            && go_route_method_in_chain(line, code, close + 1).is_none()
        {
            count += 1;
        }
    }
    count
}

pub(crate) fn route_object_dynamic_count(line: &str) -> usize {
    let code = code_shape_without_literal_content(line);
    let call = ".route(";
    let mut count = 0;
    let mut offset = 0;
    while let Some(found) = code[offset..].find(call) {
        let start = offset + found;
        offset = start + call.len();
        if !route_like_receiver(&code[..start]) {
            continue;
        }
        let arg_start = start + call.len();
        let Some(close) = matching_close_paren(&code, arg_start - 1) else {
            count += 1;
            continue;
        };
        if let Some((object_start, object_end)) = object_argument_range(&code, arg_start) {
            let object_line = &line[object_start..object_end];
            let object_code = &code[object_start..object_end];
            if object_field_literal(object_line, object_code, "method").is_none()
                || object_field_literal(object_line, object_code, "url")
                    .or_else(|| object_field_literal(object_line, object_code, "path"))
                    .is_none()
            {
                count += 1;
            }
        } else if quoted_literal_at(line[arg_start..close].trim_start()).is_none() {
            count += 1;
        }
    }
    count
}

pub(crate) fn route_mount_unknown_kinds(line: &str) -> Vec<&'static str> {
    let code = code_shape_without_literal_content(line);
    let call = ".use(";
    let mut kinds = Vec::new();
    let mut offset = 0;
    while let Some(found) = code[offset..].find(call) {
        let start = offset + found;
        offset = start + call.len();
        if !route_like_receiver(&code[..start]) {
            continue;
        }
        let arg_start = start + call.len();
        let Some(close) = matching_close_paren(&code, arg_start - 1) else {
            kinds.push("route_mount_unresolved");
            continue;
        };
        let arg = line[arg_start..close].trim();
        if quoted_literal_at(arg).is_some_and(|path| path.starts_with('/')) {
            kinds.push("route_mount_prefix");
            continue;
        }
        let first_arg = arg.split(',').next().unwrap_or("").trim();
        if !first_arg.is_empty()
            && arg.contains(',')
            && (first_arg.contains('+')
                || first_arg.contains("${")
                || first_arg.to_ascii_lowercase().contains("prefix")
                || first_arg.to_ascii_lowercase().contains("path")
                || first_arg.to_ascii_lowercase().contains("route"))
        {
            kinds.push("route_mount_dynamic_prefix");
            continue;
        }
        let lower = first_arg.to_ascii_lowercase();
        if !first_arg.is_empty()
            && !arg.contains(',')
            && first_arg.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '$')
            })
            && ["router", "route", "app", "api"]
                .iter()
                .any(|token| lower.contains(token))
        {
            kinds.push("route_mount_target");
        }
    }
    kinds
}
