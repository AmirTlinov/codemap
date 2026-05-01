#[derive(Debug, Clone)]
struct JsStaticImport {
    spec: String,
    clause: Option<String>,
    is_type: bool,
}

fn extract_js_import_specs(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for import in extract_js_static_imports(text) {
        out.insert(import.spec);
    }
    out.extend(extract_js_export_from_specs(text));
    out.extend(extract_js_call_import_specs(text));
    out
}

fn extract_js_import_bindings(text: &str) -> ImportBindingsBySpec {
    let mut out = BTreeMap::new();
    for import in extract_js_static_imports(text) {
        if import.is_type {
            continue;
        }
        let Some(clause) = import.clause.as_deref() else {
            continue;
        };
        let bindings = parse_js_import_clause_bindings(clause);
        if !bindings.is_empty() {
            out.entry(import.spec)
                .or_insert_with(BTreeMap::new)
                .extend(bindings);
        }
    }
    for (spec, bindings) in extract_js_reexport_bindings(text) {
        out.entry(spec)
            .or_insert_with(BTreeMap::new)
            .extend(bindings);
    }
    out
}

fn extract_js_reexport_bindings(text: &str) -> ImportBindingsBySpec {
    let mut out = BTreeMap::new();
    for start in js_keyword_positions(text, "export") {
        let statement = js_statement_slice(text, start);
        let statement_without_comments = strip_js_comments_from_text(statement);
        let Some(cap) = js_reexport_statement_re().captures(&statement_without_comments) else {
            continue;
        };
        if cap.name("type").is_some() {
            continue;
        }
        let Some(spec) = cap.name("spec").map(|m| m.as_str().trim().to_string()) else {
            continue;
        };
        let mut bindings = BTreeMap::new();
        if cap.name("star").is_some() {
            bindings.insert("export:*".to_string(), "*".to_string());
        } else if let Some(named) = cap.name("named") {
            let clause = format!("{{{}}}", named.as_str());
            let mut named_bindings = BTreeMap::new();
            collect_js_named_import_bindings(&clause, &mut named_bindings);
            for (exported, imported) in named_bindings {
                bindings.insert(format!("export:{exported}"), imported);
            }
        }
        if !bindings.is_empty() {
            out.entry(spec)
                .or_insert_with(BTreeMap::new)
                .extend(bindings);
        }
    }
    out
}

fn extract_js_static_imports(text: &str) -> Vec<JsStaticImport> {
    js_keyword_positions(text, "import")
        .into_iter()
        .filter_map(|start| {
            let after = skip_ascii_whitespace(text, start + "import".len());
            let next = text.as_bytes().get(after).copied();
            if matches!(next, Some(b'.' | b'(')) {
                return None;
            }
            parse_js_static_import_statement(js_statement_slice(text, start))
        })
        .collect()
}

fn parse_js_static_import_statement(statement: &str) -> Option<JsStaticImport> {
    let cap = js_static_import_statement_re().captures(statement)?;
    let spec = cap.name("spec")?.as_str().trim().to_string();
    let clause = cap.name("clause").map(|m| m.as_str().trim().to_string());
    Some(JsStaticImport {
        spec,
        clause,
        is_type: cap.name("type").is_some(),
    })
}

fn extract_js_export_from_specs(text: &str) -> BTreeSet<String> {
    js_keyword_positions(text, "export")
        .into_iter()
        .filter_map(|start| {
            js_export_from_re()
                .captures(js_statement_slice(text, start))
                .and_then(|cap| cap.name("spec").map(|m| m.as_str().trim().to_string()))
        })
        .collect()
}

fn extract_js_call_import_specs(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for start in js_keyword_positions(text, "import") {
        let after = skip_ascii_whitespace(text, start + "import".len());
        if text.as_bytes().get(after) == Some(&b'(')
            && let Some(spec) = parse_js_call_string_arg(text, after)
        {
            out.insert(spec);
        }
    }
    for start in js_keyword_positions(text, "require") {
        let after = skip_ascii_whitespace(text, start + "require".len());
        if text.as_bytes().get(after) == Some(&b'(')
            && let Some(spec) = parse_js_call_string_arg(text, after)
        {
            out.insert(spec);
        }
    }
    out
}

fn parse_js_call_string_arg(text: &str, open_paren: usize) -> Option<String> {
    let quote = skip_ascii_whitespace(text, open_paren + 1);
    let quote_byte = *text.as_bytes().get(quote)?;
    if !matches!(quote_byte, b'\'' | b'"') {
        return None;
    }
    read_js_quoted_string(text, quote)
}

fn read_js_quoted_string(text: &str, quote: usize) -> Option<String> {
    let bytes = text.as_bytes();
    let quote_byte = *bytes.get(quote)?;
    if !matches!(quote_byte, b'\'' | b'"') {
        return None;
    }
    let mut index = quote + 1;
    let mut out = String::new();
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'\\' {
            index = index.saturating_add(2);
            continue;
        }
        if byte == quote_byte {
            return Some(out);
        }
        out.push(byte as char);
        index += 1;
    }
    None
}

fn js_statement_slice(text: &str, start: usize) -> &str {
    let bytes = text.as_bytes();
    let mut index = start;
    let mut state = JsScanState::Code;
    while index < bytes.len() {
        match state {
            JsScanState::Code => {
                if bytes[index] == b';' {
                    return &text[start..=index];
                }
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
                        std::string::String::from_utf8_lossy(&bytes[start..index]).as_ref(),
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
    &text[start..]
}
