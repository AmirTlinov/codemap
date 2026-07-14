// Responsibility: repo-js-jsx-tag-spans
use crate::repo::{is_uppercase_symbol, static_jsx_visible_text};

#[derive(Debug, Clone)]
pub(crate) struct JsxOpeningTagSpan {
    pub(crate) tag: String,
    pub(crate) raw_tag: String,
    pub(crate) source: String,
    pub(crate) start: usize,
    pub(crate) opening_end: usize,
    pub(crate) self_closing: bool,
}

pub(crate) fn jsx_opening_tag_spans(text: &str) -> Vec<JsxOpeningTagSpan> {
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

pub(crate) fn find_jsx_closing_tag_start(text: &str, tag: &str, start: usize) -> Option<usize> {
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

pub(crate) fn jsx_byte_is_inside_custom_component_boundary(text: &str, byte: usize) -> bool {
    jsx_opening_tag_spans(text).into_iter().any(|opening| {
        opening.start < byte
            && !opening.self_closing
            && is_uppercase_symbol(&opening.tag)
            && find_jsx_closing_tag_start(text, &opening.tag, opening.opening_end)
                .map(|close_start| close_start > byte)
                .unwrap_or(false)
    })
}

pub(crate) fn jsx_byte_is_inside_expression(text: &str, byte: usize) -> bool {
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

pub(crate) fn jsx_element_visible_text(text: &str, opening: &JsxOpeningTagSpan) -> Option<String> {
    let close_start = find_jsx_closing_tag_start(text, &opening.tag, opening.opening_end)?;
    if close_start < opening.opening_end {
        return None;
    }
    static_jsx_visible_text(&text[opening.opening_end..close_start])
}
