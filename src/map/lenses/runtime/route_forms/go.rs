// Responsibility: go-route-registrations
use crate::map::{
    code_shape_without_literal_content, matching_close_paren, quoted_literal_at,
    route_call_second_arg_identifier, route_chain_segment, top_level_chain_call_offset,
};
use crate::model::{EvidenceLocation, EvidenceStrength, RuntimeRoute};

pub(crate) fn go_route_registrations(
    rel: &str,
    line: &str,
    line_number: usize,
) -> Vec<RuntimeRoute> {
    let code = code_shape_without_literal_content(line);
    let call = ".HandleFunc(";
    let mut routes = Vec::new();
    let mut offset = 0;
    while let Some(found) = code[offset..].find(call) {
        let start = offset + found;
        let open_paren = start + call.len() - 1;
        let Some(close) = matching_close_paren(&code, open_paren) else {
            offset = start + call.len();
            continue;
        };
        offset = close + 1;
        let Some(path) = quoted_literal_at(line[open_paren + 1..close].trim_start()) else {
            continue;
        };
        let method = if let Some(method) = go_route_method_in_chain(line, &code, close + 1) {
            method
        } else if go_route_has_methods_chain(&code, close + 1) {
            continue;
        } else {
            "ANY".to_string()
        };
        routes.push(RuntimeRoute {
            method: Some(method),
            path,
            file: rel.to_string(),
            handler_symbol: route_call_second_arg_identifier(line, &code, open_paren + 1),
            middleware_or_guards: Vec::new(),
            evidence: "go_http_route_registration".to_string(),
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

pub(crate) fn go_route_method_in_chain(
    line: &str,
    code: &str,
    chain_start: usize,
) -> Option<String> {
    let chain = route_chain_segment(&code[chain_start..]);
    let call = ".Methods(";
    let start = top_level_chain_call_offset(chain, call)?;
    quoted_literal_at(&line[chain_start + start + call.len()..])
        .map(|method| method.to_ascii_uppercase())
}

pub(crate) fn go_route_has_methods_chain(code: &str, chain_start: usize) -> bool {
    let chain = route_chain_segment(&code[chain_start..]);
    top_level_chain_call_offset(chain, ".Methods(").is_some()
}
