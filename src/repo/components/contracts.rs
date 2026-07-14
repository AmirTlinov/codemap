// Responsibility: repo-components-contracts
use crate::model::{FileInfo, SymbolInfo};
use crate::repo::{
    component_body_shadows_labelledby, component_direct_render_texts, component_signature,
    is_uppercase_symbol, js_balanced_call_end, js_identifier_boundary_after,
    js_identifier_boundary_before, jsx_element_body_has_exact_expression,
    jsx_opening_has_dialog_labelledby_attrs, jsx_opening_tag_spans,
    params_destructure_direct_shorthand_prop, skip_js_whitespace, strip_js_comments_from_text,
};
use std::fs;
use std::path::Path;

pub(crate) fn file_exports_dialog_labelledby_contract(
    root: &Path,
    info: &FileInfo,
    export_name: &str,
) -> bool {
    let Ok(text) = fs::read_to_string(root.join(&info.rel)) else {
        return false;
    };
    info.symbols
        .iter()
        .filter(|symbol| symbol.exported && symbol.name == export_name)
        .any(|symbol| {
            let body = symbol_body_text(&text, symbol);
            component_body_has_dialog_labelledby_contract(&body)
        })
}

pub(crate) fn symbol_body_text(text: &str, symbol: &SymbolInfo) -> String {
    text.lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            let line_no = idx + 1;
            (line_no >= symbol.line_start && line_no <= symbol.line_end).then_some(line)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn component_body_has_dialog_labelledby_contract(body: &str) -> bool {
    let Some(signature) = component_signature(body) else {
        return false;
    };
    if !params_destructure_direct_shorthand_prop(&signature.params, "labelledBy")
        || !params_destructure_direct_shorthand_prop(&signature.params, "children")
        || component_body_shadows_labelledby(body)
        || component_body_has_unparsed_control_flow(body)
    {
        return false;
    }
    let mut found_dialog_labelledby = false;
    for render_text in component_direct_render_texts(body, &signature) {
        if component_render_text_is_empty_ui(&render_text) {
            continue;
        }
        if js_text_contains_unparsed_render_control_flow(&render_text) {
            return false;
        }
        if js_text_contains_call_with_jsx_argument(&render_text) {
            return false;
        }
        let stripped = strip_js_comments_from_text(&render_text);
        if jsx_render_contains_custom_component_boundary(&stripped) {
            return false;
        }
        let mut found_in_render = false;
        for opening in jsx_opening_tag_spans(&stripped) {
            if !jsx_opening_has_dialog_labelledby_attrs(&opening) {
                continue;
            }
            found_in_render = true;
            found_dialog_labelledby = true;
            if !jsx_element_body_has_exact_expression(&stripped, &opening, "children") {
                return false;
            }
        }
        if !found_in_render {
            return false;
        }
    }
    found_dialog_labelledby
}

fn jsx_render_contains_custom_component_boundary(render_text: &str) -> bool {
    jsx_opening_tag_spans(render_text)
        .iter()
        .any(|opening| is_uppercase_symbol(&opening.tag))
}

fn component_render_text_is_empty_ui(render_text: &str) -> bool {
    let trimmed = render_text
        .trim()
        .trim_start_matches('(')
        .trim_end_matches(')')
        .trim()
        .trim_end_matches(';')
        .trim();
    matches!(trimmed, "null" | "false" | "undefined")
}

fn component_body_has_unparsed_control_flow(body: &str) -> bool {
    let stripped = strip_js_comments_from_text(body);
    let chars: Vec<(usize, char)> = stripped.char_indices().collect();
    let mut index = 0usize;
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
        if js_keyword_at(&stripped, byte, "if")
            && js_control_keyword_has_braced_body(&stripped, byte, "if")
        {
            return true;
        }
        for keyword in ["switch", "try", "for", "while", "do"] {
            if js_keyword_at(&stripped, byte, keyword) {
                return true;
            }
        }
        index += 1;
    }
    false
}

fn js_keyword_at(text: &str, byte: usize, keyword: &str) -> bool {
    text[byte..].starts_with(keyword)
        && js_identifier_boundary_before(text, byte)
        && js_identifier_boundary_after(text, byte + keyword.len())
}

fn js_control_keyword_has_braced_body(text: &str, byte: usize, keyword: &str) -> bool {
    let mut cursor = skip_js_whitespace(text, byte + keyword.len());
    if !text[cursor..].starts_with('(') {
        return false;
    }
    let Some(after_condition) = js_balanced_call_end(text, cursor) else {
        return false;
    };
    cursor = skip_js_whitespace(text, after_condition);
    text[cursor..].starts_with('{')
}

fn js_text_contains_unparsed_render_control_flow(text: &str) -> bool {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut index = 0usize;
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
        if ch == '?' {
            return true;
        }
        if (ch == '&' && text[byte + ch.len_utf8()..].starts_with('&'))
            || (ch == '|' && text[byte + ch.len_utf8()..].starts_with('|'))
        {
            return true;
        }
    }
    false
}

fn js_text_contains_call_with_jsx_argument(text: &str) -> bool {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut index = 0usize;
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
        if !(ch.is_ascii_alphabetic() || matches!(ch, '_' | '$')) {
            index += 1;
            continue;
        }
        let ident_start = byte;
        index += 1;
        while index < chars.len() {
            let (_, next) = chars[index];
            if next.is_ascii_alphanumeric() || matches!(next, '_' | '$' | '.') {
                index += 1;
            } else {
                break;
            }
        }
        let ident_end = chars
            .get(index)
            .map(|(next_byte, _)| *next_byte)
            .unwrap_or(text.len());
        let ident = &text[ident_start..ident_end];
        if matches!(ident, "if" | "for" | "while" | "switch" | "return") {
            continue;
        }
        let cursor = skip_js_whitespace(text, ident_end);
        if text[cursor..].starts_with('(')
            && let Some(call_end) = js_balanced_call_end(text, cursor)
        {
            let args = &text[cursor + 1..call_end.saturating_sub(1)];
            if !jsx_opening_tag_spans(args).is_empty() {
                return true;
            }
        }
    }
    false
}
