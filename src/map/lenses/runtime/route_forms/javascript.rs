// Responsibility: javascript-route-registrations
use crate::map::{
    code_shape_without_literal_content, object_argument_range, object_field_identifier,
    object_field_literal, quoted_literal_at, route_call_handler_and_middleware_identifiers,
    route_chain_has_top_level_method, route_chain_method_handler_and_middleware_identifiers,
    route_chain_segment, route_like_receiver, static_route_methods,
};
use crate::model::{
    EvidenceLocation, EvidenceStrength, MiddlewareOrGuard, MiddlewareOrGuardKind, RuntimeRoute,
};

pub(crate) fn javascript_route_registrations(
    rel: &str,
    line: &str,
    line_number: usize,
) -> Vec<RuntimeRoute> {
    let code = code_shape_without_literal_content(line);
    let mut routes = Vec::new();
    for method in static_route_methods() {
        for start in top_level_receiver_call_offsets(&code, &format!(".{method}(")) {
            let call = format!(".{method}(");
            let arg_start = start + call.len();
            let Some(path) = quoted_literal_at(line[arg_start..].trim_start()) else {
                continue;
            };
            let (handler_symbol, middleware) =
                route_call_handler_and_middleware_identifiers(line, &code, arg_start);
            routes.push(RuntimeRoute {
                method: Some(method.to_ascii_uppercase()),
                path,
                file: rel.to_string(),
                handler_symbol,
                middleware_or_guards: middleware_entities(rel, line_number, middleware),
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
                let (handler_symbol, middleware) =
                    route_chain_method_handler_and_middleware_identifiers(
                        line,
                        &code,
                        arg_start + close + 1,
                        method,
                    );
                routes.push(RuntimeRoute {
                    method: Some(method.to_ascii_uppercase()),
                    path: path.clone(),
                    file: rel.to_string(),
                    handler_symbol,
                    middleware_or_guards: middleware_entities(rel, line_number, middleware),
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
        let middleware = object_field_identifier(object_line, object_code, "preHandler")
            .into_iter()
            .collect();
        routes.push(RuntimeRoute {
            method: Some(method.to_ascii_uppercase()),
            path,
            file: rel.to_string(),
            handler_symbol: object_field_identifier(object_line, object_code, "handler"),
            middleware_or_guards: middleware_entities(rel, line_number, middleware),
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

fn middleware_entities(
    rel: &str,
    line_number: usize,
    names: Vec<String>,
) -> Vec<MiddlewareOrGuard> {
    names
        .into_iter()
        .map(|name| MiddlewareOrGuard {
            owner: format!("{rel}#{name}"),
            name,
            kind: MiddlewareOrGuardKind::Middleware,
            evidence: "javascript_route_middleware_argument".to_string(),
            strength: EvidenceStrength::High,
            locations: vec![EvidenceLocation::line(rel, line_number, "route_middleware")],
        })
        .collect()
}
