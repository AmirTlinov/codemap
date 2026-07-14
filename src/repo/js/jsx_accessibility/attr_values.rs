// Responsibility: repo-js-jsx-attr-values
use crate::repo::{quoted_strings, skip_js_whitespace};

pub(crate) fn quoted_prefix_has_jsx_attr(prefix: &str, attr: &str) -> bool {
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

pub(crate) fn jsx_single_static_attr_value(opening: &str, attr: &str) -> Option<String> {
    jsx_single_static_attr_value_by(opening, attr, quoted_prefix_has_jsx_attr)
}

pub(crate) fn jsx_single_exact_static_attr_value(opening: &str, attr: &str) -> Option<String> {
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

pub(crate) fn jsx_opening_has_spread_attr(opening: &str) -> bool {
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

fn id_is_static_accessible_reference(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty()
        && trimmed.len() <= 80
        && trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
}
