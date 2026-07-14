// Responsibility: map-test-surface-terms
use crate::model::FileInfo;
use crate::repo;
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) fn test_surface_terms(file: &FileInfo) -> BTreeSet<String> {
    let mut terms = semantic_path_terms(&file.rel);
    terms.extend(file.surface_tokens.iter().cloned());
    terms
}

pub(crate) fn semantic_path_terms(path: &str) -> BTreeSet<String> {
    let normalized = repo::normalize_rel_path(path);
    let without_ext = Path::new(&normalized)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(normalized.as_str());
    repo::tokenize(&normalized)
        .into_iter()
        .chain(semantic_name_terms(without_ext))
        .filter(|term| meaningful_surface_term(term))
        .collect()
}

pub(crate) fn semantic_name_terms(name: &str) -> BTreeSet<String> {
    let mut expanded = String::new();
    let mut previous_lower_or_digit = false;
    for ch in name.chars() {
        if ch == '-' || ch == '_' || ch == '.' || ch == '/' {
            expanded.push(' ');
            previous_lower_or_digit = false;
            continue;
        }
        if ch.is_ascii_uppercase() && previous_lower_or_digit {
            expanded.push(' ');
        }
        previous_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        expanded.push(ch);
    }
    repo::tokenize(&expanded)
        .into_iter()
        .filter(|term| meaningful_surface_term(term))
        .collect()
}

pub(crate) fn meaningful_surface_term(term: &str) -> bool {
    term.len() >= 3
        && !matches!(
            term,
            "app"
                | "apps"
                | "src"
                | "lib"
                | "libs"
                | "test"
                | "tests"
                | "spec"
                | "unit"
                | "e2e"
                | "tsx"
                | "jsx"
                | "mjs"
                | "cjs"
                | "typescript"
                | "javascript"
                | "component"
                | "components"
                | "feature"
                | "features"
                | "page"
                | "pages"
                | "hook"
                | "hooks"
                | "util"
                | "utils"
                | "index"
                | "main"
                | "type"
                | "types"
                | "support"
                | "setup"
                | "helper"
                | "helpers"
                | "fixture"
                | "fixtures"
                | "blueprint"
        )
}

pub(crate) fn meaningful_surface_phrase(phrase: &str) -> bool {
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

pub(crate) fn surface_terms(value: &str) -> BTreeSet<String> {
    value
        .split(|ch: char| !(ch.is_alphanumeric() || ch == '_'))
        .map(str::to_lowercase)
        .filter(|term| term.len() >= 2)
        .collect()
}
