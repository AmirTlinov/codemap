fn rust_axum_routes_from_text(rel: &str, text: &str) -> Vec<RuntimeRoute> {
    if !rust_axum_route_context(text) {
        return Vec::new();
    }
    let mut routes = Vec::new();
    let mut rust_axum_chain_continuation = false;
    for (line_number, line) in runtime_code_lines(text) {
        let (line_routes, chain_continues) = rust_axum_route_registrations(
            rel,
            &line,
            line_number,
            rust_axum_chain_continuation,
        );
        routes.extend(line_routes);
        rust_axum_chain_continuation =
            chain_continues || rust_axum_router_new_on_line(&line);
    }
    routes
}

fn rust_axum_route_context(text: &str) -> bool {
    text.contains("use axum") || text.contains("axum::") || text.contains("Router::new()")
}

fn rust_axum_route_registrations(
    rel: &str,
    line: &str,
    line_number: usize,
    allow_leading_continuation: bool,
) -> (Vec<RuntimeRoute>, bool) {
    let code = code_shape_without_literal_content(line);
    let mut routes = Vec::new();
    let mut chain_continues = false;
    let mut allow_continuation = allow_leading_continuation;
    let call = ".route(";
    let mut offset = 0;
    while let Some(found) = code[offset..].find(call) {
        let start = offset + found;
        if !rust_axum_route_receiver_allowed(&code, start, allow_continuation) {
            offset = start + call.len();
            continue;
        }
        chain_continues = true;
        allow_continuation = true;
        let open_paren = start + call.len() - 1;
        let Some(close_paren) = matching_close_paren(&code, open_paren) else {
            offset = start + call.len();
            continue;
        };
        let arg_start = open_paren + 1;
        let Some(path) = quoted_literal_at(line[arg_start..].trim_start()) else {
            offset = close_paren + 1;
            continue;
        };
        let Some(comma) = top_level_comma(&code, arg_start, close_paren) else {
            offset = close_paren + 1;
            continue;
        };
        for (method, handler_symbol) in rust_axum_method_handlers(line, &code, comma + 1, close_paren)
        {
            routes.push(RuntimeRoute {
                method: Some(method),
                path: path.clone(),
                file: rel.to_string(),
                handler_symbol,
                evidence: "rust_axum_route_registration".to_string(),
                strength: EvidenceStrength::High,
                locations: vec![EvidenceLocation::line(
                    rel,
                    line_number,
                    "route_registration",
                )],
            });
        }
        offset = close_paren + 1;
    }
    (routes, chain_continues)
}

fn rust_axum_route_receiver_allowed(
    code: &str,
    route_call_start: usize,
    allow_leading_continuation: bool,
) -> bool {
    let prefix = code[..route_call_start].trim_end();
    (allow_leading_continuation && prefix.is_empty()) || rust_axum_router_new_ends_prefix(prefix)
}

fn rust_axum_router_new_on_line(line: &str) -> bool {
    let code = code_shape_without_literal_content(line);
    let mut offset = 0;
    while let Some(found) = code[offset..].find("Router::new()") {
        let start = offset + found;
        if rust_axum_router_new_has_left_boundary(&code, start) {
            return true;
        }
        offset = start + "Router::new()".len();
    }
    false
}

fn rust_axum_router_new_ends_prefix(prefix: &str) -> bool {
    let prefix = prefix.trim_end();
    let needle = "Router::new()";
    let Some(start) = prefix.len().checked_sub(needle.len()) else {
        return false;
    };
    prefix.ends_with(needle) && rust_axum_router_new_has_left_boundary(prefix, start)
}

fn rust_axum_router_new_has_left_boundary(code: &str, start: usize) -> bool {
    code[..start]
        .chars()
        .next_back()
        .is_none_or(|ch| !is_identifier_char(ch))
}

fn rust_axum_method_handlers(
    line: &str,
    code: &str,
    start: usize,
    end: usize,
) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut index = start;
    while index < end {
        if depth == 0
            && let Some((method, open_paren)) = rust_axum_method_call_at(code, index, end)
            && let Some(close_paren) = matching_close_paren(code, open_paren)
        {
            out.push((
                rust_axum_method_name(method),
                simple_identifier_argument(line, code, open_paren + 1, close_paren),
            ));
            index = close_paren + 1;
            continue;
        }
        let Some(ch) = code[index..end].chars().next() else {
            break;
        };
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
        index += ch.len_utf8();
    }
    out
}

fn rust_axum_method_call_at(code: &str, index: usize, end: usize) -> Option<(&'static str, usize)> {
    for method in rust_axum_route_methods() {
        let direct_call = format!("{method}(");
        if code[index..end].starts_with(&direct_call)
            && rust_method_call_has_left_boundary(code, index)
        {
            return Some((method, index + method.len()));
        }
        let chained_call = format!(".{method}(");
        if code[index..end].starts_with(&chained_call) {
            return Some((method, index + method.len() + 1));
        }
    }
    None
}

fn rust_method_call_has_left_boundary(code: &str, index: usize) -> bool {
    code[..index]
        .chars()
        .next_back()
        .is_none_or(|ch| !is_identifier_char(ch) && ch != ':')
}

fn rust_axum_route_methods() -> &'static [&'static str] {
    &[
        "get", "post", "put", "patch", "delete", "head", "options", "any",
    ]
}

fn rust_axum_method_name(method: &str) -> String {
    if method == "any" {
        "ANY".to_string()
    } else {
        method.to_ascii_uppercase()
    }
}
