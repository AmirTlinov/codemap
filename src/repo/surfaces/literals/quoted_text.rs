// Responsibility: repo-surfaces-quoted-text
use crate::repo::{js_regex_literal_can_start, js_regex_literal_end};

pub(crate) fn strip_js_comments_from_line(line: &str, in_block_comment: &mut bool) -> String {
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut index = 0;
    let mut segment_start = 0;
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    while index < bytes.len() {
        if *in_block_comment {
            if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                *in_block_comment = false;
                index = (index + 2).min(bytes.len());
                segment_start = index;
            } else {
                index += 1;
            }
            continue;
        }
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
                index += 1;
                continue;
            }
            if bytes[index] == b'\\' {
                escaped = true;
                index += 1;
                continue;
            }
            if bytes[index] == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }
        if bytes[index] == b'/' {
            out.push_str(&line[segment_start..index]);
            if js_regex_literal_can_start(&out)
                && let Some(end) = js_regex_literal_end(bytes, index)
            {
                out.push_str(&line[index..end]);
                index = end;
                segment_start = index;
                continue;
            }
            if bytes.get(index + 1) == Some(&b'/') {
                return out;
            }
            if bytes.get(index + 1) == Some(&b'*') {
                *in_block_comment = true;
                index += 2;
                segment_start = index;
                continue;
            }
            segment_start = index;
        }
        if matches!(bytes[index], b'"' | b'\'' | b'`') {
            quote = Some(bytes[index]);
            escaped = false;
        }
        index += 1;
    }
    if !*in_block_comment {
        out.push_str(&line[segment_start..]);
    }
    out
}

pub(crate) fn static_jsx_visible_text(line: &str) -> Option<String> {
    let mut out = String::new();
    let mut in_tag = false;
    let mut brace_depth = 0usize;
    for ch in line.chars() {
        if in_tag {
            if ch == '>' {
                in_tag = false;
                out.push(' ');
            }
            continue;
        }
        if brace_depth > 0 {
            if ch == '{' {
                brace_depth += 1;
            } else if ch == '}' {
                brace_depth = brace_depth.saturating_sub(1);
                if brace_depth == 0 {
                    out.push(' ');
                }
            }
            continue;
        }
        if ch == '<' {
            in_tag = true;
            out.push(' ');
            continue;
        }
        if ch == '{' {
            brace_depth = 1;
            out.push(' ');
            continue;
        }
        out.push(ch);
    }
    let text = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.len() < 3
        || text.len() > 180
        || !text.chars().any(|ch| ch.is_alphabetic())
        || text.contains('=')
    {
        return None;
    }
    Some(text)
}

#[derive(Debug)]
pub(crate) struct QuotedString {
    pub(crate) value: String,
    pub(crate) prefix: String,
}

pub(crate) fn quoted_strings(text: &str) -> Vec<QuotedString> {
    let mut values = Vec::new();
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut index = 0;
    let mut in_block_comment = false;
    while index < chars.len() {
        let (start, ch) = chars[index];
        let next = chars.get(index + 1).map(|(_, next)| *next);
        if in_block_comment {
            if ch == '*' && next == Some('/') {
                in_block_comment = false;
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }
        if ch == '/' && next == Some('/') {
            break;
        }
        if ch == '/' && next == Some('*') {
            in_block_comment = true;
            index += 2;
            continue;
        }
        if !matches!(ch, '"' | '\'' | '`') {
            index += 1;
            continue;
        }
        let quote = ch;
        let mut value = String::new();
        let mut escaped = false;
        index += 1;
        while index < chars.len() {
            let (_, inner) = chars[index];
            index += 1;
            if escaped {
                value.push(inner);
                escaped = false;
                continue;
            }
            if inner == '\\' {
                escaped = true;
                continue;
            }
            if inner == quote {
                break;
            }
            value.push(inner);
        }
        values.push(QuotedString {
            value,
            prefix: text[..start].to_string(),
        });
    }
    values
}
