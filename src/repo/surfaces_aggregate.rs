fn merge_surface_extraction(target: &mut SurfaceExtraction, source: SurfaceExtraction) {
    target.tokens.extend(source.tokens);
    target.phrases.extend(source.phrases);
    target.visited_routes.extend(source.visited_routes);
}

fn playwright_test_bindings(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for import in extract_js_static_imports(text) {
        if import.is_type || import.spec != "@playwright/test" {
            continue;
        }
        let Some(clause) = import.clause.as_deref() else {
            continue;
        };
        for (local, imported) in parse_js_import_clause_bindings(clause) {
            if imported == "test" {
                out.insert(local);
            }
        }
    }
    out
}

fn line_declares_playwright_page_fixture(
    line: &str,
    playwright_test_bindings: &BTreeSet<String>,
) -> bool {
    if playwright_test_bindings.is_empty() || !line.contains("=>") {
        return false;
    }
    let before_arrow = line.split("=>").next().unwrap_or(line);
    if !line_has_playwright_test_call(before_arrow, playwright_test_bindings) {
        return false;
    }
    let Some((start, end)) = js_last_balanced_object_span(before_arrow) else {
        return false;
    };
    js_split_top_level_commas(&before_arrow[start + 1..end])
        .iter()
        .any(|part| js_destructure_part_is_direct_shorthand_prop(part, "page"))
}

fn line_declares_disabled_playwright_describe(
    line: &str,
    playwright_test_bindings: &BTreeSet<String>,
) -> bool {
    playwright_test_bindings.iter().any(|binding| {
        line_has_playwright_test_method_call(line, binding, &["describe.skip", "describe.fixme"])
    })
}

fn line_starts_playwright_describe_callback_body(line: &str) -> bool {
    line_starts_arrow_callback_body(line) || line_starts_function_callback_body(line)
}

fn line_starts_nested_playwright_body(line: &str) -> bool {
    line_starts_function_callback_body(line) || line_starts_method_callback_body(line)
}

fn line_has_arrow_callback(line: &str) -> bool {
    let mut search_start = 0usize;
    while let Some(relative) = line[search_start..].find("=>") {
        let arrow = search_start + relative;
        if !js_byte_is_inside_string_or_regex_literal(line, arrow) {
            return true;
        }
        search_start = arrow + 2;
    }
    false
}

fn line_starts_arrow_callback_body(line: &str) -> bool {
    let mut search_start = 0usize;
    while let Some(relative) = line[search_start..].find("=>") {
        let arrow = search_start + relative;
        if js_byte_is_inside_string_or_regex_literal(line, arrow) {
            search_start = arrow + 2;
            continue;
        }
        let cursor = skip_js_whitespace(line, arrow + 2);
        if line[cursor..].starts_with('{') {
            return true;
        }
        search_start = arrow + 2;
    }
    false
}

fn line_ends_js_statement(line: &str) -> bool {
    line.trim_end().ends_with(';')
}

fn line_starts_method_callback_body(line: &str) -> bool {
    let trimmed = line.trim_start();
    let Some(open) = trimmed.find('(') else {
        return false;
    };
    if js_byte_is_inside_string_or_regex_literal(trimmed, open) {
        return false;
    }
    let before_open = trimmed[..open].trim_end();
    if before_open.contains(['.', '=']) {
        return false;
    }
    let Some(name) = before_open
        .rsplit(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'))
        .find(|part| !part.is_empty())
    else {
        return false;
    };
    if matches!(
        name,
        "if" | "for" | "while" | "switch" | "catch" | "function" | "test" | "expect" | "page"
    ) {
        return false;
    }
    let Some(close) = js_balanced_call_end(trimmed, open) else {
        return false;
    };
    let cursor = skip_js_whitespace(trimmed, close);
    trimmed[cursor..].starts_with('{')
}

fn line_declares_pending_nested_body(line: &str) -> bool {
    line_declares_pending_function_body(line) || line_declares_pending_method_body(line)
}

fn line_opens_pending_nested_body(line: &str) -> bool {
    line.trim_start().starts_with('{')
}

fn line_opens_pending_control_flow_body(line: &str) -> bool {
    line.trim_start().starts_with('{')
}

fn line_opens_control_flow_body(line: &str) -> bool {
    line.char_indices()
        .any(|(byte, ch)| ch == '{' && !js_byte_is_inside_string_or_regex_literal(line, byte))
}

fn line_starts_unparsed_playwright_control_flow(line: &str) -> bool {
    let mut trimmed = line.trim_start();
    while let Some(rest) = trimmed.strip_prefix('}') {
        trimmed = rest.trim_start();
    }
    for keyword in [
        "if", "else", "for", "while", "switch", "try", "catch", "finally", "do",
    ] {
        if let Some(rest) = trimmed.strip_prefix(keyword)
            && rest
                .chars()
                .next()
                .map(|ch| ch.is_whitespace() || matches!(ch, '(' | '{'))
                .unwrap_or(true)
        {
            return true;
        }
    }
    false
}

fn line_has_playwright_scope_terminator_before_role_name_call(
    line: &str,
    playwright_test_bindings: &BTreeSet<String>,
) -> bool {
    let Some(call_start) = line.find("getByRole") else {
        return false;
    };
    let prefix = &line[..call_start];
    if line_declares_runtime_playwright_skip(prefix, playwright_test_bindings) {
        return true;
    }
    let Some(last_semicolon) = prefix.rfind(';') else {
        return false;
    };
    prefix[..last_semicolon]
        .split(';')
        .any(js_segment_is_scope_terminator)
}

fn line_terminates_playwright_page_scope(
    line: &str,
    playwright_test_bindings: &BTreeSet<String>,
) -> bool {
    line_declares_runtime_playwright_skip(line, playwright_test_bindings)
        || line.split(';').any(js_segment_is_scope_terminator)
}

fn line_declares_runtime_playwright_skip(
    line: &str,
    playwright_test_bindings: &BTreeSet<String>,
) -> bool {
    playwright_test_bindings.iter().any(|binding| {
        line_has_playwright_test_method_call(line, binding, &["skip", "fixme"])
            && !line_has_playwright_test_method_call(
                line,
                binding,
                &["describe.skip", "describe.fixme"],
            )
    })
}

fn js_segment_is_scope_terminator(segment: &str) -> bool {
    let trimmed = segment.trim_start();
    js_segment_starts_with_keyword(trimmed, "return")
        || js_segment_starts_with_keyword(trimmed, "throw")
}

fn js_segment_starts_with_keyword(segment: &str, keyword: &str) -> bool {
    segment
        .strip_prefix(keyword)
        .map(|rest| {
            rest.chars()
                .next()
                .map(|ch| ch.is_whitespace() || ch == ';')
                .unwrap_or(true)
        })
        .unwrap_or(false)
}

