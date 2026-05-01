fn accessible_name_surfaces_from_native_labelled_ids(text: &str) -> SurfaceExtraction {
    let stripped = strip_js_comments_from_text(text);
    let mut surfaces = SurfaceExtraction::default();
    for opening in jsx_opening_tag_spans(&stripped) {
        let Some(role) = jsx_accessible_role_for_opening(&opening.tag, &opening.source) else {
            continue;
        };
        let Some(id) = jsx_single_static_attr_value(&opening.source, "aria-labelledby") else {
            continue;
        };
        let mut roles = BTreeSet::new();
        roles.insert(role);
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

fn jsx_accessible_role_for_opening(tag: &str, opening: &str) -> Option<String> {
    if tag != tag.to_ascii_lowercase() {
        return None;
    }
    let mut role_attrs = 0usize;
    let mut roles = Vec::new();
    for quoted in quoted_strings(opening) {
        if quoted_prefix_has_jsx_attr(&quoted.prefix, "role") {
            role_attrs += 1;
            if let Some(role) = normalize_accessible_role(&quoted.value) {
                roles.push(role);
            }
        }
    }
    if role_attrs > 0 {
        return match roles.as_slice() {
            [role] if role_attrs == 1 => Some(role.clone()),
            _ => None,
        };
    }
    normalize_accessible_role(tag)
}

fn normalize_accessible_role(value: &str) -> Option<String> {
    let role = value.trim().to_ascii_lowercase();
    match role.as_str() {
        "alertdialog" => Some("alertdialog".to_string()),
        "dialog" => Some("dialog".to_string()),
        _ => None,
    }
}

fn quoted_prefix_has_jsx_attr(prefix: &str, attr: &str) -> bool {
    trailing_jsx_attr_name(prefix)
        .map(|name| name.eq_ignore_ascii_case(attr))
        .unwrap_or(false)
}

fn quoted_prefix_has_exact_jsx_attr(prefix: &str, attr: &str) -> bool {
    trailing_jsx_attr_name(prefix)
        .map(|name| name == attr)
        .unwrap_or(false)
}

fn trailing_jsx_attr_name(prefix: &str) -> Option<String> {
    let before_eq = prefix.trim_end().strip_suffix('=')?.trim_end();
    before_eq
        .rsplit(|ch: char| !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '$')))
        .find(|part| !part.is_empty())
        .map(str::to_string)
}

#[derive(Debug, Clone)]
struct JsxOpeningTagSpan {
    tag: String,
    raw_tag: String,
    source: String,
    start: usize,
    opening_end: usize,
    self_closing: bool,
}

fn jsx_opening_tag_spans(text: &str) -> Vec<JsxOpeningTagSpan> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut spans = Vec::new();
    let mut index = 0usize;
    let mut outer_quote = None;
    let mut outer_escaped = false;
    while index < chars.len() {
        let (start_byte, ch) = chars[index];
        if let Some(active_quote) = outer_quote {
            index += 1;
            if outer_escaped {
                outer_escaped = false;
                continue;
            }
            if ch == '\\' {
                outer_escaped = true;
                continue;
            }
            if ch == active_quote {
                outer_quote = None;
            }
            continue;
        }
        if matches!(ch, '"' | '\'' | '`') {
            outer_quote = Some(ch);
            outer_escaped = false;
            index += 1;
            continue;
        }
        if ch != '<' {
            index += 1;
            continue;
        }
        index += 1;
        while index < chars.len() && chars[index].1.is_whitespace() {
            index += 1;
        }
        if matches!(
            chars.get(index).map(|(_, ch)| *ch),
            None | Some('/' | '!' | '?' | '>')
        ) {
            continue;
        }
        let tag_start_index = index;
        while index < chars.len() {
            let (_, ch) = chars[index];
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '$' | '-' | '.') {
                index += 1;
            } else {
                break;
            }
        }
        if index == tag_start_index {
            continue;
        }
        let tag_start_byte = chars[tag_start_index].0;
        let tag_end_byte = chars[index - 1].0 + chars[index - 1].1.len_utf8();
        let raw_tag = &text[tag_start_byte..tag_end_byte];
        let tag = raw_tag.rsplit('.').next().unwrap_or(raw_tag).to_string();
        let raw_tag = raw_tag.to_string();

        let mut scan = index;
        let mut quote = None;
        let mut escaped = false;
        let mut brace_depth = 0usize;
        while scan < chars.len() {
            let (byte, ch) = chars[scan];
            scan += 1;
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
            if ch == '{' {
                brace_depth += 1;
                continue;
            }
            if ch == '}' {
                brace_depth = brace_depth.saturating_sub(1);
                continue;
            }
            if ch == '>' && brace_depth == 0 {
                let opening_end = byte + ch.len_utf8();
                let source = text[start_byte..opening_end].to_string();
                let self_closing = source.trim_end_matches('>').trim_end().ends_with('/');
                spans.push(JsxOpeningTagSpan {
                    tag,
                    raw_tag,
                    source,
                    start: start_byte,
                    opening_end,
                    self_closing,
                });
                index = scan;
                break;
            }
        }
    }
    spans
}

fn find_jsx_closing_tag_start(text: &str, tag: &str, start: usize) -> Option<usize> {
    let tag_lower = tag.to_ascii_lowercase();
    let mut index = start.min(text.len());
    while index < text.len() {
        let rel = text[index..].find("</")?;
        let close_start = index + rel;
        let mut cursor = close_start + 2;
        while cursor < text.len() {
            let ch = text[cursor..].chars().next()?;
            if ch.is_whitespace() {
                cursor += ch.len_utf8();
            } else {
                break;
            }
        }
        let name_start = cursor;
        while cursor < text.len() {
            let ch = text[cursor..].chars().next()?;
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '$' | '-' | '.') {
                cursor += ch.len_utf8();
            } else {
                break;
            }
        }
        let raw_name = &text[name_start..cursor];
        let name = raw_name.rsplit('.').next().unwrap_or(raw_name);
        let next = text[cursor..].chars().next();
        if name.eq_ignore_ascii_case(&tag_lower)
            && next
                .map(|ch| ch.is_whitespace() || ch == '>')
                .unwrap_or(false)
        {
            return Some(close_start);
        }
        index = close_start + 2;
    }
    None
}

fn jsx_single_static_attr_value(opening: &str, attr: &str) -> Option<String> {
    jsx_single_static_attr_value_by(opening, attr, quoted_prefix_has_jsx_attr)
}

fn jsx_single_exact_static_attr_value(opening: &str, attr: &str) -> Option<String> {
    jsx_single_static_attr_value_by(opening, attr, quoted_prefix_has_exact_jsx_attr)
}

fn jsx_single_static_attr_value_by(
    opening: &str,
    attr: &str,
    attr_matches: fn(&str, &str) -> bool,
) -> Option<String> {
    if jsx_opening_has_spread_attr(opening) {
        return None;
    }
    let mut attr_occurrences = 0usize;
    let mut values = Vec::new();
    for quoted in quoted_strings(opening) {
        if attr_matches(&quoted.prefix, attr) {
            attr_occurrences += 1;
            let ids = quoted
                .value
                .split_whitespace()
                .map(str::trim)
                .filter(|id| id_is_static_accessible_reference(id))
                .collect::<Vec<_>>();
            if ids.len() == 1 {
                values.push(ids[0].to_string());
            }
        }
    }
    match (attr_occurrences, values.as_slice()) {
        (1, [only]) => Some(only.clone()),
        _ => None,
    }
}

fn jsx_opening_has_spread_attr(opening: &str) -> bool {
    let chars: Vec<(usize, char)> = opening.char_indices().collect();
    let mut index = 0usize;
    let mut quote = None;
    let mut escaped = false;
    let mut brace_depth = 0usize;
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
            match ch {
                '{' => brace_depth += 1,
                '}' => brace_depth = brace_depth.saturating_sub(1),
                _ => {}
            }
            index += 1;
            continue;
        }
        if ch == '{' {
            let cursor = skip_js_whitespace(opening, byte + ch.len_utf8());
            if opening[cursor..].starts_with("...") {
                return true;
            }
            brace_depth = 1;
            index += 1;
            continue;
        }
        index += 1;
    }
    false
}

fn add_accessible_name_surface_from_label_in_opening_scope(
    surfaces: &mut SurfaceExtraction,
    text: &str,
    opening: &JsxOpeningTagSpan,
    id: &str,
    roles: &BTreeSet<String>,
) {
    if opening.self_closing {
        return;
    }
    let Some(close_start) = find_jsx_closing_tag_start(text, &opening.tag, opening.opening_end)
    else {
        return;
    };
    if close_start < opening.opening_end {
        return;
    }
    let body = &text[opening.opening_end..close_start];
    let mut matching_label_count = 0usize;
    let mut matching_labels = Vec::new();
    for candidate in jsx_opening_tag_spans(body) {
        if jsx_single_static_attr_value(&candidate.source, "id").as_deref() != Some(id) {
            continue;
        }
        matching_label_count += 1;
        if jsx_byte_is_inside_expression(body, candidate.start) {
            continue;
        }
        if jsx_byte_is_inside_custom_component_boundary(body, candidate.start) {
            continue;
        }
        if candidate.self_closing {
            continue;
        }
        if let Some(text) = jsx_element_visible_text(body, &candidate) {
            matching_labels.push(text);
        }
    }
    if matching_label_count == 1
        && let [text] = matching_labels.as_slice()
    {
        add_accessible_name_parts_surface(surfaces, roles, std::slice::from_ref(text));
    }
}

fn jsx_byte_is_inside_custom_component_boundary(text: &str, byte: usize) -> bool {
    jsx_opening_tag_spans(text).into_iter().any(|opening| {
        opening.start < byte
            && !opening.self_closing
            && is_uppercase_symbol(&opening.tag)
            && find_jsx_closing_tag_start(text, &opening.tag, opening.opening_end)
                .map(|close_start| close_start > byte)
                .unwrap_or(false)
    })
}

fn jsx_byte_is_inside_expression(text: &str, byte: usize) -> bool {
    let mut index = 0usize;
    let mut quote = None;
    let mut escaped = false;
    let mut brace_depth = 0usize;
    while index < text.len() && index < byte {
        let Some(ch) = text[index..].chars().next() else {
            break;
        };
        if let Some(active_quote) = quote {
            index += ch.len_utf8();
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
            index += ch.len_utf8();
            continue;
        }
        match ch {
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            _ => {}
        }
        index += ch.len_utf8();
    }
    brace_depth > 0
}

fn jsx_element_visible_text(text: &str, opening: &JsxOpeningTagSpan) -> Option<String> {
    let close_start = find_jsx_closing_tag_start(text, &opening.tag, opening.opening_end)?;
    if close_start < opening.opening_end {
        return None;
    }
    static_jsx_visible_text(&text[opening.opening_end..close_start])
}

fn add_accessible_name_parts_surface(
    surfaces: &mut SurfaceExtraction,
    roles: &BTreeSet<String>,
    parts: &[String],
) {
    let text = parts
        .iter()
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if !text.is_empty() {
        add_accessible_role_name_surfaces(surfaces, roles, &text);
    }
}

fn id_is_static_accessible_reference(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty()
        && trimmed.len() <= 80
        && trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
}

fn add_accessible_role_name_surfaces(
    surfaces: &mut SurfaceExtraction,
    roles: &BTreeSet<String>,
    name: &str,
) {
    surfaces.tokens.extend(surface_literal_terms(name));
    surfaces.phrases.extend(surface_literal_phrases(name, true));
    for role in roles {
        add_accessible_role_name_surface(surfaces, role, name);
    }
}

fn add_accessible_role_name_surface(surfaces: &mut SurfaceExtraction, role: &str, name: &str) {
    if !surface_label_literal_is_structural(name) {
        return;
    }
    let Some(name_phrase) = normalize_surface_phrase(name) else {
        return;
    };
    let terms = surface_phrase_terms(&name_phrase);
    if terms.is_empty() {
        return;
    }
    surfaces.tokens.extend(terms);
    surfaces
        .phrases
        .insert(format!("a11y-role-{role}-name-{name_phrase}"));
}

