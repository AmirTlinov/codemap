// Responsibility: repo-js-scanner
use crate::repo::{
    identifier_re, js_regex_literal_can_start, js_regex_literal_end, strip_js_comments_from_line,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
pub(crate) enum JsScanState {
    Code,
    LineComment,
    BlockComment,
    Quoted(u8),
    Template,
}

pub(crate) fn js_keyword_positions(text: &str, keyword: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    let keyword_bytes = keyword.as_bytes();
    let mut out = Vec::new();
    let mut index = 0;
    let mut state = JsScanState::Code;
    while index < bytes.len() {
        match state {
            JsScanState::Code => {
                if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
                    state = JsScanState::LineComment;
                    index += 2;
                    continue;
                }
                if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
                    state = JsScanState::BlockComment;
                    index += 2;
                    continue;
                }
                if bytes[index] == b'/'
                    && js_regex_literal_can_start(
                        std::string::String::from_utf8_lossy(&bytes[..index]).as_ref(),
                    )
                    && let Some(end) = js_regex_literal_end(bytes, index)
                {
                    index = end;
                    continue;
                }
                if matches!(bytes[index], b'\'' | b'"') {
                    state = JsScanState::Quoted(bytes[index]);
                } else if bytes[index] == b'`' {
                    state = JsScanState::Template;
                } else if bytes[index..].starts_with(keyword_bytes)
                    && js_keyword_boundary(bytes, index, keyword_bytes.len())
                {
                    out.push(index);
                    index += keyword_bytes.len();
                    continue;
                }
            }
            JsScanState::LineComment => {
                if bytes[index] == b'\n' {
                    state = JsScanState::Code;
                }
            }
            JsScanState::BlockComment => {
                if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                    state = JsScanState::Code;
                    index += 2;
                    continue;
                }
            }
            JsScanState::Quoted(quote) => {
                if bytes[index] == b'\\' {
                    index = index.saturating_add(2);
                    continue;
                }
                if bytes[index] == quote {
                    state = JsScanState::Code;
                }
            }
            JsScanState::Template => {
                if bytes[index] == b'\\' {
                    index = index.saturating_add(2);
                    continue;
                }
                if bytes[index] == b'`' {
                    state = JsScanState::Code;
                }
            }
        }
        index += 1;
    }
    out
}

fn js_keyword_boundary(bytes: &[u8], start: usize, len: usize) -> bool {
    let before = start
        .checked_sub(1)
        .and_then(|index| bytes.get(index))
        .copied();
    let after = bytes.get(start + len).copied();
    !before.map(is_js_identifier_byte).unwrap_or(false)
        && !after.map(is_js_identifier_byte).unwrap_or(false)
}

fn is_js_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$')
}

pub(crate) fn skip_ascii_whitespace(text: &str, mut index: usize) -> usize {
    let bytes = text.as_bytes();
    while bytes
        .get(index)
        .map(|byte| byte.is_ascii_whitespace())
        .unwrap_or(false)
    {
        index += 1;
    }
    index
}

pub(crate) fn parse_js_import_clause_bindings(clause: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let clause = clause.trim();
    if clause.is_empty() || clause.starts_with('{') {
        collect_js_named_import_bindings(clause, &mut out);
        return out;
    }
    if let Some(namespace) = clause.strip_prefix("* as ") {
        if let Some(name) = first_identifier(namespace) {
            out.insert(name, "*".to_string());
        }
        return out;
    }
    if let Some((default, rest)) = clause.split_once(',') {
        if let Some(name) = first_identifier(default) {
            out.insert(name, "default".to_string());
        }
        collect_js_named_import_bindings(rest, &mut out);
    } else if let Some(name) = first_identifier(clause) {
        out.insert(name, "default".to_string());
    }
    out
}

pub(crate) fn collect_js_named_import_bindings(clause: &str, out: &mut BTreeMap<String, String>) {
    let Some(start) = clause.find('{') else {
        return;
    };
    let Some(end) = clause.rfind('}') else {
        return;
    };
    let inner = strip_js_comments_from_text(&clause[start + 1..end]);
    for part in inner.split(',') {
        let part = part.trim();
        if part.is_empty() || part.starts_with("type ") {
            continue;
        }
        let (imported, local) = part
            .split_once(" as ")
            .map(|(imported, alias)| (imported.trim(), alias.trim()))
            .unwrap_or((part, part));
        let Some(imported_name) = first_identifier(imported) else {
            continue;
        };
        if let Some(local_name) = first_identifier(local) {
            out.insert(local_name, imported_name);
        }
    }
}

pub(crate) fn strip_js_comments_from_text(text: &str) -> String {
    let mut out = String::new();
    let mut in_block_comment = false;
    for line in text.lines() {
        out.push_str(&strip_js_comments_from_line(line, &mut in_block_comment));
        out.push('\n');
    }
    out
}

pub(crate) fn first_identifier(value: &str) -> Option<String> {
    identifier_re()
        .find(value.trim())
        .map(|m| m.as_str().to_string())
}

pub(crate) fn identifier_is_selector_tail(text: &str, start: usize) -> bool {
    text[..start]
        .chars()
        .rev()
        .find(|ch| !ch.is_whitespace())
        .map(|ch| ch == '.')
        .unwrap_or(false)
}
