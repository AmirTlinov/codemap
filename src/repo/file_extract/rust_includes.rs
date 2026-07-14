// Responsibility: repo-file-extract-rust-includes
use std::collections::BTreeSet;

pub(crate) fn extract_rust_include_specs(text: &str) -> BTreeSet<String> {
    let bytes = text.as_bytes();
    let mut specs = BTreeSet::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            let mut depth = 1usize;
            index += 2;
            while index + 1 < bytes.len() && depth > 0 {
                if bytes[index] == b'/' && bytes[index + 1] == b'*' {
                    depth += 1;
                    index += 2;
                    continue;
                }
                if bytes[index] == b'*' && bytes[index + 1] == b'/' {
                    depth = depth.saturating_sub(1);
                    index += 2;
                    continue;
                }
                index += 1;
            }
            continue;
        }
        if let Some((len, hashes)) = rust_raw_string_start(bytes, index) {
            index += len;
            while index < bytes.len() && !rust_raw_string_end_at(bytes, index, hashes) {
                index += 1;
            }
            index = (index + hashes + 1).min(bytes.len());
            continue;
        }
        if bytes[index] == b'"' {
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(bytes.len());
                    continue;
                }
                if bytes[index] == b'"' {
                    index += 1;
                    break;
                }
                index += 1;
            }
            continue;
        }
        if let Some(len) = rust_char_literal_len(bytes, index) {
            index += len;
            continue;
        }
        if rust_identifier_at(bytes, index, b"include")
            && let Some((spec, next)) = parse_rust_include_spec(bytes, index + "include".len())
        {
            specs.insert(spec);
            index = next;
            continue;
        }
        index += 1;
    }
    specs
}

pub(crate) fn rust_include_blind_spot_lines(text: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    let mut lines = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            let mut depth = 1usize;
            index += 2;
            while index + 1 < bytes.len() && depth > 0 {
                if bytes[index] == b'/' && bytes[index + 1] == b'*' {
                    depth += 1;
                    index += 2;
                    continue;
                }
                if bytes[index] == b'*' && bytes[index + 1] == b'/' {
                    depth = depth.saturating_sub(1);
                    index += 2;
                    continue;
                }
                index += 1;
            }
            continue;
        }
        if let Some((len, hashes)) = rust_raw_string_start(bytes, index) {
            index += len;
            while index < bytes.len() && !rust_raw_string_end_at(bytes, index, hashes) {
                index += 1;
            }
            index = (index + hashes + 1).min(bytes.len());
            continue;
        }
        if bytes[index] == b'"' {
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(bytes.len());
                    continue;
                }
                if bytes[index] == b'"' {
                    index += 1;
                    break;
                }
                index += 1;
            }
            continue;
        }
        if let Some(len) = rust_char_literal_len(bytes, index) {
            index += len;
            continue;
        }
        if rust_identifier_at(bytes, index, b"include")
            && let Some((static_spec, next)) =
                parse_rust_include_invocation(bytes, index + "include".len())
        {
            if !static_spec {
                lines.push(byte_line_number(text, index));
            }
            index = next;
            continue;
        }
        index += 1;
    }
    lines
}

fn parse_rust_include_invocation(bytes: &[u8], mut index: usize) -> Option<(bool, usize)> {
    index = skip_ascii_space(bytes, index);
    if bytes.get(index) != Some(&b'!') {
        return None;
    }
    index = skip_ascii_space(bytes, index + 1);
    if bytes.get(index) != Some(&b'(') {
        return None;
    }
    index = skip_ascii_space(bytes, index + 1);
    if let Some((spec, next)) = parse_rust_string_literal(bytes, index) {
        return Some((spec.ends_with(".rs"), next));
    }
    Some((false, index.saturating_add(1)))
}

fn byte_line_number(text: &str, byte_index: usize) -> usize {
    text.as_bytes()
        .iter()
        .take(byte_index)
        .filter(|byte| **byte == b'\n')
        .count()
        + 1
}

fn parse_rust_include_spec(bytes: &[u8], mut index: usize) -> Option<(String, usize)> {
    index = skip_ascii_space(bytes, index);
    if bytes.get(index) != Some(&b'!') {
        return None;
    }
    index = skip_ascii_space(bytes, index + 1);
    if bytes.get(index) != Some(&b'(') {
        return None;
    }
    index = skip_ascii_space(bytes, index + 1);
    parse_rust_string_literal(bytes, index)
        .and_then(|(spec, next)| spec.ends_with(".rs").then_some((spec, next)))
}

fn parse_rust_string_literal(bytes: &[u8], mut index: usize) -> Option<(String, usize)> {
    if let Some((len, hashes)) = rust_raw_string_start(bytes, index) {
        let start = index + len;
        index = start;
        while index < bytes.len() && !rust_raw_string_end_at(bytes, index, hashes) {
            index += 1;
        }
        if index >= bytes.len() {
            return None;
        }
        let spec = std::str::from_utf8(&bytes[start..index]).ok()?.to_string();
        return Some((spec, index + hashes + 1));
    }
    if bytes.get(index) != Some(&b'"') {
        return None;
    }
    let start = index + 1;
    index = start;
    while index < bytes.len() {
        if bytes[index] == b'\\' {
            index += 2;
            continue;
        }
        if bytes[index] == b'"' {
            let spec = std::str::from_utf8(&bytes[start..index]).ok()?.to_string();
            return Some((spec, index + 1));
        }
        index += 1;
    }
    None
}

fn skip_ascii_space(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    index
}

fn rust_identifier_at(bytes: &[u8], index: usize, ident: &[u8]) -> bool {
    bytes
        .get(index..index + ident.len())
        .is_some_and(|slice| slice == ident)
        && index
            .checked_sub(1)
            .and_then(|before| bytes.get(before))
            .is_none_or(|byte| !rust_identifier_byte(*byte))
        && bytes
            .get(index + ident.len())
            .is_none_or(|byte| !rust_identifier_byte(*byte))
}

fn rust_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn rust_char_literal_len(bytes: &[u8], index: usize) -> Option<usize> {
    if bytes.get(index) != Some(&b'\'') {
        return None;
    }
    let mut cursor = index + 1;
    if cursor >= bytes.len() || bytes[cursor] == b'\n' {
        return None;
    }
    if bytes[cursor] == b'\\' {
        cursor = cursor.saturating_add(2);
    } else {
        while cursor < bytes.len() && bytes[cursor] != b'\'' && bytes[cursor] != b'\n' {
            cursor += 1;
            if cursor.saturating_sub(index) > 8 {
                return None;
            }
        }
    }
    (bytes.get(cursor) == Some(&b'\'')).then_some(cursor - index + 1)
}

pub(crate) fn rust_raw_string_start(bytes: &[u8], index: usize) -> Option<(usize, usize)> {
    if index
        .checked_sub(1)
        .and_then(|previous| bytes.get(previous))
        .copied()
        .is_some_and(rust_identifier_byte)
    {
        return None;
    }
    let mut cursor = index;
    if bytes.get(cursor) == Some(&b'b') && bytes.get(cursor + 1) == Some(&b'r') {
        cursor += 2;
    } else if bytes.get(cursor) == Some(&b'r') {
        cursor += 1;
    } else {
        return None;
    }
    let mut hashes = 0usize;
    while bytes.get(cursor) == Some(&b'#') {
        hashes += 1;
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b'"') {
        return None;
    }
    Some((cursor - index + 1, hashes))
}

pub(crate) fn rust_raw_string_end_at(bytes: &[u8], index: usize, hashes: usize) -> bool {
    bytes.get(index) == Some(&b'"')
        && (0..hashes).all(|offset| bytes.get(index + 1 + offset) == Some(&b'#'))
}
