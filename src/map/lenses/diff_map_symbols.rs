fn changed_symbols_from_delta(
    rel: &str,
    file: &FileInfo,
    delta: &LineDelta,
    added_code: &BTreeMap<usize, String>,
    base_code: &BTreeMap<usize, String>,
) -> Vec<ChangedSymbol> {
    // Removed lines use old-file coordinates; matching them to current symbols
    // creates false body-change claims when code above a symbol is deleted.
    let changed_code_lines = delta
        .added
        .iter()
        .filter_map(|(line, _)| structural_changed_line(*line, added_code, base_code))
        .collect::<BTreeSet<_>>();
    file.symbols
        .iter()
        .filter(|symbol| symbol.exported)
        .filter(|symbol| {
            changed_code_lines
                .iter()
                .any(|line| *line >= symbol.line_start && *line <= symbol.line_end)
        })
        .map(|symbol| ChangedSymbol {
            path: rel.to_string(),
            name: symbol.name.clone(),
            change: "symbol_body_changed".to_string(),
            line_start: Some(symbol.line_start),
            line_end: Some(symbol.line_end),
        })
        .collect()
}

fn structural_changed_line(
    line: usize,
    changed_lookup: &BTreeMap<usize, String>,
    counterpart_lookup: &BTreeMap<usize, String>,
) -> Option<usize> {
    let changed = normalized_code_shape(changed_lookup.get(&line)?);
    if changed.is_empty() {
        return None;
    }
    let counterpart = counterpart_lookup
        .get(&line)
        .map(|code| normalized_code_shape(code))
        .unwrap_or_default();
    (changed != counterpart).then_some(line)
}

fn normalized_code_shape(code: &str) -> String {
    let code = strip_line_comment_outside_literals(code);
    code.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_line_comment_outside_literals(code: &str) -> &str {
    let mut quote = None;
    let mut escaped = false;
    let mut chars = code.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' && active_quote != '`' {
                escaped = true;
            } else if ch == active_quote {
                quote = None;
            }
            continue;
        }
        if matches!(ch, '"' | '\'' | '`') {
            quote = Some(ch);
            escaped = false;
            continue;
        }
        if ch == '/' && chars.peek().is_some_and(|(_, next)| *next == '/') {
            return &code[..index];
        }
    }
    code
}
