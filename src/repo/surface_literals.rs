fn quoted_prefix_is_page_goto_argument(prefix: &str) -> bool {
    let lower = prefix.to_ascii_lowercase();
    let Some(index) = lower.rfind("page.goto") else {
        return false;
    };
    if lower[..index]
        .chars()
        .next_back()
        .map(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$')
        .unwrap_or(false)
    {
        return false;
    }
    let tail = lower[index + "page.goto".len()..].trim_start();
    let Some(argument_prefix) = tail.strip_prefix('(') else {
        return false;
    };
    !argument_prefix.contains(')') && argument_prefix.trim().is_empty()
}

fn normalize_route_path(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if !trimmed.starts_with('/') || trimmed.starts_with("//") || trimmed.contains("${") {
        return None;
    }
    let path = trimmed
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .trim_end_matches('/');
    let path = if path.is_empty() { "/" } else { path };
    if path
        .chars()
        .any(|ch| ch.is_whitespace() || matches!(ch, '"' | '\'' | '`'))
    {
        return None;
    }
    Some(path.to_string())
}

fn line_has_surface_context(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    [
        "classname",
        "class=",
        "contentclassname",
        "data-testid",
        "data-test",
        "aria-",
        "locator(",
        "getbytestid",
        "getbyrole",
        "getbylabel",
        "getbytext",
        "queryselector",
        "tocontaintext",
        "tohavetext",
        "getattribute(",
        "setattribute(",
        "page.goto",
        "tohaveurl",
        "href=",
        "mode=",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn strip_js_comments_from_line(line: &str, in_block_comment: &mut bool) -> String {
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let mut out = String::new();
    let mut index = 0;
    let mut quote = None;
    let mut escaped = false;
    while index < chars.len() {
        let (byte, ch) = chars[index];
        let next = chars.get(index + 1).map(|(_, next)| *next);
        if *in_block_comment {
            if ch == '*' && next == Some('/') {
                *in_block_comment = false;
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }
        if let Some(active_quote) = quote {
            out.push(ch);
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
        if ch == '/'
            && js_regex_literal_can_start(&out)
            && let Some(end) = js_regex_literal_end(line.as_bytes(), byte)
        {
            out.push_str(&line[byte..end]);
            while chars
                .get(index)
                .map(|(next_byte, _)| *next_byte < end)
                .unwrap_or(false)
            {
                index += 1;
            }
            continue;
        }
        if ch == '/' && next == Some('/') {
            break;
        }
        if ch == '/' && next == Some('*') {
            *in_block_comment = true;
            index += 2;
            continue;
        }
        if matches!(ch, '"' | '\'' | '`') {
            quote = Some(ch);
            escaped = false;
        }
        out.push(ch);
        index += 1;
    }
    out
}

fn line_accepts_plain_label_surface(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    [
        "aria-label",
        "getbylabel",
        "getbyrole",
        "getbytext",
        "tocontaintext",
        "tohavetext",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn line_has_jsx_surface_container(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    line.contains('<')
        && [
            "classname",
            "class=",
            "data-testid",
            "data-test",
            "aria-",
            "role=",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
}

fn static_jsx_visible_text(line: &str) -> Option<String> {
    let mut out = String::new();
    let mut in_tag = false;
    let mut brace_depth = 0usize;
    for ch in line.chars() {
        if in_tag {
            if ch == '>' {
                in_tag = false;
                out.push(' ');
            }
            continue;
        }
        if brace_depth > 0 {
            if ch == '{' {
                brace_depth += 1;
            } else if ch == '}' {
                brace_depth = brace_depth.saturating_sub(1);
                if brace_depth == 0 {
                    out.push(' ');
                }
            }
            continue;
        }
        if ch == '<' {
            in_tag = true;
            out.push(' ');
            continue;
        }
        if ch == '{' {
            brace_depth = 1;
            out.push(' ');
            continue;
        }
        out.push(ch);
    }
    let text = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.len() < 3
        || text.len() > 180
        || !text.chars().any(|ch| ch.is_alphabetic())
        || text.contains('=')
    {
        return None;
    }
    Some(text)
}

fn quoted_value_is_module_specifier_context(prefix: &str) -> bool {
    let lower = strip_trailing_js_comments(&prefix.to_ascii_lowercase());
    let trimmed = lower.trim_end();
    if token_ends_with(trimmed, "from") || token_ends_with(trimmed, "import") {
        return true;
    }
    if let Some(before_call) = trimmed.strip_suffix('(') {
        let before_call = before_call.trim_end();
        return token_ends_with(before_call, "import") || token_ends_with(before_call, "require");
    }
    token_ends_with(trimmed, "require")
}

fn strip_trailing_js_comments(value: &str) -> String {
    let mut out = value.trim_end().to_string();
    loop {
        let trimmed = out.trim_end();
        if !trimmed.ends_with("*/") {
            return trimmed.to_string();
        }
        let Some(start) = trimmed.rfind("/*") else {
            return trimmed.to_string();
        };
        out.truncate(start);
    }
}

fn token_ends_with(value: &str, token: &str) -> bool {
    let Some(before) = value.strip_suffix(token) else {
        return false;
    };
    before
        .chars()
        .next_back()
        .map(|ch| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'))
        .unwrap_or(true)
}

#[derive(Debug)]
struct QuotedString {
    value: String,
    prefix: String,
}

fn quoted_strings(text: &str) -> Vec<QuotedString> {
    let mut values = Vec::new();
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut index = 0;
    let mut in_block_comment = false;
    while index < chars.len() {
        let (start, ch) = chars[index];
        let next = chars.get(index + 1).map(|(_, next)| *next);
        if in_block_comment {
            if ch == '*' && next == Some('/') {
                in_block_comment = false;
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }
        if ch == '/' && next == Some('/') {
            break;
        }
        if ch == '/' && next == Some('*') {
            in_block_comment = true;
            index += 2;
            continue;
        }
        if !matches!(ch, '"' | '\'' | '`') {
            index += 1;
            continue;
        }
        let quote = ch;
        let mut value = String::new();
        let mut escaped = false;
        index += 1;
        while index < chars.len() {
            let (_, inner) = chars[index];
            index += 1;
            if escaped {
                value.push(inner);
                escaped = false;
                continue;
            }
            if inner == '\\' {
                escaped = true;
                continue;
            }
            if inner == quote {
                break;
            }
            value.push(inner);
        }
        values.push(QuotedString {
            value,
            prefix: text[..start].to_string(),
        });
    }
    values
}

fn surface_literal_is_structural(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.len() < 3 || trimmed.len() > 160 {
        return false;
    }
    if surface_literal_is_module_specifier(trimmed) {
        return false;
    }
    trimmed.starts_with('.')
        || trimmed.starts_with('#')
        || trimmed.starts_with('/')
        || trimmed.contains("data-testid")
        || trimmed.contains("data-test")
        || trimmed.contains("aria-")
        || trimmed.contains('-')
        || trimmed.contains('_')
}

fn surface_literal_is_module_specifier(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.starts_with("./")
        || trimmed.starts_with("../")
        || trimmed.starts_with("@/")
        || (trimmed.starts_with('@') && trimmed.contains('/'))
        || (trimmed.contains('/')
            && !trimmed.starts_with('/')
            && !trimmed.starts_with('.')
            && !trimmed.starts_with('#')
            && !trimmed.contains(char::is_whitespace))
}

fn surface_label_literal_is_structural(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.len() < 3 || trimmed.len() > 100 {
        return false;
    }
    if surface_literal_is_module_specifier(trimmed) {
        return false;
    }
    let terms = surface_phrase_terms(&normalize_surface_phrase(trimmed).unwrap_or_default());
    !terms.is_empty()
        && terms
            .iter()
            .all(|term| term.chars().all(|ch| ch.is_alphanumeric()))
}

fn surface_literal_phrases(value: &str, preserve_whole: bool) -> BTreeSet<String> {
    let route_surface = value.trim().starts_with('/');
    if preserve_whole
        && let Some(phrase) = normalize_surface_phrase(value)
        && (surface_phrase_is_specific(&phrase)
            || (route_surface && surface_phrase_terms(&phrase).len() >= 2))
    {
        return BTreeSet::from([phrase]);
    }
    value
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '>' | '+' | '~' | ',' | '[' | ']'))
        .filter_map(normalize_surface_phrase)
        .filter(|phrase| {
            surface_phrase_is_specific(phrase)
                || (route_surface && surface_phrase_terms(phrase).len() >= 2)
        })
        .collect()
}

fn normalize_surface_phrase(value: &str) -> Option<String> {
    let mut trimmed = value
        .trim()
        .trim_matches(|ch: char| {
            matches!(
                ch,
                '.' | '#' | '"' | '\'' | '`' | '(' | ')' | '{' | '}' | ';'
            )
        })
        .replace("__", "-")
        .replace(['.', '#', '/', '_', ':', '(', ')'], "-");
    trimmed = trimmed.split_whitespace().collect::<Vec<_>>().join("-");
    while trimmed.contains("--") {
        trimmed = trimmed.replace("--", "-");
    }
    let trimmed = trimmed.trim_matches('-').to_lowercase();
    if trimmed.is_empty()
        || trimmed.contains("${")
        || trimmed.starts_with("http")
        || trimmed.starts_with("mailto")
    {
        return None;
    }
    Some(trimmed)
}

fn surface_phrase_is_specific(phrase: &str) -> bool {
    let terms = surface_phrase_terms(phrase);
    terms.len() >= 2
        && terms
            .iter()
            .any(|term| !matches!(term.as_str(), "frame" | "title" | "canvas" | "node"))
}

fn surface_phrase_terms(phrase: &str) -> BTreeSet<String> {
    surface_terms(&phrase.replace(['.', '#', '/', '-', '_', ':'], " "))
        .into_iter()
        .filter(|term| term.len() >= 3)
        .filter(|term| {
            !matches!(
                term.as_str(),
                "the"
                    | "and"
                    | "for"
                    | "with"
                    | "from"
                    | "true"
                    | "false"
                    | "null"
                    | "undefined"
                    | "data"
                    | "test"
                    | "testid"
                    | "aria"
                    | "label"
                    | "role"
                    | "root"
                    | "blueprint"
                    | "nodrag"
                    | "nopan"
            )
        })
        .collect()
}

fn surface_literal_terms(value: &str) -> BTreeSet<String> {
    surface_terms(&value.replace(['.', '#', '/', '-', '_', ':'], " "))
        .into_iter()
        .filter(|term| term.len() >= 3)
        .filter(|term| {
            !matches!(
                term.as_str(),
                "the"
                    | "and"
                    | "for"
                    | "with"
                    | "from"
                    | "true"
                    | "false"
                    | "null"
                    | "undefined"
                    | "data"
                    | "test"
                    | "testid"
                    | "aria"
                    | "label"
                    | "role"
                    | "button"
                    | "link"
                    | "input"
                    | "text"
                    | "page"
                    | "root"
                    | "blueprint"
            )
        })
        .collect()
}

fn surface_terms(value: &str) -> BTreeSet<String> {
    value
        .split(|ch: char| !(ch.is_alphanumeric() || ch == '_'))
        .map(str::to_lowercase)
        .filter(|term| term.len() >= 2)
        .collect()
}

