fn javascript_route_registrations(rel: &str, line: &str, line_number: usize) -> Vec<RuntimeRoute> {
    let code = code_shape_without_literal_content(line);
    let mut routes = Vec::new();
    for method in static_route_methods() {
        for start in top_level_receiver_call_offsets(&code, &format!(".{method}(")) {
            let call = format!(".{method}(");
            let arg_start = start + call.len();
            let Some(path) = quoted_literal_at(line[arg_start..].trim_start()) else {
                continue;
            };
            routes.push(RuntimeRoute {
                method: Some(method.to_ascii_uppercase()),
                path,
                file: rel.to_string(),
                handler_symbol: route_call_second_arg_identifier(line, &code, arg_start),
                evidence: "javascript_route_registration".to_string(),
                strength: EvidenceStrength::High,
                locations: vec![EvidenceLocation::line(
                    rel,
                    line_number,
                    "route_registration",
                )],
            });
        }
    }
    routes.extend(javascript_chained_route_registrations(
        rel,
        line,
        line_number,
    ));
    routes.extend(javascript_object_route_registrations(
        rel,
        line,
        line_number,
    ));
    routes
}

fn top_level_receiver_call_offsets(code: &str, call: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut index = 0;
    while index < code.len() {
        if depth == 0 && code[index..].starts_with(call) && route_like_receiver(&code[..index]) {
            out.push(index);
            depth += 1;
            index += call.len();
            continue;
        }
        let Some(ch) = code[index..].chars().next() else {
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

fn javascript_chained_route_registrations(
    rel: &str,
    line: &str,
    line_number: usize,
) -> Vec<RuntimeRoute> {
    let code = code_shape_without_literal_content(line);
    let mut routes = Vec::new();
    let call = ".route(";
    for start in top_level_receiver_call_offsets(&code, call) {
        let arg_start = start + call.len();
        let Some(path) = quoted_literal_at(&line[arg_start..]) else {
            continue;
        };
        let Some(close) = code[arg_start..].find(')') else {
            continue;
        };
        let chain = route_chain_segment(&code[arg_start + close + 1..]);
        for method in static_route_methods() {
            if route_chain_has_top_level_method(chain, method) {
                routes.push(RuntimeRoute {
                    method: Some(method.to_ascii_uppercase()),
                    path: path.clone(),
                    file: rel.to_string(),
                    handler_symbol: route_chain_method_handler_identifier(
                        line,
                        &code,
                        arg_start + close + 1,
                        method,
                    ),
                    evidence: "javascript_route_chain_registration".to_string(),
                    strength: EvidenceStrength::High,
                    locations: vec![EvidenceLocation::line(
                        rel,
                        line_number,
                        "route_registration",
                    )],
                });
            }
        }
    }
    routes
}

fn route_chain_segment(after_route_call: &str) -> &str {
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

fn route_chain_has_top_level_method(chain: &str, method: &str) -> bool {
    let method_call = format!(".{method}(");
    top_level_chain_call_offset(chain, &method_call).is_some()
}

fn top_level_chain_call_offset(chain: &str, call: &str) -> Option<usize> {
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

fn javascript_object_route_registrations(
    rel: &str,
    line: &str,
    line_number: usize,
) -> Vec<RuntimeRoute> {
    let code = code_shape_without_literal_content(line);
    let mut routes = Vec::new();
    let call = ".route(";
    for start in top_level_receiver_call_offsets(&code, call) {
        let arg_start = start + call.len();
        let Some((object_start, object_end)) = object_argument_range(&code, arg_start) else {
            continue;
        };
        let object_line = &line[object_start..object_end];
        let object_code = &code[object_start..object_end];
        let Some(method) = object_field_literal(object_line, object_code, "method") else {
            continue;
        };
        let Some(path) = object_field_literal(object_line, object_code, "url")
            .or_else(|| object_field_literal(object_line, object_code, "path"))
        else {
            continue;
        };
        routes.push(RuntimeRoute {
            method: Some(method.to_ascii_uppercase()),
            path,
            file: rel.to_string(),
            handler_symbol: object_field_identifier(object_line, object_code, "handler"),
            evidence: "javascript_route_object_registration".to_string(),
            strength: EvidenceStrength::High,
            locations: vec![EvidenceLocation::line(
                rel,
                line_number,
                "route_registration",
            )],
        });
    }
    routes
}

fn python_route_decorators(rel: &str, line: &str, line_number: usize) -> Vec<RuntimeRoute> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('@') {
        return Vec::new();
    }
    static_route_methods()
        .iter()
        .filter_map(|method| {
            let call = format!(".{method}(");
            let start = trimmed.find(&call)?;
            let receiver_prefix = &trimmed[..start];
            if !python_decorator_receiver_is_local(receiver_prefix)
                || !route_like_receiver(receiver_prefix)
            {
                return None;
            }
            let path = quoted_literal_at(trimmed[start + call.len()..].trim_start())?;
            Some(RuntimeRoute {
                method: Some(method.to_ascii_uppercase()),
                path,
                file: rel.to_string(),
                handler_symbol: None,
                evidence: "python_route_decorator".to_string(),
                strength: EvidenceStrength::High,
                locations: vec![EvidenceLocation::line(rel, line_number, "route_decorator")],
            })
        })
        .collect()
}

fn python_decorator_receiver_is_local(prefix: &str) -> bool {
    !prefix
        .chars()
        .any(|ch| matches!(ch, '(' | '[' | '{' | ','))
}

fn go_route_registrations(rel: &str, line: &str, line_number: usize) -> Vec<RuntimeRoute> {
    let code = code_shape_without_literal_content(line);
    let Some(start) = code
        .find("http.HandleFunc(")
        .or_else(|| code.find(".HandleFunc("))
    else {
        return Vec::new();
    };
    let Some(path) =
        quoted_literal_at(line[start..].split_once('(').map(|(_, tail)| tail).unwrap_or(""))
    else {
        return Vec::new();
    };
    let Some(open_paren) = code[start..].find('(').map(|found| start + found) else {
        return Vec::new();
    };
    let Some(close) = matching_close_paren(&code, open_paren) else {
        return Vec::new();
    };
    let method = if let Some(method) = go_route_method_in_chain(line, &code, close + 1) {
        method
    } else if go_route_has_methods_chain(&code, close + 1) {
        return Vec::new();
    } else {
        "ANY".to_string()
    };
    vec![RuntimeRoute {
        method: Some(method),
        path,
        file: rel.to_string(),
        handler_symbol: route_call_second_arg_identifier(line, &code, open_paren + 1),
        evidence: "go_http_route_registration".to_string(),
        strength: EvidenceStrength::High,
        locations: vec![EvidenceLocation::line(rel, line_number, "route_registration")],
    }]
}

fn object_field_literal(line: &str, code: &str, field: &str) -> Option<String> {
    let value_start = object_field_value_start(line, code, field)?;
    quoted_literal_at(&line[value_start..])
}

fn object_field_value_start(line: &str, code: &str, field: &str) -> Option<usize> {
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

fn is_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'
}

fn object_argument_range(code: &str, arg_start: usize) -> Option<(usize, usize)> {
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

fn go_route_method_in_chain(line: &str, code: &str, chain_start: usize) -> Option<String> {
    let chain = route_chain_segment(&code[chain_start..]);
    let call = ".Methods(";
    let start = top_level_chain_call_offset(chain, call)?;
    quoted_literal_at(&line[chain_start + start + call.len()..])
        .map(|method| method.to_ascii_uppercase())
}

fn go_route_has_methods_chain(code: &str, chain_start: usize) -> bool {
    let chain = route_chain_segment(&code[chain_start..]);
    top_level_chain_call_offset(chain, ".Methods(").is_some()
}

fn matching_close_paren(code: &str, open_paren: usize) -> Option<usize> {
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

fn static_route_methods() -> &'static [&'static str] {
    &["get", "post", "put", "patch", "delete", "all", "head", "options"]
}

fn route_like_receiver(prefix: &str) -> bool {
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
