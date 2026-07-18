// Responsibility: repo-surfaces-literal-context

use crate::repo::{normalize_route_path, quoted_strings};

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

pub(crate) fn static_url_route_binding(line: &str) -> Option<(String, String)> {
    let (declaration, value) = line.split_once('=')?;
    let binding = declaration
        .trim()
        .strip_prefix("const ")
        .or_else(|| declaration.trim().strip_prefix("let "))
        .or_else(|| declaration.trim().strip_prefix("var "))?
        .trim();
    if !javascript_identifier(binding) {
        return None;
    }
    let value = value.trim_start();
    let arguments = value
        .strip_prefix("new URL")?
        .trim_start()
        .strip_prefix('(')?;
    let literal = quoted_strings(arguments).into_iter().next()?.value;
    let route = absolute_url_route(&literal)?;
    Some((binding.to_string(), route))
}

pub(crate) fn page_goto_url_binding(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let index = lower.find("page.goto")?;
    if lower[..index]
        .chars()
        .next_back()
        .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$')
    {
        return None;
    }
    let arguments = line[index + "page.goto".len()..]
        .trim_start()
        .strip_prefix('(')?;
    let first = arguments.split([',', ')']).next()?.trim();
    let binding = first
        .strip_suffix(".toString(")
        .or_else(|| first.strip_suffix(".href"))
        .unwrap_or(first)
        .trim();
    javascript_identifier(binding).then(|| binding.to_string())
}

fn absolute_url_route(value: &str) -> Option<String> {
    let (_, authority_and_path) = value.split_once("://")?;
    let path_start = authority_and_path.find('/')?;
    normalize_route_path(&authority_and_path[path_start..])
}

fn javascript_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || matches!(first, '_' | '$'))
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '$'))
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
