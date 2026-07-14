// Responsibility: repo-surfaces-literal-context

pub(crate) fn quoted_prefix_is_page_goto_argument(prefix: &str) -> bool {
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

pub(crate) fn line_has_surface_context(line: &str) -> bool {
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

pub(crate) fn line_accepts_plain_label_surface(line: &str) -> bool {
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

pub(crate) fn line_has_jsx_surface_container(line: &str) -> bool {
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

pub(crate) fn quoted_value_is_module_specifier_context(prefix: &str) -> bool {
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
