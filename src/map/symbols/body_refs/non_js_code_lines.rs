// Responsibility: map-symbols-non-js-code-lines
use crate::map::is_identifier_byte;

pub(crate) fn non_js_code_line_without_strings_and_comments(
    line: &str,
    ext: &str,
    state: &mut NonJsCodeState,
) -> String {
    if ext == "py" {
        return python_code_line_without_strings_and_comments(line, state);
    }
    c_like_code_line_without_strings_and_comments(line, ext, state)
}

#[derive(Debug, Default)]
pub(crate) struct NonJsCodeState {
    in_block_comment: bool,
    quote: Option<NonJsQuoteState>,
    escaped: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NonJsQuoteState {
    Quoted(u8),
    PythonTriple(u8),
    SwiftTriple,
    GoRaw,
    RustRaw { hashes: usize },
}

fn python_code_line_without_strings_and_comments(line: &str, state: &mut NonJsCodeState) -> String {
    let bytes = line.as_bytes();
    let mut out = String::new();
    let mut index = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(active_quote) = state.quote {
            if let NonJsQuoteState::PythonTriple(quote_byte) = active_quote {
                if python_triple_quote_at(bytes, index, quote_byte) {
                    state.quote = None;
                    push_spaces(&mut out, 3);
                    index += 3;
                } else {
                    out.push(' ');
                    index += 1;
                }
                continue;
            }
            let NonJsQuoteState::Quoted(active_quote) = active_quote else {
                state.quote = None;
                continue;
            };
            if state.escaped {
                out.push(' ');
                state.escaped = false;
                index += 1;
                continue;
            }
            if byte == b'\\' {
                state.escaped = true;
                out.push(' ');
                index += 1;
                continue;
            }
            if byte == active_quote {
                state.quote = None;
            }
            out.push(' ');
            index += 1;
            continue;
        }
        if byte == b'#' {
            break;
        }
        if let Some((len, quote_state)) = python_string_start(bytes, index) {
            state.quote = Some(quote_state);
            state.escaped = false;
            push_spaces(&mut out, len);
            index += len;
            continue;
        }
        out.push(byte as char);
        index += 1;
    }
    out
}

pub(crate) fn c_like_code_line_without_strings_and_comments(
    line: &str,
    ext: &str,
    state: &mut NonJsCodeState,
) -> String {
    let bytes = line.as_bytes();
    let mut out = String::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if state.in_block_comment {
            if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                state.in_block_comment = false;
                push_spaces(&mut out, 2);
                index += 2;
            } else {
                out.push(' ');
                index += 1;
            }
            continue;
        }
        if let Some(active_quote) = state.quote {
            match active_quote {
                NonJsQuoteState::Quoted(quote_byte) => {
                    if state.escaped {
                        state.escaped = false;
                        out.push(' ');
                        index += 1;
                        continue;
                    }
                    if bytes[index] == b'\\' {
                        state.escaped = true;
                        out.push(' ');
                        index += 1;
                        continue;
                    }
                    if bytes[index] == quote_byte {
                        state.quote = None;
                    }
                    out.push(' ');
                    index += 1;
                }
                NonJsQuoteState::SwiftTriple => {
                    if bytes.get(index..index + 3) == Some(b"\"\"\"") {
                        state.quote = None;
                        push_spaces(&mut out, 3);
                        index += 3;
                    } else {
                        out.push(' ');
                        index += 1;
                    }
                }
                NonJsQuoteState::GoRaw => {
                    if bytes[index] == b'`' {
                        state.quote = None;
                    }
                    out.push(' ');
                    index += 1;
                }
                NonJsQuoteState::RustRaw { hashes } => {
                    if rust_raw_string_end_at(bytes, index, hashes) {
                        state.quote = None;
                        push_spaces(&mut out, hashes + 1);
                        index += hashes + 1;
                    } else {
                        out.push(' ');
                        index += 1;
                    }
                }
                NonJsQuoteState::PythonTriple(_) => {
                    state.quote = None;
                }
            }
            continue;
        }
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            break;
        }
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            state.in_block_comment = true;
            push_spaces(&mut out, 2);
            index += 2;
            continue;
        }
        if ext == "rs"
            && let Some((len, hashes)) = rust_raw_string_start(bytes, index)
        {
            state.quote = Some(NonJsQuoteState::RustRaw { hashes });
            state.escaped = false;
            push_spaces(&mut out, len);
            index += len;
            continue;
        }
        if ext == "swift" && bytes.get(index..index + 3) == Some(b"\"\"\"") {
            state.quote = Some(NonJsQuoteState::SwiftTriple);
            state.escaped = false;
            push_spaces(&mut out, 3);
            index += 3;
            continue;
        }
        if ext == "go" && bytes[index] == b'`' {
            state.quote = Some(NonJsQuoteState::GoRaw);
            state.escaped = false;
            out.push(' ');
            index += 1;
            continue;
        }
        if c_like_quote_starts(ext, bytes, index) {
            state.quote = Some(NonJsQuoteState::Quoted(bytes[index]));
            state.escaped = false;
            out.push(' ');
            index += 1;
            continue;
        }
        out.push(bytes[index] as char);
        index += 1;
    }
    out
}

fn python_string_start(bytes: &[u8], index: usize) -> Option<(usize, NonJsQuoteState)> {
    if matches!(bytes.get(index), Some(b'"' | b'\'')) {
        let quote = bytes[index];
        if python_triple_quote_at(bytes, index, quote) {
            return Some((3, NonJsQuoteState::PythonTriple(quote)));
        }
        return Some((1, NonJsQuoteState::Quoted(quote)));
    }
    let first = *bytes.get(index)?;
    if !matches!(first, b'r' | b'R' | b'u' | b'U' | b'b' | b'B' | b'f' | b'F') {
        return None;
    }
    if index
        .checked_sub(1)
        .and_then(|previous| bytes.get(previous))
        .copied()
        .map(is_identifier_byte)
        .unwrap_or(false)
    {
        return None;
    }
    let mut cursor = index;
    while matches!(
        bytes.get(cursor),
        Some(b'r' | b'R' | b'u' | b'U' | b'b' | b'B' | b'f' | b'F')
    ) {
        cursor += 1;
    }
    let quote = *bytes.get(cursor)?;
    if !matches!(quote, b'"' | b'\'') {
        return None;
    }
    let prefix_len = cursor - index;
    if python_triple_quote_at(bytes, cursor, quote) {
        Some((prefix_len + 3, NonJsQuoteState::PythonTriple(quote)))
    } else {
        Some((prefix_len + 1, NonJsQuoteState::Quoted(quote)))
    }
}

fn python_triple_quote_at(bytes: &[u8], index: usize, quote: u8) -> bool {
    bytes.get(index) == Some(&quote)
        && bytes.get(index + 1) == Some(&quote)
        && bytes.get(index + 2) == Some(&quote)
}

pub(crate) fn rust_raw_string_start(bytes: &[u8], index: usize) -> Option<(usize, usize)> {
    if index
        .checked_sub(1)
        .and_then(|previous| bytes.get(previous))
        .copied()
        .map(is_identifier_byte)
        .unwrap_or(false)
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

fn c_like_quote_starts(ext: &str, bytes: &[u8], index: usize) -> bool {
    match bytes[index] {
        b'"' => true,
        b'\'' if ext == "rs" => rust_single_quote_starts_char_literal(bytes, index),
        b'\'' if ext == "go" => true,
        _ => false,
    }
}

fn rust_single_quote_starts_char_literal(bytes: &[u8], index: usize) -> bool {
    if bytes.get(index + 1) == Some(&b'\\') {
        return bytes
            .get(index + 2..index + 6)
            .map(|tail| tail.contains(&b'\''))
            .unwrap_or(false);
    }
    bytes.get(index + 2) == Some(&b'\'')
}

fn push_spaces(out: &mut String, count: usize) {
    for _ in 0..count {
        out.push(' ');
    }
}
