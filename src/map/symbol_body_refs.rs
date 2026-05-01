fn symbol_body_declares_js_local_binding(body: &str, name: &str, ext: &str) -> bool {
    if repo::extract_local_bindings(body, ext).contains(name) {
        return true;
    }
    let mut in_block_comment = false;
    let mut quote = None;
    for line in body.lines() {
        let code =
            js_code_line_without_strings_and_comments(line, &mut in_block_comment, &mut quote);
        if js_line_declares_local_binding(&code, name) {
            return true;
        }
    }
    false
}

fn js_line_declares_local_binding(line: &str, name: &str) -> bool {
    identifier_ranges(line, name).any(|(start, end)| {
        let before = &line[..start];
        let after = &line[end..];
        previous_word_is(before, "const")
            || previous_word_is(before, "let")
            || previous_word_is(before, "var")
            || previous_word_is(before, "function")
            || previous_word_is(before, "class")
            || identifier_occurrence_is_declaration_param(line, start, end)
            || ((previous_nonspace_byte(before) == Some(b'{')
                || previous_nonspace_byte(before) == Some(b'['))
                && next_nonspace_byte(after)
                    .map(|byte| matches!(byte, b'}' | b':' | b',' | b'='))
                    .unwrap_or(false)
                && identifier_occurrence_is_binding_pattern(line, start))
    })
}

fn identifier_occurrence_is_declaration_param(line: &str, start: usize, end: usize) -> bool {
    let before = &line[..start];
    let after = &line[end..];
    if after.trim_start().starts_with("=>") {
        return true;
    }
    let Some(open) = before.rfind('(') else {
        return false;
    };
    let Some(close_after) = after.find(')') else {
        return false;
    };
    let before_open = before[..open].trim_end();
    let tail = after[close_after + 1..].trim_start();
    if before_open.contains("function") || before_open.ends_with("catch") {
        return true;
    }
    if tail.starts_with("=>") {
        return true;
    }
    if tail.starts_with('{') {
        let name_before_params = before_open
            .rsplit(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'))
            .find(|part| !part.is_empty())
            .unwrap_or_default();
        return !matches!(
            name_before_params,
            "if" | "for" | "while" | "switch" | "catch" | "with"
        );
    }
    false
}

fn identifier_occurrence_is_binding_pattern(line: &str, start: usize) -> bool {
    let before = &line[..start];
    let opener = before.rfind(['{', '[']).unwrap_or(0);
    let before_opener = before[..opener].trim_end();
    previous_word_is(before_opener, "const")
        || previous_word_is(before_opener, "let")
        || previous_word_is(before_opener, "var")
        || before_opener.ends_with('(')
        || before_opener.ends_with(',')
}

fn symbol_body_text(project: &Project, info: &FileInfo, symbol_name: &str) -> Option<String> {
    let symbols = matching_symbols(info, symbol_name);
    if symbols.is_empty() {
        return None;
    }
    let line_start = symbols.iter().map(|symbol| symbol.line_start).min()?;
    let line_end = symbols
        .iter()
        .map(|symbol| symbol.line_end)
        .max()
        .unwrap_or(line_start);
    let text = std::fs::read_to_string(project.root.join(&info.rel)).ok()?;
    Some(
        text.lines()
            .skip(line_start.saturating_sub(1))
            .take(line_end.saturating_sub(line_start).saturating_add(1))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn imported_binding_target_symbol_name(
    project: &Project,
    target_rel: &str,
    imported: &str,
) -> Option<String> {
    let target = project.files.get(target_rel)?;
    if imported == "default" {
        let name = default_export_symbol_name(project, target_rel)?;
        return (!matching_symbols(target, &name).is_empty()).then_some(name);
    }
    (!matching_symbols(target, imported).is_empty()).then(|| imported.to_string())
}

fn symbol_body_references_imported_local(body: &str, local: &str, ext: &str) -> bool {
    if !matches!(
        ext,
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte"
    ) {
        return non_js_symbol_body_references_local(body, local, ext);
    }
    let mut in_block_comment = false;
    let mut quote = None;
    let mut type_brace_depth: Option<usize> = None;
    let jsx_capable = matches!(ext, "tsx" | "jsx" | "vue" | "svelte");
    for line in body.lines() {
        let trimmed = line.trim_start();
        let code =
            js_code_line_without_strings_and_comments(line, &mut in_block_comment, &mut quote);
        if let Some(depth) = type_brace_depth.as_mut() {
            *depth = js_brace_depth_after_line(*depth, &code);
            if js_type_context_line_is_complete(trimmed, *depth) {
                type_brace_depth = None;
            }
            continue;
        }
        if js_type_context_line_starts(trimmed) {
            let depth = js_brace_depth_after_line(0, &code);
            if !js_type_context_line_is_complete(trimmed, depth) {
                type_brace_depth = Some(depth);
            }
            continue;
        }
        if line_has_value_identifier_reference(&code, local)
            || (jsx_capable && line_has_jsx_tag_identifier_reference(&code, local))
        {
            return true;
        }
    }
    false
}

fn non_js_symbol_body_references_local(body: &str, local: &str, ext: &str) -> bool {
    let mut state = NonJsCodeState::default();
    for line in body.lines() {
        let code = non_js_code_line_without_strings_and_comments(line, ext, &mut state);
        if line_has_value_identifier_reference(&code, local) {
            return true;
        }
    }
    false
}

fn non_js_code_line_without_strings_and_comments(
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
struct NonJsCodeState {
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

fn c_like_code_line_without_strings_and_comments(
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

fn rust_raw_string_start(bytes: &[u8], index: usize) -> Option<(usize, usize)> {
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

fn rust_raw_string_end_at(bytes: &[u8], index: usize, hashes: usize) -> bool {
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

