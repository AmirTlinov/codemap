// Responsibility: runtime-lens-code-shape
use std::collections::BTreeMap;

pub(crate) fn quoted_literal_at(value: &str) -> Option<String> {
    let value = value.trim_start();
    let quote = value.chars().next()?;
    if quote != '"' && quote != '\'' && quote != '`' {
        return None;
    }
    let mut escaped = false;
    let mut close = None;
    for (offset, ch) in value[1..].char_indices() {
        if escaped {
            escaped = false;
        } else if ch == '\\' && quote != '`' {
            escaped = true;
        } else if ch == quote {
            close = Some(1 + offset);
            break;
        }
    }
    let close = close?;
    let literal = &value[1..close];
    if quote == '`' && literal.contains("${") {
        return None;
    }
    let after = value[close + quote.len_utf8()..].trim_start();
    if after
        .chars()
        .next()
        .is_some_and(|ch| !matches!(ch, ',' | ')' | ']' | '}' | ';'))
    {
        return None;
    }
    Some(literal.to_string())
}

pub(crate) fn quoted_literal_contents(line: &str) -> Vec<String> {
    let chars = line.chars().collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        let quote = chars[index];
        if !matches!(quote, '"' | '\'' | '`') {
            index += 1;
            continue;
        }
        let mut literal = String::new();
        let mut escaped = false;
        index += 1;
        while index < chars.len() {
            let ch = chars[index];
            if escaped {
                literal.push(ch);
                escaped = false;
            } else if ch == '\\' && quote != '`' {
                escaped = true;
            } else if ch == quote {
                out.push(literal);
                break;
            } else {
                literal.push(ch);
            }
            index += 1;
        }
        index += 1;
    }
    out
}

fn push_masked_char(out: &mut String, ch: char) {
    out.extend(std::iter::repeat_n(' ', ch.len_utf8()));
}

fn push_masked_chars(out: &mut String, chars: &[char]) {
    for ch in chars {
        push_masked_char(out, *ch);
    }
}

fn mask_cross_line_runtime_context(
    line: &str,
    in_block_comment: &mut bool,
    in_template_literal: &mut bool,
    in_triple_quote: &mut Option<char>,
) -> String {
    let chars = line.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(line.len());
    let mut index = 0;
    let mut quote = None;
    let mut escaped = false;
    while index < chars.len() {
        let ch = chars[index];
        let next = chars.get(index + 1).copied();
        if let Some(quote) = *in_triple_quote {
            push_masked_char(&mut out, ch);
            if ch == quote
                && chars.get(index + 1) == Some(&quote)
                && chars.get(index + 2) == Some(&quote)
            {
                out.push(' ');
                out.push(' ');
                *in_triple_quote = None;
                index += 3;
            } else {
                index += 1;
            }
            continue;
        }
        if *in_template_literal {
            push_masked_char(&mut out, ch);
            if !escaped && ch == '`' {
                *in_template_literal = false;
            }
            escaped = !escaped && ch == '\\';
            index += 1;
            continue;
        }
        if *in_block_comment {
            if ch == '*' && next == Some('/') {
                out.push(' ');
                out.push(' ');
                *in_block_comment = false;
                index += 2;
            } else {
                push_masked_char(&mut out, ch);
                index += 1;
            }
            continue;
        }
        if let Some(active_quote) = quote {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' && active_quote != '`' {
                escaped = true;
            } else if ch == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }
        if (ch == '"' || ch == '\'')
            && chars.get(index + 1) == Some(&ch)
            && chars.get(index + 2) == Some(&ch)
        {
            out.push(' ');
            out.push(' ');
            out.push(' ');
            *in_triple_quote = Some(ch);
            index += 3;
            continue;
        }
        if matches!(ch, '"' | '\'' | '`') {
            if ch == '`' && !template_literal_closes_on_line(&chars, index + 1) {
                push_masked_char(&mut out, ch);
                *in_template_literal = true;
                index += 1;
                continue;
            }
            quote = Some(ch);
            escaped = false;
            out.push(ch);
            index += 1;
            continue;
        }
        if ch == '/' && next == Some('/') {
            out.extend(chars[index..].iter());
            break;
        }
        if ch == '/' && next == Some('*') {
            out.push(' ');
            out.push(' ');
            *in_block_comment = true;
            index += 2;
            continue;
        }
        if ch == '/' && regex_literal_can_start(&out) {
            out.push('/');
            index += 1;
            let mut escaped_regex = false;
            let mut in_class = false;
            while index < chars.len() {
                let regex_ch = chars[index];
                out.push(regex_ch);
                if escaped_regex {
                    escaped_regex = false;
                } else if regex_ch == '\\' {
                    escaped_regex = true;
                } else if regex_ch == '[' {
                    in_class = true;
                } else if regex_ch == ']' {
                    in_class = false;
                } else if regex_ch == '/' && !in_class {
                    index += 1;
                    while index < chars.len() && chars[index].is_ascii_alphabetic() {
                        out.push(chars[index]);
                        index += 1;
                    }
                    break;
                }
                index += 1;
            }
            continue;
        }
        out.push(ch);
        index += 1;
    }
    out
}

pub(crate) fn runtime_code_lines(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut in_block_comment = false;
    let mut in_template_literal = false;
    let mut in_triple_quote = None;
    for (index, line) in text.lines().enumerate() {
        let line = mask_cross_line_runtime_context(
            line,
            &mut in_block_comment,
            &mut in_template_literal,
            &mut in_triple_quote,
        );
        if line_is_comment(&line) || line.trim().is_empty() {
            continue;
        }
        out.push((index + 1, line));
    }
    out
}

pub(crate) fn runtime_code_line_lookup(text: &str) -> BTreeMap<usize, String> {
    runtime_code_lines(text).into_iter().collect()
}

fn template_literal_closes_on_line(chars: &[char], mut index: usize) -> bool {
    let mut escaped = false;
    while index < chars.len() {
        let ch = chars[index];
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '`' {
            return true;
        }
        index += 1;
    }
    false
}

pub(crate) fn code_shape_without_literal_content(line: &str) -> String {
    let chars = line.chars().collect::<Vec<_>>();
    let mut out = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        let next = chars.get(index + 1).copied();
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
                push_masked_char(&mut out, ch);
            } else if ch == '\\' && active_quote != '`' {
                escaped = true;
                push_masked_char(&mut out, ch);
            } else if ch == active_quote {
                quote = None;
                out.push(ch);
            } else {
                push_masked_char(&mut out, ch);
            }
            index += 1;
            continue;
        }
        if ch == '/' && next == Some('/') {
            push_masked_chars(&mut out, &chars[index..]);
            break;
        }
        if ch == '/' && next == Some('*') {
            out.push(' ');
            out.push(' ');
            index += 2;
            while index < chars.len() {
                if chars[index] == '*' && chars.get(index + 1) == Some(&'/') {
                    out.push(' ');
                    out.push(' ');
                    index += 2;
                    break;
                }
                push_masked_char(&mut out, chars[index]);
                index += 1;
            }
            continue;
        }
        if ch == '/' && regex_literal_can_start(&out) {
            out.push('/');
            index += 1;
            let mut escaped_regex = false;
            let mut in_class = false;
            while index < chars.len() {
                let regex_ch = chars[index];
                if escaped_regex {
                    escaped_regex = false;
                    push_masked_char(&mut out, regex_ch);
                } else if regex_ch == '\\' {
                    escaped_regex = true;
                    push_masked_char(&mut out, regex_ch);
                } else if regex_ch == '[' {
                    in_class = true;
                    push_masked_char(&mut out, regex_ch);
                } else if regex_ch == ']' {
                    in_class = false;
                    push_masked_char(&mut out, regex_ch);
                } else if regex_ch == '/' && !in_class {
                    out.push('/');
                    index += 1;
                    while index < chars.len() && chars[index].is_ascii_alphabetic() {
                        out.push(' ');
                        index += 1;
                    }
                    break;
                } else {
                    push_masked_char(&mut out, regex_ch);
                }
                index += 1;
            }
            continue;
        }
        if ch == '#' {
            push_masked_chars(&mut out, &chars[index..]);
            break;
        }
        if matches!(ch, '"' | '\'' | '`') {
            quote = Some(ch);
            escaped = false;
            out.push(ch);
            index += 1;
            continue;
        }
        out.push(ch);
        index += 1;
    }
    out
}

fn regex_literal_can_start(code_before: &str) -> bool {
    let trimmed = code_before.trim_end();
    if trimmed.is_empty() {
        return true;
    }
    if trimmed.ends_with("return")
        || trimmed.ends_with("case")
        || trimmed.ends_with("throw")
        || trimmed.ends_with("typeof")
    {
        return true;
    }
    trimmed.chars().next_back().is_some_and(|ch| {
        matches!(
            ch,
            '(' | '[' | '{' | '=' | ':' | ',' | ';' | '!' | '?' | '&' | '|'
        )
    })
}

fn line_is_comment(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//")
        || trimmed.starts_with('#')
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
}
