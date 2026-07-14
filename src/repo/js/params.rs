// Responsibility: repo-js-params
use crate::repo::{identifier_re, js_keyword_positions, language_keyword, skip_ascii_whitespace};
use std::collections::BTreeSet;

pub(crate) fn collect_js_balanced_param_bindings(text: &str, out: &mut BTreeSet<String>) {
    for start in js_keyword_positions(text, "function") {
        let mut index = skip_ascii_whitespace(text, start + "function".len());
        if text
            .as_bytes()
            .get(index)
            .map(|byte| byte.is_ascii_alphabetic() || matches!(byte, b'_' | b'$'))
            .unwrap_or(false)
        {
            while text
                .as_bytes()
                .get(index)
                .map(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$'))
                .unwrap_or(false)
            {
                index += 1;
            }
            index = skip_ascii_whitespace(text, index);
        }
        if text.as_bytes().get(index) == Some(&b'(')
            && let Some(end) = js_balanced_pattern_end(text, index)
        {
            collect_js_param_bindings(&text[index + 1..end], out);
        }
    }

    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'(' {
            index += 1;
            continue;
        }
        let Some(end) = js_balanced_pattern_end(text, index) else {
            index += 1;
            continue;
        };
        if js_param_list_context(text, index, end) {
            collect_js_param_bindings(&text[index + 1..end], out);
        }
        index = end.saturating_add(1);
    }
}

fn js_param_list_context(text: &str, open: usize, close: usize) -> bool {
    let before_open = text[..open].trim_end();
    let after_close = text[close + 1..].trim_start();
    if before_open.ends_with("catch") {
        return true;
    }
    if js_tail_starts_arrow(after_close) {
        return true;
    }
    if js_tail_starts_block(after_close) {
        let name_before_params = before_open
            .rsplit(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'))
            .find(|part| !part.is_empty())
            .unwrap_or_default();
        return !matches!(
            name_before_params,
            "if" | "for" | "while" | "switch" | "catch" | "with" | "function"
        );
    }
    false
}

fn js_tail_starts_arrow(tail: &str) -> bool {
    if tail.starts_with("=>") {
        return true;
    }
    if !tail.starts_with(':') {
        return false;
    }
    let before_arrow = tail.split("=>").next().unwrap_or(tail);
    tail.contains("=>") && !before_arrow.contains(['{', ';'])
}

fn js_tail_starts_block(tail: &str) -> bool {
    if tail.starts_with('{') {
        return true;
    }
    if !tail.starts_with(':') {
        return false;
    }
    let before_block = tail.split('{').next().unwrap_or(tail);
    tail.contains('{') && !before_block.contains([';', '='])
}

pub(crate) fn js_destructuring_binding_patterns(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for keyword in ["const", "let", "var"] {
        for start in js_keyword_positions(text, keyword) {
            let pattern_start = skip_ascii_whitespace(text, start + keyword.len());
            let Some(open) = text.as_bytes().get(pattern_start).copied() else {
                continue;
            };
            if !matches!(open, b'{' | b'[') {
                continue;
            }
            let Some(pattern_end) = js_balanced_pattern_end(text, pattern_start) else {
                continue;
            };
            let after = skip_ascii_whitespace(text, pattern_end + 1);
            if text.as_bytes().get(after) == Some(&b'=') {
                out.push(&text[pattern_start..=pattern_end]);
            }
        }
    }
    out
}

pub(crate) fn js_balanced_pattern_end(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut stack = vec![match bytes.get(start).copied()? {
        b'{' => b'}',
        b'[' => b']',
        b'(' => b')',
        _ => return None,
    }];
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'{' => stack.push(b'}'),
            b'[' => stack.push(b']'),
            b'(' => stack.push(b')'),
            b'}' | b']' | b')' => {
                if stack.pop() != Some(bytes[index]) {
                    return None;
                }
                if stack.is_empty() {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

pub(crate) fn collect_js_param_bindings(params: &str, out: &mut BTreeSet<String>) {
    for ident in identifier_re().find_iter(params).map(|m| m.as_str()) {
        if !language_keyword(ident) {
            out.insert(ident.to_string());
        }
    }
}
