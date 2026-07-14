// Responsibility: repo-js-code-strip
pub(crate) fn code_without_comments_or_strings(text: &str, ext: &str) -> String {
    let mut out = String::new();
    let mut code_state = CodeStripState::default();
    for raw_line in text.lines() {
        let comment_stripped = match ext {
            "py" => strip_python_comment_from_line(raw_line),
            "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte" | "rs" | "go"
            | "swift" => strip_c_like_code_line_for_identifier_refs(raw_line, &mut code_state),
            _ => raw_line.to_string(),
        };
        if ext == "py" {
            out.push_str(&strip_string_literals_from_line(&comment_stripped));
        } else {
            out.push_str(&comment_stripped);
        }
        out.push('\n');
    }
    out
}

#[derive(Debug, Default)]
struct CodeStripState {
    in_block_comment: bool,
    quote: Option<u8>,
    escaped: bool,
}

fn strip_c_like_code_line_for_identifier_refs(line: &str, state: &mut CodeStripState) -> String {
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut index = 0;
    let mut segment_start = 0;
    while index < bytes.len() {
        if state.in_block_comment {
            if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                state.in_block_comment = false;
                index += 2;
                segment_start = index;
            } else {
                index += 1;
            }
            continue;
        }
        if let Some(active_quote) = state.quote {
            if state.escaped {
                state.escaped = false;
            } else if bytes[index] == b'\\' && active_quote != b'`' {
                state.escaped = true;
            } else if bytes[index] == active_quote {
                state.quote = None;
                index += 1;
                segment_start = index;
                continue;
            }
            index += 1;
            continue;
        }
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            out.push_str(&line[segment_start..index]);
            return out;
        }
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            out.push_str(&line[segment_start..index]);
            out.push(' ');
            state.in_block_comment = true;
            index += 2;
            segment_start = index;
            continue;
        }
        if matches!(bytes[index], b'"' | b'\'' | b'`') {
            out.push_str(&line[segment_start..index]);
            out.push(' ');
            state.quote = Some(bytes[index]);
            state.escaped = false;
            index += 1;
            segment_start = index;
            continue;
        }
        index += 1;
    }
    if state.quote.is_none() && !state.in_block_comment {
        out.push_str(&line[segment_start..]);
    }
    out
}

fn strip_python_comment_from_line(line: &str) -> String {
    let mut out = String::new();
    let mut quote = None;
    let mut escaped = false;
    for ch in line.chars() {
        if let Some(active_quote) = quote {
            out.push(ch);
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
        if ch == '#' {
            break;
        }
        if matches!(ch, '"' | '\'') {
            quote = Some(ch);
            escaped = false;
        }
        out.push(ch);
    }
    out
}

fn strip_string_literals_from_line(line: &str) -> String {
    let mut out = String::new();
    let mut quote = None;
    let mut escaped = false;
    for ch in line.chars() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active_quote {
                quote = None;
            }
            out.push(' ');
            continue;
        }
        if matches!(ch, '"' | '\'' | '`') {
            quote = Some(ch);
            escaped = false;
            out.push(' ');
            continue;
        }
        out.push(ch);
    }
    out
}

pub(crate) fn language_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "async"
            | "await"
            | "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "crate"
            | "def"
            | "defer"
            | "else"
            | "enum"
            | "export"
            | "extends"
            | "false"
            | "fn"
            | "for"
            | "from"
            | "func"
            | "function"
            | "if"
            | "impl"
            | "import"
            | "in"
            | "internal"
            | "interface"
            | "let"
            | "match"
            | "mod"
            | "mut"
            | "nil"
            | "none"
            | "null"
            | "package"
            | "private"
            | "protocol"
            | "pub"
            | "public"
            | "return"
            | "self"
            | "static"
            | "struct"
            | "super"
            | "switch"
            | "this"
            | "trait"
            | "true"
            | "type"
            | "undefined"
            | "use"
            | "var"
            | "where"
            | "while"
    )
}
