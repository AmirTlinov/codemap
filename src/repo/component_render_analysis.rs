#[derive(Debug)]
struct ComponentSignature {
    params: String,
    block_open: Option<usize>,
    expression_start: Option<usize>,
}

fn component_signature(body: &str) -> Option<ComponentSignature> {
    let function_match = js_function_params_re().captures(body).and_then(|captures| {
        let full = captures.get(0)?;
        if !component_function_prefix_is_direct_declaration(&body[..full.start()]) {
            return None;
        }
        let params = captures.name("params")?.as_str().to_string();
        Some((full.start(), full.end(), params))
    });
    let arrow_match = js_arrow_params_re().captures(body).and_then(|captures| {
        let full = captures.get(0)?;
        if !component_arrow_prefix_is_direct_initializer(&body[..full.start()]) {
            return None;
        }
        let params = captures.name("params")?.as_str().to_string();
        Some((full.start(), full.end(), params))
    });
    let use_arrow = match (function_match.as_ref(), arrow_match.as_ref()) {
        (Some((function_start, _, _)), Some((arrow_start, _, _))) => arrow_start < function_start,
        (None, Some(_)) => true,
        _ => false,
    };
    if use_arrow {
        let (_, end, params) = arrow_match?;
        let cursor = skip_js_whitespace(body, end);
        if body[cursor..].starts_with('{') {
            Some(ComponentSignature {
                params,
                block_open: Some(cursor),
                expression_start: None,
            })
        } else {
            Some(ComponentSignature {
                params,
                block_open: None,
                expression_start: Some(cursor),
            })
        }
    } else {
        let (_, end, params) = function_match?;
        let cursor = skip_js_whitespace(body, end);
        let block_open = body[cursor..].find('{').map(|offset| cursor + offset)?;
        Some(ComponentSignature {
            params,
            block_open: Some(block_open),
            expression_start: None,
        })
    }
}

fn component_function_prefix_is_direct_declaration(prefix: &str) -> bool {
    prefix
        .split_whitespace()
        .all(|token| matches!(token, "export" | "default" | "async"))
}

fn component_arrow_prefix_is_direct_initializer(prefix: &str) -> bool {
    prefix.trim_end().ends_with('=')
}

fn skip_js_whitespace(text: &str, mut cursor: usize) -> usize {
    while cursor < text.len() {
        let Some(ch) = text[cursor..].chars().next() else {
            break;
        };
        if ch.is_whitespace() {
            cursor += ch.len_utf8();
        } else {
            break;
        }
    }
    cursor
}

fn component_direct_render_texts(body: &str, signature: &ComponentSignature) -> Vec<String> {
    if let Some(start) = signature.expression_start {
        return vec![body[start..].trim().trim_end_matches(';').to_string()];
    }
    signature
        .block_open
        .map(|block_open| top_level_return_expressions(body, block_open))
        .unwrap_or_default()
}

fn top_level_return_expressions(text: &str, block_open: usize) -> Vec<String> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let Some(mut index) = chars.iter().position(|(byte, _)| *byte == block_open) else {
        return Vec::new();
    };
    index += 1;
    let mut expressions = Vec::new();
    let mut brace_depth = 1usize;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    while index < chars.len() {
        let (byte, ch) = chars[index];
        if let Some(active_quote) = quote {
            index += 1;
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == active_quote {
                quote = None;
            }
            continue;
        }
        if matches!(ch, '"' | '\'' | '`') {
            quote = Some(ch);
            escaped = false;
            index += 1;
            continue;
        }
        if brace_depth == 1 && text[byte..].starts_with("return") {
            let before_ok = js_identifier_boundary_before(text, byte);
            let after = byte + "return".len();
            let after_ok = js_identifier_boundary_after(text, after);
            if before_ok && after_ok {
                if let Some(expression) = extract_return_expression(text, after) {
                    expressions.push(expression);
                }
                index += "return".len();
                continue;
            }
        }
        match ch {
            '{' => brace_depth += 1,
            '}' => {
                brace_depth = brace_depth.saturating_sub(1);
                if brace_depth == 0 {
                    break;
                }
            }
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            _ => {}
        }
        index += 1;
    }
    let _ = (paren_depth, bracket_depth);
    expressions
}

fn extract_return_expression(text: &str, start: usize) -> Option<String> {
    let chars: Vec<(usize, char)> = text[start..].char_indices().collect();
    let mut index = 0usize;
    let mut brace_depth = 1usize;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    while index < chars.len() {
        let (relative_byte, ch) = chars[index];
        if let Some(active_quote) = quote {
            index += 1;
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == active_quote {
                quote = None;
            }
            continue;
        }
        if matches!(ch, '"' | '\'' | '`') {
            quote = Some(ch);
            escaped = false;
            index += 1;
            continue;
        }
        match ch {
            '{' => brace_depth += 1,
            '}' => {
                brace_depth = brace_depth.saturating_sub(1);
                if brace_depth == 0 {
                    return Some(text[start..start + relative_byte].trim().to_string());
                }
            }
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            ';' if brace_depth == 1 && paren_depth == 0 && bracket_depth == 0 => {
                return Some(text[start..start + relative_byte].trim().to_string());
            }
            _ => {}
        }
        index += 1;
    }
    let expression = text[start..].trim();
    (!expression.is_empty()).then_some(expression.to_string())
}

fn params_destructure_direct_shorthand_prop(params: &str, prop: &str) -> bool {
    let Some((start, end)) = js_first_balanced_object_span(params) else {
        return false;
    };
    js_split_top_level_commas(&params[start + 1..end])
        .into_iter()
        .any(|part| js_destructure_part_is_direct_shorthand_prop(&part, prop))
}

fn js_first_balanced_object_span(value: &str) -> Option<(usize, usize)> {
    let chars: Vec<(usize, char)> = value.char_indices().collect();
    let start_index = chars.iter().position(|(_, ch)| *ch == '{')?;
    let start_byte = chars[start_index].0;
    let mut index = start_index;
    let mut brace_depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    while index < chars.len() {
        let (byte, ch) = chars[index];
        index += 1;
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == active_quote {
                quote = None;
            }
            continue;
        }
        if matches!(ch, '"' | '\'' | '`') {
            quote = Some(ch);
            escaped = false;
            continue;
        }
        match ch {
            '{' => brace_depth += 1,
            '}' => {
                brace_depth = brace_depth.saturating_sub(1);
                if brace_depth == 0 {
                    return Some((start_byte, byte));
                }
            }
            _ => {}
        }
    }
    None
}

fn js_destructure_part_is_direct_shorthand_prop(part: &str, prop: &str) -> bool {
    let trimmed = part.trim();
    let Some(rest) = trimmed.strip_prefix(prop) else {
        return false;
    };
    if !js_identifier_boundary_after(trimmed, prop.len()) {
        return false;
    }
    let rest = rest.trim_start();
    rest.is_empty() || rest.starts_with('=')
}

fn component_body_shadows_labelledby(body: &str) -> bool {
    js_labelledby_local_binding_re().is_match(body)
}
