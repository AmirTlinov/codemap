// Responsibility: repo-surfaces-phrase-terms
use std::collections::BTreeSet;

pub(crate) fn normalize_route_path(value: &str) -> Option<String> {
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

pub(crate) fn surface_literal_is_structural(value: &str) -> bool {
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

pub(crate) fn surface_label_literal_is_structural(value: &str) -> bool {
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

pub(crate) fn surface_literal_phrases(value: &str, preserve_whole: bool) -> BTreeSet<String> {
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

pub(crate) fn normalize_surface_phrase(value: &str) -> Option<String> {
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

pub(crate) fn surface_phrase_terms(phrase: &str) -> BTreeSet<String> {
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

pub(crate) fn surface_literal_terms(value: &str) -> BTreeSet<String> {
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

pub(crate) fn surface_terms(value: &str) -> BTreeSet<String> {
    value
        .split(|ch: char| !(ch.is_alphanumeric() || ch == '_'))
        .map(str::to_lowercase)
        .filter(|term| term.len() >= 2)
        .collect()
}
