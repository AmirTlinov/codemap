struct JsCallSpan {
    source: String,
    start: usize,
    end: usize,
}

fn js_call_spans_with_end(text: &str, callee: &str) -> Vec<JsCallSpan> {
    let mut spans = Vec::new();
    let mut search_start = 0usize;
    while let Some(relative) = text[search_start..].find(callee) {
        let callee_start = search_start + relative;
        let callee_end = callee_start + callee.len();
        if js_byte_is_inside_string_or_regex_literal(text, callee_start) {
            search_start = callee_end;
            continue;
        }
        if !js_callee_has_playwright_receiver(text, callee_start) {
            search_start = callee_end;
            continue;
        }
        if text[..callee_start]
            .chars()
            .next_back()
            .map(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$')
            .unwrap_or(false)
            || text[callee_end..]
                .chars()
                .next()
                .map(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$')
                .unwrap_or(false)
        {
            search_start = callee_end;
            continue;
        }
        let mut cursor = callee_end;
        while cursor < text.len() {
            let Some(ch) = text[cursor..].chars().next() else {
                break;
            };
            if ch.is_whitespace() {
                cursor += ch.len_utf8();
            } else {
                break;
            }
        }
        if !text[cursor..].starts_with('(') {
            search_start = callee_end;
            continue;
        }
        let Some(end) = js_balanced_call_end(text, cursor) else {
            search_start = callee_end;
            continue;
        };
        spans.push(JsCallSpan {
            source: text[callee_start..end].to_string(),
            start: callee_start,
            end,
        });
        search_start = end;
    }
    spans
}

fn js_callee_has_playwright_receiver(text: &str, callee_start: usize) -> bool {
    let before = text[..callee_start].trim_end();
    let Some(receiver) = before.strip_suffix('.') else {
        return false;
    };
    let receiver = receiver.trim_end();
    js_receiver_ends_with_standalone_page(receiver)
        || js_receiver_contains_page_method_call(receiver, "locator")
        || js_receiver_contains_page_method_call(receiver, "frameLocator")
}

fn js_receiver_ends_with_standalone_page(receiver: &str) -> bool {
    let Some(start) = receiver.rfind("page") else {
        return false;
    };
    start + "page".len() == receiver.len()
        && js_identifier_boundary_before(receiver, start)
        && receiver[..start]
            .chars()
            .next_back()
            .map(|ch| ch != '.')
            .unwrap_or(true)
}

fn js_receiver_contains_page_method_call(receiver: &str, method: &str) -> bool {
    let needle = format!("page.{method}");
    let mut search_start = 0usize;
    while let Some(relative) = receiver[search_start..].find(&needle) {
        let page_start = search_start + relative;
        if js_identifier_boundary_before(receiver, page_start)
            && receiver[..page_start]
                .chars()
                .next_back()
                .map(|ch| ch != '.')
                .unwrap_or(true)
        {
            let method_end = page_start + needle.len();
            let cursor = skip_js_whitespace(receiver, method_end);
            if receiver[cursor..].starts_with('(')
                && let Some(call_end) = js_balanced_call_end(receiver, cursor)
                && skip_js_whitespace(receiver, call_end) == receiver.len()
            {
                return true;
            }
        }
        search_start = page_start + needle.len();
    }
    false
}

fn js_byte_is_inside_string_or_regex_literal(text: &str, byte: usize) -> bool {
    let bytes = text.as_bytes();
    let mut index = 0usize;
    let mut prefix = String::new();
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    while index < bytes.len() && index < byte {
        let byte_value = bytes[index];
        if let Some(active_quote) = quote {
            index += 1;
            if escaped {
                prefix.push(' ');
                escaped = false;
                continue;
            }
            if byte_value == b'\\' {
                escaped = true;
                prefix.push(' ');
                continue;
            }
            if byte_value == active_quote {
                quote = None;
            }
            prefix.push(' ');
            continue;
        }
        if matches!(byte_value, b'"' | b'\'' | b'`') {
            quote = Some(byte_value);
            escaped = false;
            prefix.push(' ');
            index += 1;
            continue;
        }
        if byte_value == b'/'
            && js_regex_literal_can_start(&prefix)
            && let Some(end) = js_regex_literal_end(bytes, index)
        {
            if byte < end {
                return true;
            }
            prefix.push(' ');
            index = end;
            continue;
        }
        prefix.push(byte_value as char);
        index += 1;
    }
    quote.is_some()
}

fn js_balanced_call_end(text: &str, open_paren: usize) -> Option<usize> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut index = chars.iter().position(|(byte, _)| *byte == open_paren)?;
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    while index < chars.len() {
        let (byte, ch) = chars[index];
        index += 1;
        if let Some(active_quote) = quote {
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
            continue;
        }
        match ch {
            '(' => paren_depth += 1,
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 {
                    return Some(byte + ch.len_utf8());
                }
            }
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            _ => {}
        }
    }
    None
}

fn js_top_level_arguments(call: &str) -> Vec<String> {
    let Some(open) = call.find('(') else {
        return Vec::new();
    };
    let Some(close) = js_balanced_call_end(call, open) else {
        return Vec::new();
    };
    if close <= open + 1 {
        return Vec::new();
    }
    js_split_top_level_commas(&call[open + 1..close - 1])
}

fn js_split_top_level_commas(value: &str) -> Vec<String> {
    let chars: Vec<(usize, char)> = value.char_indices().collect();
    let mut args = Vec::new();
    let mut start = 0usize;
    let mut index = 0usize;
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    while index < chars.len() {
        let (byte, ch) = chars[index];
        index += 1;
        if let Some(active_quote) = quote {
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
            continue;
        }
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            ',' if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 => {
                args.push(value[start..byte].trim().to_string());
                start = byte + ch.len_utf8();
            }
            _ => {}
        }
    }
    let tail = value[start..].trim();
    if !tail.is_empty() {
        args.push(tail.to_string());
    }
    args
}

