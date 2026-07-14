// Responsibility: repo-playwright-scope-tracking
use crate::repo::{
    first_identifier, js_balanced_call_end, js_balanced_pattern_end,
    js_byte_is_inside_string_or_regex_literal, js_destructure_part_is_direct_shorthand_prop,
    js_first_balanced_object_span, js_identifier_boundary_after, js_identifier_boundary_before,
    js_keyword_positions, js_regex_literal_can_start, js_regex_literal_end,
    js_split_top_level_commas, skip_js_whitespace,
};
use std::collections::BTreeSet;

pub(crate) fn line_declares_pending_function_body(line: &str) -> bool {
    for start in js_keyword_positions(line, "function") {
        let Some(open) = line[start..].find('(').map(|relative| start + relative) else {
            continue;
        };
        if js_byte_is_inside_string_or_regex_literal(line, open) {
            continue;
        }
        let Some(close) = js_balanced_call_end(line, open) else {
            continue;
        };
        let cursor = skip_js_whitespace(line, close);
        if !line[cursor..].starts_with('{') && !line[cursor..].contains(';') {
            return true;
        }
    }
    false
}

pub(crate) fn line_declares_pending_method_body(line: &str) -> bool {
    let trimmed = line.trim_start();
    let Some(open) = trimmed.find('(') else {
        return false;
    };
    if js_byte_is_inside_string_or_regex_literal(trimmed, open) {
        return false;
    }
    let before_open = trimmed[..open].trim_end();
    if before_open.contains(['.', '=']) {
        return false;
    }
    let Some(name) = before_open
        .rsplit(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'))
        .find(|part| !part.is_empty())
    else {
        return false;
    };
    if matches!(
        name,
        "if" | "for" | "while" | "switch" | "catch" | "function" | "test" | "expect" | "page"
    ) {
        return false;
    }
    let Some(close) = js_balanced_call_end(trimmed, open) else {
        return false;
    };
    let cursor = skip_js_whitespace(trimmed, close);
    !trimmed[cursor..].starts_with('{') && !trimmed[cursor..].contains(';')
}

pub(crate) fn line_starts_function_callback_body(line: &str) -> bool {
    for start in js_keyword_positions(line, "function") {
        let Some(open) = line[start..].find('(').map(|relative| start + relative) else {
            continue;
        };
        if js_byte_is_inside_string_or_regex_literal(line, open) {
            continue;
        }
        let Some(close) = js_balanced_call_end(line, open) else {
            continue;
        };
        let cursor = skip_js_whitespace(line, close);
        if line[cursor..].starts_with('{') {
            return true;
        }
    }
    false
}

pub(crate) fn line_declares_local_page_binding(line: &str) -> bool {
    line_declares_local_identifier(line, "page")
        || line_declares_arrow_param_identifier(line, "page")
}

pub(crate) fn line_declares_local_identifier(line: &str, ident: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with("import ") || trimmed.starts_with("import{") {
        return false;
    }
    ["const", "let", "var", "function", "class"]
        .iter()
        .any(|keyword| line_declares_identifier_after_keyword(line, keyword, ident))
}

fn line_declares_identifier_after_keyword(line: &str, keyword: &str, ident: &str) -> bool {
    for start in js_keyword_positions(line, keyword) {
        let mut cursor = skip_js_whitespace(line, start + keyword.len());
        if line[cursor..].starts_with('{')
            && let Some(end) = js_balanced_pattern_end(line, cursor)
        {
            return js_split_top_level_commas(&line[cursor + 1..end])
                .iter()
                .any(|part| js_destructure_part_is_direct_shorthand_prop(part, ident));
        }
        if line[cursor..].starts_with(ident)
            && js_identifier_boundary_after(line, cursor + ident.len())
        {
            return true;
        }
        if matches!(keyword, "function")
            && let Some(name) = first_identifier(&line[cursor..])
            && name != ident
        {
            cursor += name.len();
            cursor = skip_js_whitespace(line, cursor);
            if line[cursor..].starts_with('(')
                && let Some(end) = js_balanced_call_end(line, cursor)
            {
                return js_param_list_contains_direct_identifier(&line[cursor + 1..end - 1], ident);
            }
        }
    }
    false
}

fn line_declares_arrow_param_identifier(line: &str, ident: &str) -> bool {
    let mut search_start = 0usize;
    while let Some(relative) = line[search_start..].find("=>") {
        let arrow = search_start + relative;
        if js_byte_is_inside_string_or_regex_literal(line, arrow) {
            search_start = arrow + 2;
            continue;
        }
        if arrow_params_contain_identifier(&line[..arrow], ident) {
            return true;
        }
        search_start = arrow + 2;
    }
    false
}

fn arrow_params_contain_identifier(before_arrow: &str, ident: &str) -> bool {
    let trimmed = before_arrow.trim_end();
    if let Some(close) = trimmed.rfind(')')
        && close + 1 == trimmed.len()
    {
        let Some(open) = matching_open_paren(trimmed, close) else {
            return false;
        };
        return js_param_list_contains_direct_identifier(&trimmed[open + 1..close], ident);
    }
    trimmed
        .rsplit(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'))
        .find(|part| !part.is_empty())
        .map(|part| part == ident)
        .unwrap_or(false)
}

fn js_param_list_contains_direct_identifier(params: &str, ident: &str) -> bool {
    js_split_top_level_commas(params).iter().any(|part| {
        let trimmed = part.trim();
        trimmed == ident
            || (trimmed.starts_with('{')
                && trimmed.ends_with('}')
                && js_split_top_level_commas(&trimmed[1..trimmed.len().saturating_sub(1)])
                    .iter()
                    .any(|part| js_destructure_part_is_direct_shorthand_prop(part, ident)))
    })
}

fn matching_open_paren(text: &str, close: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    for (byte, ch) in text[..=close].char_indices().rev() {
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
            ')' => depth += 1,
            '(' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(byte);
                }
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn js_last_balanced_object_span(value: &str) -> Option<(usize, usize)> {
    let mut cursor = 0usize;
    let mut last = None;
    while cursor < value.len() {
        let Some((start, end)) = js_first_balanced_object_span(&value[cursor..]) else {
            break;
        };
        last = Some((cursor + start, cursor + end));
        cursor += end + 1;
    }
    last
}

pub(crate) fn line_has_playwright_test_call(
    line: &str,
    playwright_test_bindings: &BTreeSet<String>,
) -> bool {
    playwright_test_bindings
        .iter()
        .any(|binding| line_has_playwright_test_binding_call(line, binding))
}

fn line_has_playwright_test_binding_call(line: &str, binding: &str) -> bool {
    if line_has_playwright_test_method_call(line, binding, &["only"]) {
        return true;
    }
    line_has_playwright_test_direct_call(line, binding)
}

fn line_has_playwright_test_direct_call(line: &str, binding: &str) -> bool {
    let mut search_start = 0usize;
    while let Some(relative) = line[search_start..].find(binding) {
        let start = search_start + relative;
        let end = start + binding.len();
        if js_byte_is_inside_string_or_regex_literal(line, start)
            || !js_identifier_boundary_before(line, start)
            || !js_identifier_boundary_after(line, end)
        {
            search_start = end;
            continue;
        }
        let cursor = skip_js_whitespace(line, end);
        if line[cursor..].starts_with('(') {
            return true;
        }
        search_start = end;
    }
    false
}

pub(crate) fn line_has_playwright_test_method_call(
    line: &str,
    binding: &str,
    methods: &[&str],
) -> bool {
    let mut search_start = 0usize;
    while let Some(relative) = line[search_start..].find(binding) {
        let start = search_start + relative;
        let end = start + binding.len();
        if js_byte_is_inside_string_or_regex_literal(line, start)
            || !js_identifier_boundary_before(line, start)
            || !js_identifier_boundary_after(line, end)
        {
            search_start = end;
            continue;
        }
        for method_path in methods {
            let mut cursor = skip_js_whitespace(line, end);
            let mut matched = true;
            for segment in method_path.split('.') {
                if !line[cursor..].starts_with('.') {
                    matched = false;
                    break;
                }
                cursor = skip_js_whitespace(line, cursor + 1);
                if !line[cursor..].starts_with(segment)
                    || !js_identifier_boundary_after(line, cursor + segment.len())
                {
                    matched = false;
                    break;
                }
                cursor = skip_js_whitespace(line, cursor + segment.len());
            }
            if matched && line[cursor..].starts_with('(') {
                return true;
            }
        }
        search_start = end;
    }
    false
}

pub(crate) fn apply_js_brace_delta(mut depth: usize, line: &str) -> usize {
    let bytes = line.as_bytes();
    let mut index = 0usize;
    let mut prefix = String::new();
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(active_quote) = quote {
            index += 1;
            if escaped {
                prefix.push(' ');
                escaped = false;
                continue;
            }
            if byte == b'\\' {
                escaped = true;
                prefix.push(' ');
                continue;
            }
            if byte == active_quote {
                quote = None;
            }
            prefix.push(' ');
            continue;
        }
        if matches!(byte, b'"' | b'\'' | b'`') {
            quote = Some(byte);
            escaped = false;
            prefix.push(' ');
            index += 1;
            continue;
        }
        if byte == b'/'
            && js_regex_literal_can_start(&prefix)
            && let Some(end) = js_regex_literal_end(bytes, index)
        {
            prefix.push(' ');
            index = end;
            continue;
        }
        match byte {
            b'{' => depth += 1,
            b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
        prefix.push(byte as char);
        index += 1;
    }
    depth
}
