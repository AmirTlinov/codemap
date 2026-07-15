// Responsibility: dynamic-runtime-line-detection
use crate::map::{
    code_shape_without_literal_content, go_route_has_methods_chain, go_route_method_in_chain,
    is_identifier_char, matching_close_paren, object_argument_range, object_field_literal,
    quoted_literal_at, route_like_receiver, static_route_methods,
};

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
    let code = code_shape_without_literal_content(line);
    static_route_methods().iter().any(|method| {
        let call = format!(".{method}(");
        code.find(&call).is_some_and(|start| {
            if !route_like_receiver(&code[..start]) {
                return false;
            }
            let arg = line[start + call.len()..].trim_start();
            quoted_literal_at(arg).is_none() && (arg.contains('+') || arg.contains("${"))
        })
    })
}

pub(crate) fn route_dynamic_path_line(line: &str) -> bool {
    let code = code_shape_without_literal_content(line);
    static_route_methods().iter().any(|method| {
        let call = format!(".{method}(");
        code.find(&call).is_some_and(|start| {
            if !route_like_receiver(&code[..start]) {
                return false;
            }
            let arg = line[start + call.len()..].trim_start();
            quoted_literal_at(arg).is_none()
        })
    })
}

pub(crate) fn route_dynamic_method_line(line: &str) -> bool {
    let code = code_shape_without_literal_content(line);
    if go_dynamic_route_method_line(line, &code) {
        return true;
    }
    let mut offset = 0;
    while let Some(found) = code[offset..].find('[') {
        let start = offset + found;
        if route_like_receiver(&code[..start])
            && code[start..]
                .find("](")
                .is_some_and(|close| quoted_literal_at(&line[start + close + 2..]).is_some())
        {
            return true;
        }
        offset = start + 1;
    }
    false
}

fn go_dynamic_route_method_line(line: &str, code: &str) -> bool {
    let Some(start) = code
        .find("http.HandleFunc(")
        .or_else(|| code.find(".HandleFunc("))
    else {
        return false;
    };
    if quoted_literal_at(
        line[start..]
            .split_once('(')
            .map(|(_, tail)| tail)
            .unwrap_or(""),
    )
    .is_none()
    {
        return false;
    }
    let Some(open_paren) = code[start..].find('(').map(|found| start + found) else {
        return false;
    };
    let Some(close) = matching_close_paren(code, open_paren) else {
        return false;
    };
    go_route_has_methods_chain(code, close + 1)
        && go_route_method_in_chain(line, code, close + 1).is_none()
}

pub(crate) fn route_object_dynamic_line(line: &str) -> bool {
    let code = code_shape_without_literal_content(line);
    let call = ".route(";
    let Some(start) = code.find(call) else {
        return false;
    };
    let arg_start = start + call.len();
    if !route_like_receiver(&code[..start]) {
        return false;
    }
    let Some((object_start, object_end)) = object_argument_range(&code, arg_start) else {
        return false;
    };
    let object_line = &line[object_start..object_end];
    let object_code = &code[object_start..object_end];
    object_field_literal(object_line, object_code, "method").is_none()
        || object_field_literal(object_line, object_code, "url")
            .or_else(|| object_field_literal(object_line, object_code, "path"))
            .is_none()
}

pub(crate) fn route_mount_prefix_unknown_kind(line: &str) -> Option<&'static str> {
    let code = code_shape_without_literal_content(line);
    let call = ".use(";
    let start = code.find(call)?;
    let arg_start = start + call.len();
    if !route_like_receiver(&code[..start]) {
        return None;
    }
    let arg = line[arg_start..].trim_start();
    if quoted_literal_at(arg).is_some_and(|path| path.starts_with('/')) {
        return Some("route_mount_prefix");
    }
    let first_arg = arg.split([',', ')']).next()?.trim();
    if !first_arg.is_empty()
        && arg.contains(',')
        && (first_arg.contains('+')
            || first_arg.contains("${")
            || first_arg.to_ascii_lowercase().contains("prefix")
            || first_arg.to_ascii_lowercase().contains("path")
            || first_arg.to_ascii_lowercase().contains("route"))
    {
        return Some("route_mount_dynamic_prefix");
    }
    None
}
