fn extract_symbols(text: &str, ext: &str) -> Vec<SymbolInfo> {
    if ext == "swift" {
        let cleaned = code_without_comments_or_strings(text, ext);
        return symbols_with_ranges(extract_swift_symbols(&cleaned), &cleaned, ext);
    }
    if matches!(
        ext,
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte"
    ) {
        let cleaned = code_without_comments_or_strings(text, ext);
        return extract_js_symbols_from_cleaned(text, &cleaned, ext);
    }
    let starts = match ext {
        "rs" => extract_rust_symbols(text),
        "py" => extract_python_symbols(text),
        "go" => extract_go_symbols(text),
        _ => Vec::new(),
    };
    symbols_with_ranges(starts, text, ext)
}

fn extract_js_symbols_from_cleaned(text: &str, cleaned: &str, ext: &str) -> Vec<SymbolInfo> {
    symbols_with_ranges(extract_js_symbols(cleaned), text, ext)
}

fn extract_identifier_references(text: &str, ext: &str) -> BTreeSet<String> {
    if !is_source_ext(ext) {
        return BTreeSet::new();
    }
    let cleaned = code_without_comments_or_strings(text, ext);
    extract_identifier_references_from_cleaned(&cleaned)
}

fn extract_identifier_references_from_cleaned(cleaned: &str) -> BTreeSet<String> {
    identifier_re()
        .find_iter(cleaned)
        .filter(|m| !identifier_is_selector_tail(cleaned, m.start()))
        .map(|m| m.as_str())
        .filter(|name| !language_keyword(name))
        .map(str::to_string)
        .collect()
}

fn extract_jsx_tags(text: &str, ext: &str) -> BTreeSet<String> {
    if !matches!(ext, "tsx" | "jsx" | "vue" | "svelte") {
        return BTreeSet::new();
    }
    let cleaned = code_without_comments_or_strings(text, ext);
    extract_jsx_tags_from_cleaned(&cleaned, ext)
}

fn extract_jsx_tags_from_cleaned(cleaned: &str, ext: &str) -> BTreeSet<String> {
    if !matches!(ext, "tsx" | "jsx" | "vue" | "svelte") {
        return BTreeSet::new();
    }
    let mut out = BTreeSet::new();
    let mut type_brace_depth: Option<usize> = None;
    for line in cleaned.lines() {
        let trimmed = line.trim_start();
        if let Some(depth) = type_brace_depth.as_mut() {
            *depth = js_type_brace_depth_after_line(*depth, line);
            if js_type_context_line_is_complete(trimmed, *depth) {
                type_brace_depth = None;
            }
            continue;
        }
        if js_type_context_line_starts(trimmed) {
            let depth = js_type_brace_depth_after_line(0, line);
            if !js_type_context_line_is_complete(trimmed, depth) {
                type_brace_depth = Some(depth);
            }
            continue;
        }
        let line = js_line_without_regex_literals(line);
        out.extend(
            jsx_tag_re()
                .captures_iter(&line)
                .filter(|cap| {
                    cap.get(0)
                        .zip(cap.get(1))
                        .map(|(tag, name)| jsx_tag_context_is_value(&line, tag.start(), name.end()))
                        .unwrap_or(false)
                })
                .filter_map(|cap| cap.get(1))
                .map(|m| m.as_str().to_string()),
        );
    }
    out
}

fn jsx_tag_context_is_value(text: &str, tag_start: usize, name_end: usize) -> bool {
    let before = tag_start
        .checked_sub(1)
        .and_then(|index| text.as_bytes().get(index))
        .copied();
    if before
        .map(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$' | b'.'))
        .unwrap_or(false)
    {
        return false;
    }
    let after = text[name_end..]
        .bytes()
        .find(|byte| !byte.is_ascii_whitespace());
    if matches!(after, Some(b'|' | b'&' | b',' | b'=')) {
        return false;
    }
    let after = text[name_end..].trim_start();
    !(after.starts_with("extends ")
        || after.starts_with("extends\t")
        || after.starts_with("extends\n")
        || after.starts_with(">()")
        || after.starts_with("> ()"))
}

fn js_type_context_line_starts(trimmed: &str) -> bool {
    trimmed.starts_with("type ")
        || trimmed.starts_with("export type ")
        || trimmed.starts_with("interface ")
        || trimmed.starts_with("export interface ")
}

fn js_type_context_line_is_complete(trimmed: &str, brace_depth: usize) -> bool {
    brace_depth == 0 && (trimmed.contains(';') || trimmed.ends_with('}') || trimmed.ends_with("};"))
}

fn js_type_brace_depth_after_line(mut depth: usize, code: &str) -> usize {
    for byte in code.bytes() {
        match byte {
            b'{' => depth = depth.saturating_add(1),
            b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth
}

fn js_line_without_regex_literals(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut out = String::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'/'
            && js_regex_literal_can_start(&out)
            && let Some(end) = js_regex_literal_end(bytes, index)
        {
            index = end;
            continue;
        }
        out.push(bytes[index] as char);
        index += 1;
    }
    out
}

fn js_regex_literal_can_start(prefix: &str) -> bool {
    [
        "return", "await", "throw", "yield", "case", "delete", "void", "typeof", "else",
    ]
    .iter()
    .any(|word| previous_word_is(prefix, word))
        || prefix.trim_end().ends_with("=>")
        || previous_nonspace_byte(prefix)
            .map(|byte| {
                matches!(
                    byte,
                    b'(' | b')'
                        | b'='
                        | b':'
                        | b'['
                        | b'{'
                        | b','
                        | b'!'
                        | b'?'
                        | b';'
                        | b'|'
                        | b'&'
                )
            })
            .unwrap_or(true)
}

fn previous_word_is(before: &str, word: &str) -> bool {
    let bytes = before.as_bytes();
    let mut end = bytes.len();
    while end > 0 && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    if end == 0 {
        return false;
    }
    let mut start = end;
    while start > 0 && is_identifier_byte(bytes[start - 1]) {
        start -= 1;
    }
    std::str::from_utf8(&bytes[start..end]) == Ok(word)
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$')
}

fn previous_nonspace_byte(before: &str) -> Option<u8> {
    before
        .bytes()
        .rev()
        .find(|byte| !byte.is_ascii_whitespace())
}

fn js_regex_literal_end(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'/') || matches!(bytes.get(start + 1), Some(b'/' | b'*') | None) {
        return None;
    }
    let mut index = start + 1;
    let mut in_class = false;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => {
                index = index.saturating_add(2);
                continue;
            }
            b'[' => in_class = true,
            b']' => in_class = false,
            b'/' if !in_class => {
                index += 1;
                while bytes
                    .get(index)
                    .map(|byte| byte.is_ascii_alphabetic())
                    .unwrap_or(false)
                {
                    index += 1;
                }
                return Some(index);
            }
            _ => {}
        }
        index += 1;
    }
    None
}

pub(crate) fn extract_local_bindings(text: &str, ext: &str) -> BTreeSet<String> {
    if !matches!(
        ext,
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte"
    ) {
        return BTreeSet::new();
    }
    let cleaned = code_without_comments_or_strings(text, ext);
    extract_local_bindings_from_cleaned(&cleaned, ext)
}

fn extract_local_bindings_from_cleaned(cleaned: &str, ext: &str) -> BTreeSet<String> {
    if !matches!(
        ext,
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte"
    ) {
        return BTreeSet::new();
    }
    let mut out = BTreeSet::new();
    collect_js_balanced_param_bindings(cleaned, &mut out);
    for cap in js_single_arrow_param_re().captures_iter(cleaned) {
        if let Some(param) = cap.name("param") {
            let name = param.as_str();
            if !language_keyword(name) {
                out.insert(name.to_string());
            }
        }
    }
    for cap in js_for_binding_re().captures_iter(cleaned) {
        if let Some(binding) = cap.name("binding") {
            collect_js_param_bindings(binding.as_str(), &mut out);
        }
    }
    for pattern in js_destructuring_binding_patterns(cleaned) {
        collect_js_param_bindings(pattern, &mut out);
    }
    out
}
