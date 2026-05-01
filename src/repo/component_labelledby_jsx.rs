fn jsx_opening_has_dialog_labelledby_attrs(opening: &JsxOpeningTagSpan) -> bool {
    !jsx_opening_has_spread_attr(&opening.source)
        && jsx_accessible_role_for_opening(&opening.tag, &opening.source).as_deref()
            == Some("dialog")
        && jsx_opening_has_exact_expression_attr(&opening.source, "aria-labelledby", "labelledBy")
}

fn jsx_opening_has_exact_expression_attr(opening: &str, attr: &str, expr: &str) -> bool {
    let chars: Vec<(usize, char)> = opening.char_indices().collect();
    let mut index = 0usize;
    let mut quote = None;
    let mut escaped = false;
    let mut brace_depth = 0usize;
    let mut attr_count = 0usize;
    let mut exact_matches = 0usize;
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
        if brace_depth > 0 {
            if ch == '{' {
                brace_depth += 1;
            } else if ch == '}' {
                brace_depth = brace_depth.saturating_sub(1);
            }
            index += 1;
            continue;
        }
        if ch == '{' {
            brace_depth = 1;
            index += 1;
            continue;
        }
        if opening[byte..].starts_with(attr)
            && jsx_attr_boundary_before(opening, byte)
            && jsx_attr_boundary_after(opening, byte + attr.len())
        {
            attr_count += 1;
            if jsx_attr_expression_value_matches(opening, byte + attr.len(), expr) {
                exact_matches += 1;
            }
        }
        index += 1;
    }
    attr_count == 1 && exact_matches == 1
}

fn jsx_attr_expression_value_matches(opening: &str, mut cursor: usize, expr: &str) -> bool {
    cursor = skip_js_whitespace(opening, cursor);
    if !opening[cursor..].starts_with('=') {
        return false;
    }
    cursor += 1;
    cursor = skip_js_whitespace(opening, cursor);
    if !opening[cursor..].starts_with('{') {
        return false;
    }
    cursor += 1;
    cursor = skip_js_whitespace(opening, cursor);
    if !opening[cursor..].starts_with(expr) {
        return false;
    }
    cursor += expr.len();
    if !js_identifier_boundary_after(opening, cursor) {
        return false;
    }
    cursor = skip_js_whitespace(opening, cursor);
    opening[cursor..].starts_with('}')
}

fn jsx_attr_boundary_before(text: &str, byte: usize) -> bool {
    text[..byte]
        .chars()
        .next_back()
        .map(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '$')))
        .unwrap_or(true)
}

fn jsx_attr_boundary_after(text: &str, byte: usize) -> bool {
    text[byte..]
        .chars()
        .next()
        .map(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '$')))
        .unwrap_or(true)
}

fn jsx_element_body_has_exact_expression(
    render_text: &str,
    opening: &JsxOpeningTagSpan,
    expr: &str,
) -> bool {
    if opening.self_closing {
        return false;
    }
    let Some(close_start) =
        find_jsx_closing_tag_start(render_text, &opening.tag, opening.opening_end)
    else {
        return false;
    };
    close_start >= opening.opening_end
        && jsx_body_has_exact_expression(&render_text[opening.opening_end..close_start], expr)
}

fn jsx_body_has_exact_expression(body: &str, expr: &str) -> bool {
    let chars: Vec<(usize, char)> = body.char_indices().collect();
    let mut index = 0usize;
    let mut in_tag = false;
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
        if in_tag {
            if matches!(ch, '"' | '\'' | '`') {
                quote = Some(ch);
                escaped = false;
            } else if ch == '>' {
                in_tag = false;
            }
            index += 1;
            continue;
        }
        if ch == '<' {
            in_tag = true;
            index += 1;
            continue;
        }
        if ch == '{' && jsx_expression_at(body, byte + ch.len_utf8(), expr) {
            return true;
        }
        index += 1;
    }
    false
}

fn jsx_expression_at(body: &str, mut cursor: usize, expr: &str) -> bool {
    cursor = skip_js_whitespace(body, cursor);
    if !body[cursor..].starts_with(expr) {
        return false;
    }
    cursor += expr.len();
    if !js_identifier_boundary_after(body, cursor) {
        return false;
    }
    cursor = skip_js_whitespace(body, cursor);
    body[cursor..].starts_with('}')
}

fn accessible_name_surfaces_from_component_labelled_ids(
    text: &str,
    component_roles: &BTreeMap<String, String>,
) -> SurfaceExtraction {
    let stripped = strip_js_comments_from_text(text);
    let mut surfaces = SurfaceExtraction::default();
    for opening in jsx_opening_tag_spans(&stripped) {
        if opening.raw_tag != opening.tag {
            continue;
        }
        let Some(role) = component_roles.get(&opening.tag) else {
            continue;
        };
        let Some(id) = jsx_single_exact_static_attr_value(&opening.source, "labelledBy") else {
            continue;
        };
        let mut roles = BTreeSet::new();
        roles.insert(role.clone());
        add_accessible_name_surface_from_label_in_opening_scope(
            &mut surfaces,
            &stripped,
            &opening,
            &id,
            &roles,
        );
    }
    surfaces
}

