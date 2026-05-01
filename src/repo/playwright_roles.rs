fn add_playwright_role_name_surfaces(
    surfaces: &mut SurfaceExtraction,
    pending: &mut BTreeMap<String, (String, String)>,
    line: &str,
) {
    let negative_assertion = line_has_negative_playwright_assertion(line);
    let reassigned_pending = pending
        .keys()
        .filter(|local| line_reassigns_pending_locator(line, local))
        .cloned()
        .collect::<Vec<_>>();
    for local in reassigned_pending {
        pending.remove(&local);
    }
    let mut resolved_pending = Vec::new();
    for (local, (role, name)) in pending.iter() {
        if line_has_expect_for_identifier(line, local) {
            if !negative_assertion {
                add_accessible_role_name_surface(surfaces, role, name);
            }
            resolved_pending.push(local.clone());
        }
    }
    for local in resolved_pending {
        pending.remove(&local);
    }
    if negative_assertion {
        return;
    }
    let role_names = playwright_role_name_calls(line);
    if role_names.is_empty() {
        return;
    }
    if let Some(local) = playwright_role_name_assignment(line) {
        if let Some((role, name)) = role_names.into_iter().next() {
            pending.insert(local, (role, name));
        }
        return;
    }
    let mut proven_role_names = BTreeSet::new();
    proven_role_names.extend(playwright_role_name_expect_calls(line));
    proven_role_names.extend(playwright_role_name_action_calls(line));
    for (role, name) in proven_role_names {
        add_accessible_role_name_surface(surfaces, &role, &name);
    }
}

fn playwright_role_name_calls(line: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for call in js_call_spans_with_end(line, "getByRole") {
        if let Some(role_name) = playwright_role_name_from_call(&call.source) {
            out.push(role_name);
        }
    }
    out
}

fn playwright_role_name_expect_calls(line: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for call in js_call_spans_with_end(line, "getByRole") {
        if !call_is_inside_playwright_expect(line, call.start) {
            continue;
        }
        if let Some(role_name) = playwright_role_name_from_call(&call.source) {
            out.push(role_name);
        }
    }
    out
}

fn playwright_role_name_action_calls(line: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for call in js_call_spans_with_end(line, "getByRole") {
        if !line_has_await_or_return_before(line, call.start) {
            continue;
        }
        if !line_tail_has_locator_action(line, call.end) {
            continue;
        }
        if let Some(role_name) = playwright_role_name_from_call(&call.source) {
            out.push(role_name);
        }
    }
    out
}

fn playwright_role_name_from_call(call: &str) -> Option<(String, String)> {
    let args = js_top_level_arguments(call);
    let role = args
        .first()
        .and_then(|arg| js_string_literal_value(arg))
        .and_then(|role| normalize_accessible_role(&role))?;
    let options = args.get(1)?;
    let name = js_plain_object_single_string_property_value(options, "name")?;
    Some((role, name))
}

fn line_tail_has_locator_action(line: &str, mut cursor: usize) -> bool {
    cursor = skip_js_whitespace(line, cursor);
    const ACTIONS: &[&str] = &[
        "click",
        "dblclick",
        "tap",
        "hover",
        "fill",
        "press",
        "check",
        "uncheck",
        "selectOption",
        "setInputFiles",
        "dragTo",
        "focus",
        "blur",
    ];
    for action in ACTIONS {
        let mut probe = cursor;
        if !line[probe..].starts_with('.') {
            continue;
        }
        probe = skip_js_whitespace(line, probe + 1);
        if !line[probe..].starts_with(action)
            || !js_identifier_boundary_after(line, probe + action.len())
        {
            continue;
        }
        probe = skip_js_whitespace(line, probe + action.len());
        if line[probe..].starts_with('(') {
            return true;
        }
    }
    false
}

fn call_is_inside_playwright_expect(line: &str, call_start: usize) -> bool {
    let mut search_start = 0usize;
    while let Some(relative) = line[search_start..call_start].find("expect") {
        let expect_start = search_start + relative;
        let expect_end = expect_start + "expect".len();
        search_start = expect_end;
        if js_byte_is_inside_string_or_regex_literal(line, expect_start)
            || !js_identifier_boundary_before(line, expect_start)
            || !js_identifier_boundary_after(line, expect_end)
            || !line_has_await_or_return_before(line, expect_start)
        {
            continue;
        }
        let mut cursor = skip_js_whitespace(line, expect_end);
        if line[cursor..].starts_with(".soft") {
            cursor = skip_js_whitespace(line, cursor + ".soft".len());
        }
        if !line[cursor..].starts_with('(') {
            continue;
        }
        let Some(expect_close) = js_balanced_call_end(line, cursor) else {
            continue;
        };
        if call_start > cursor
            && call_start < expect_close
            && line_tail_has_positive_expect_matcher(line, expect_close)
        {
            return true;
        }
    }
    false
}

fn playwright_role_name_assignment(line: &str) -> Option<String> {
    let call_start = line.find("getByRole")?;
    let before_call = &line[..call_start];
    let eq = before_call.rfind('=')?;
    simple_local_assignment_lhs(&before_call[..eq])
}

fn line_reassigns_pending_locator(line: &str, ident: &str) -> bool {
    if line.contains("getByRole") {
        return false;
    }
    let Some(eq) = first_js_assignment_operator(line) else {
        return false;
    };
    simple_local_assignment_lhs(&line[..eq])
        .map(|local| local == ident)
        .unwrap_or(false)
}

fn first_js_assignment_operator(line: &str) -> Option<usize> {
    let mut index = 0usize;
    while let Some(relative) = line[index..].find('=') {
        let byte = index + relative;
        index = byte + 1;
        if js_byte_is_inside_string_or_regex_literal(line, byte) {
            continue;
        }
        let before = line[..byte].chars().next_back();
        let after = line[byte + 1..].chars().next();
        if matches!(
            before,
            Some('=' | '!' | '<' | '>' | '+' | '-' | '*' | '/' | '%')
        ) || matches!(after, Some('=' | '>'))
        {
            continue;
        }
        return Some(byte);
    }
    None
}

fn simple_local_assignment_lhs(left: &str) -> Option<String> {
    let trimmed = left.trim();
    for keyword in ["const", "let", "var"] {
        if let Some(rest) = trimmed.strip_prefix(keyword)
            && rest
                .chars()
                .next()
                .map(|ch| ch.is_whitespace())
                .unwrap_or(false)
        {
            return simple_identifier_lhs(rest.trim());
        }
    }
    simple_identifier_lhs(trimmed)
}

fn simple_identifier_lhs(value: &str) -> Option<String> {
    let ident = first_identifier(value)?;
    (ident == value.trim()).then_some(ident)
}

fn line_has_expect_for_identifier(line: &str, ident: &str) -> bool {
    let mut search_start = 0usize;
    while let Some(relative) = line[search_start..].find("expect") {
        let expect_start = search_start + relative;
        let expect_end = expect_start + "expect".len();
        search_start = expect_end;
        if js_byte_is_inside_string_or_regex_literal(line, expect_start)
            || !js_identifier_boundary_before(line, expect_start)
            || !js_identifier_boundary_after(line, expect_end)
            || !line_has_await_or_return_before(line, expect_start)
        {
            continue;
        }
        let mut cursor = skip_js_whitespace(line, expect_end);
        if line[cursor..].starts_with(".soft") {
            cursor = skip_js_whitespace(line, cursor + ".soft".len());
        }
        if !line[cursor..].starts_with('(') {
            continue;
        }
        let Some(expect_close) = js_balanced_call_end(line, cursor) else {
            continue;
        };
        let args = js_split_top_level_commas(&line[cursor + 1..expect_close - 1]);
        if args.first().map(|arg| arg.trim() == ident).unwrap_or(false)
            && line_tail_has_positive_expect_matcher(line, expect_close)
        {
            return true;
        }
    }
    false
}

fn line_tail_has_positive_expect_matcher(line: &str, mut cursor: usize) -> bool {
    cursor = skip_js_whitespace(line, cursor);
    if !line[cursor..].starts_with('.') {
        return false;
    }
    cursor = skip_js_whitespace(line, cursor + 1);
    if line[cursor..].starts_with("not") && js_identifier_boundary_after(line, cursor + "not".len())
    {
        return false;
    }
    const MATCHERS: &[&str] = &[
        "toBeVisible",
        "toBeEnabled",
        "toBeDisabled",
        "toBeChecked",
        "toBeFocused",
        "toBeInViewport",
        "toHaveText",
        "toContainText",
        "toHaveAttribute",
        "toHaveCount",
        "toHaveValue",
        "toHaveURL",
        "toHaveTitle",
        "toHaveClass",
        "toHaveCSS",
    ];
    MATCHERS.iter().any(|matcher| {
        line[cursor..].starts_with(matcher)
            && js_identifier_boundary_after(line, cursor + matcher.len())
            && line[skip_js_whitespace(line, cursor + matcher.len())..].starts_with('(')
    })
}

fn line_has_await_or_return_before(line: &str, byte: usize) -> bool {
    let start = current_js_statement_start(line, byte);
    let prefix = line[start..byte].trim_start();
    js_statement_prefix_starts_with_keyword(prefix, "await")
        || js_statement_prefix_starts_with_keyword(prefix, "return")
}

fn current_js_statement_start(line: &str, byte: usize) -> usize {
    let mut start = 0usize;
    let mut index = 0usize;
    while index < byte {
        let Some(ch) = line[index..].chars().next() else {
            break;
        };
        let next = index + ch.len_utf8();
        if matches!(ch, ';' | ',' | '{' | '}')
            && !js_byte_is_inside_string_or_regex_literal(line, index)
        {
            start = next;
        }
        index = next;
    }
    start
}

fn js_statement_prefix_starts_with_keyword(prefix: &str, keyword: &str) -> bool {
    prefix
        .strip_prefix(keyword)
        .map(|rest| {
            rest.chars()
                .next()
                .map(|ch| ch.is_whitespace())
                .unwrap_or(false)
                && !js_prefix_has_top_level_runtime_split(rest)
        })
        .unwrap_or(false)
}

fn js_prefix_has_top_level_runtime_split(text: &str) -> bool {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut index = 0usize;
    let mut quote = None;
    let mut escaped = false;
    while index < chars.len() {
        let (byte, ch) = chars[index];
        if let Some(active_quote) = quote {
            index += 1;
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
        if matches!(ch, '"' | '\'' | '`') {
            quote = Some(ch);
            escaped = false;
            index += 1;
            continue;
        }
        if text[byte..].starts_with("&&")
            || text[byte..].starts_with("||")
            || text[byte..].starts_with("??")
            || matches!(ch, '?' | ':')
        {
            return true;
        }
        index += 1;
    }
    false
}

fn line_has_negative_playwright_assertion(line: &str) -> bool {
    let compact = line
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    compact.contains(".not.")
        || compact.contains("tohavecount(0")
        || compact.contains("tobehidden(")
}

