// Responsibility: runtime-lens-route-handlers
use crate::map::{
    is_identifier_char, matching_close_paren, object_field_value_start, route_chain_segment,
    top_level_chain_call_offset,
};

pub(crate) fn route_call_second_arg_identifier(
    line: &str,
    code: &str,
    arg_start: usize,
) -> Option<String> {
    let open_paren = code[..arg_start].rfind('(')?;
    let close_paren = matching_close_paren(code, open_paren)?;
    let comma = top_level_comma(code, arg_start, close_paren)?;
    simple_identifier_argument(line, code, comma + 1, close_paren)
}

pub(crate) fn route_call_handler_and_middleware_identifiers(
    line: &str,
    code: &str,
    arg_start: usize,
) -> (Option<String>, Vec<String>) {
    let Some(open_paren) = code[..arg_start].rfind('(') else {
        return (None, Vec::new());
    };
    let Some(close_paren) = matching_close_paren(code, open_paren) else {
        return (None, Vec::new());
    };
    let Some(first_comma) = top_level_comma(code, arg_start, close_paren) else {
        return (None, Vec::new());
    };
    handler_and_middleware_identifiers(line, code, first_comma + 1, close_paren)
}

pub(crate) fn route_chain_method_handler_and_middleware_identifiers(
    line: &str,
    code: &str,
    chain_start: usize,
    method: &str,
) -> (Option<String>, Vec<String>) {
    let chain = route_chain_segment(&code[chain_start..]);
    let call = format!(".{method}(");
    let Some(method_start) = top_level_chain_call_offset(chain, &call) else {
        return (None, Vec::new());
    };
    let open_paren = chain_start + method_start + call.len() - 1;
    let Some(close_paren) = matching_close_paren(code, open_paren) else {
        return (None, Vec::new());
    };
    handler_and_middleware_identifiers(line, code, open_paren + 1, close_paren)
}

fn handler_and_middleware_identifiers(
    line: &str,
    code: &str,
    start: usize,
    end: usize,
) -> (Option<String>, Vec<String>) {
    let mut arguments = Vec::new();
    let mut argument_start = start;
    while argument_start < end {
        let argument_end = top_level_comma(code, argument_start, end).unwrap_or(end);
        if let Some(identifier) =
            simple_identifier_argument(line, code, argument_start, argument_end)
        {
            arguments.push(identifier);
        } else {
            return (None, Vec::new());
        }
        if argument_end == end {
            break;
        }
        argument_start = argument_end + 1;
    }
    let handler = arguments.pop();
    (handler, arguments)
}

pub(crate) fn object_field_identifier(line: &str, code: &str, field: &str) -> Option<String> {
    let value_start = object_field_value_start(line, code, field)?;
    let end = top_level_delimiter(code, value_start, code.len(), &[',', '}']).unwrap_or(code.len());
    simple_identifier_argument(line, code, value_start, end)
}

pub(crate) fn simple_identifier_argument(
    line: &str,
    code: &str,
    start: usize,
    end: usize,
) -> Option<String> {
    let leading = code[start..end]
        .chars()
        .take_while(|ch| ch.is_whitespace())
        .map(char::len_utf8)
        .sum::<usize>();
    let ident_start = start + leading;
    let mut chars = code[ident_start..end].char_indices();
    let (_, first) = chars.next()?;
    if !is_identifier_start(first) {
        return None;
    }
    let mut ident_end = ident_start + first.len_utf8();
    for (offset, ch) in chars {
        if is_identifier_char(ch) {
            ident_end = ident_start + offset + ch.len_utf8();
        } else {
            break;
        }
    }
    if !code[ident_end..end].trim().is_empty() {
        return None;
    }
    Some(line[ident_start..ident_end].to_string())
}

pub(crate) fn top_level_comma(code: &str, start: usize, end: usize) -> Option<usize> {
    top_level_delimiter(code, start, end, &[','])
}

fn top_level_delimiter(code: &str, start: usize, end: usize, delimiters: &[char]) -> Option<usize> {
    let mut depth = 0usize;
    let mut index = start;
    while index < end {
        let ch = code[index..].chars().next()?;
        if depth == 0 && delimiters.contains(&ch) {
            return Some(index);
        }
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
        index += ch.len_utf8();
    }
    None
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_' || ch == '$'
}
