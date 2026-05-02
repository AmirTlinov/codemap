fn route_call_second_arg_identifier(
    line: &str,
    code: &str,
    arg_start: usize,
) -> Option<String> {
    let open_paren = code[..arg_start].rfind('(')?;
    let close_paren = matching_close_paren(code, open_paren)?;
    let comma = top_level_comma(code, arg_start, close_paren)?;
    simple_identifier_argument(line, code, comma + 1, close_paren)
}

fn route_chain_method_handler_identifier(
    line: &str,
    code: &str,
    chain_start: usize,
    method: &str,
) -> Option<String> {
    let chain = route_chain_segment(&code[chain_start..]);
    let call = format!(".{method}(");
    let method_start = top_level_chain_call_offset(chain, &call)?;
    let open_paren = chain_start + method_start + call.len() - 1;
    let close_paren = matching_close_paren(code, open_paren)?;
    simple_identifier_argument(line, code, open_paren + 1, close_paren)
}

fn object_field_identifier(line: &str, code: &str, field: &str) -> Option<String> {
    let value_start = object_field_value_start(line, code, field)?;
    let end = top_level_delimiter(code, value_start, code.len(), &[',', '}']).unwrap_or(code.len());
    simple_identifier_argument(line, code, value_start, end)
}

fn simple_identifier_argument(
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

fn top_level_comma(code: &str, start: usize, end: usize) -> Option<usize> {
    top_level_delimiter(code, start, end, &[','])
}

fn top_level_delimiter(
    code: &str,
    start: usize,
    end: usize,
    delimiters: &[char],
) -> Option<usize> {
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
