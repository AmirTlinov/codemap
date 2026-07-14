// Responsibility: repo-js-object-strings

pub(crate) fn js_string_literal_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let mut chars = trimmed.char_indices();
    let (_, quote) = chars.next()?;
    if !matches!(quote, '"' | '\'' | '`') {
        return None;
    }
    let mut out = String::new();
    let mut escaped = false;
    let mut end_byte = None;
    for (byte, ch) in chars {
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == quote {
            end_byte = Some(byte + ch.len_utf8());
            break;
        }
        out.push(ch);
    }
    end_byte.and_then(|end| trimmed[end..].trim().is_empty().then_some(out))
}

fn js_top_level_object_string_property_values(object: &str, key: &str) -> Vec<String> {
    let chars: Vec<(usize, char)> = object.char_indices().collect();
    let mut values = Vec::new();
    let mut index = 0usize;
    let mut object_depth = 0usize;
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
        match ch {
            '{' => {
                object_depth += 1;
                index += 1;
            }
            '}' => {
                object_depth = object_depth.saturating_sub(1);
                index += 1;
            }
            '(' => {
                paren_depth += 1;
                index += 1;
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                index += 1;
            }
            '[' => {
                bracket_depth += 1;
                index += 1;
            }
            ']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                index += 1;
            }
            _ => {
                if object_depth == 1
                    && paren_depth == 0
                    && bracket_depth == 0
                    && object[byte..].starts_with(key)
                    && js_identifier_boundary_before(object, byte)
                    && js_identifier_boundary_after(object, byte + key.len())
                    && let Some((value, next_index)) =
                        js_string_value_after_object_key(&chars, index, key.len())
                {
                    values.push(value);
                    index = next_index;
                    continue;
                }
                index += 1;
            }
        }
    }
    values
}

pub(crate) fn js_plain_object_single_string_property_value(
    object: &str,
    key: &str,
) -> Option<String> {
    let trimmed = object.trim();
    if !trimmed.starts_with('{') || !trimmed.ends_with('}') {
        return None;
    }
    if js_top_level_object_has_spread(trimmed) {
        return None;
    }
    let values = js_top_level_object_string_property_values(trimmed, key);
    match values.as_slice() {
        [only] => Some(only.clone()),
        _ => None,
    }
}

fn js_top_level_object_has_spread(object: &str) -> bool {
    let chars: Vec<(usize, char)> = object.char_indices().collect();
    let mut index = 0usize;
    let mut object_depth = 0usize;
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
        if object_depth == 1
            && paren_depth == 0
            && bracket_depth == 0
            && object[byte..].starts_with("...")
        {
            return true;
        }
        match ch {
            '{' => object_depth += 1,
            '}' => object_depth = object_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            _ => {}
        }
        index += 1;
    }
    false
}

fn js_string_value_after_object_key(
    chars: &[(usize, char)],
    key_index: usize,
    key_len: usize,
) -> Option<(String, usize)> {
    let mut index = key_index + key_len;
    while index < chars.len() && chars[index].1.is_whitespace() {
        index += 1;
    }
    if chars.get(index).map(|(_, ch)| *ch) != Some(':') {
        return None;
    }
    index += 1;
    while index < chars.len() && chars[index].1.is_whitespace() {
        index += 1;
    }
    let (_, quote) = *chars.get(index)?;
    if !matches!(quote, '"' | '\'' | '`') {
        return None;
    }
    let mut value = String::new();
    let mut escaped = false;
    index += 1;
    while index < chars.len() {
        let (_, ch) = chars[index];
        index += 1;
        if escaped {
            value.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == quote {
            return Some((value, index));
        }
        value.push(ch);
    }
    None
}

pub(crate) fn js_identifier_boundary_before(text: &str, byte: usize) -> bool {
    text[..byte]
        .chars()
        .next_back()
        .map(|ch| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'))
        .unwrap_or(true)
}

pub(crate) fn js_identifier_boundary_after(text: &str, byte: usize) -> bool {
    text[byte..]
        .chars()
        .next()
        .map(|ch| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'))
        .unwrap_or(true)
}

pub(crate) fn quoted_prefix_has_object_key(prefix: &str, key: &str) -> bool {
    let lower = prefix.to_ascii_lowercase();
    let Some(before_colon) = lower.trim_end().strip_suffix(':') else {
        return false;
    };
    trailing_js_property_name(before_colon)
        .map(|name| name == key)
        .unwrap_or(false)
}

fn trailing_js_property_name(value: &str) -> Option<String> {
    value
        .trim_end()
        .rsplit(|ch: char| !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '$' | '-')))
        .find(|part| !part.is_empty())
        .map(str::to_ascii_lowercase)
}
