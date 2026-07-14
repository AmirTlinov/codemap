// Responsibility: map-test-surface
use crate::model::{FileInfo, Project};
use crate::repo;
use std::collections::BTreeSet;
use std::path::Path;

mod proof_signals;
pub(crate) use proof_signals::*;
mod terms;
pub(crate) use terms::*;

pub(crate) fn shared_surface_phrases(
    project: &Project,
    rel: &str,
    test: &FileInfo,
) -> BTreeSet<String> {
    project
        .files
        .get(rel)
        .map(|file| {
            let mut shared = BTreeSet::new();
            for source_phrase in &file.surface_phrases {
                if !meaningful_surface_phrase(source_phrase) {
                    continue;
                }
                for test_phrase in &test.surface_phrases {
                    if !meaningful_surface_phrase(test_phrase) {
                        continue;
                    }
                    if surface_phrases_match(source_phrase, test_phrase) {
                        shared.insert(source_phrase.clone());
                    }
                }
            }
            shared
        })
        .unwrap_or_default()
}

fn surface_phrases_match(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }
    if accessible_role_name_surface(left) || accessible_role_name_surface(right) {
        return false;
    }
    let (shorter, longer) = if left.len() <= right.len() {
        (left, right)
    } else {
        (right, left)
    };
    let shorter_terms = surface_phrase_terms(shorter);
    shorter_terms.len() >= 3 && phrase_contains_with_boundaries(longer, shorter)
}

fn accessible_role_name_surface(phrase: &str) -> bool {
    phrase.starts_with("a11y-role-")
}

fn phrase_contains_with_boundaries(longer: &str, shorter: &str) -> bool {
    longer.match_indices(shorter).any(|(start, _)| {
        let before = longer[..start].chars().next_back();
        let end = start + shorter.len();
        let after = longer[end..].chars().next();
        before.map(phrase_boundary_char).unwrap_or(true)
            && after.map(phrase_boundary_char).unwrap_or(true)
    })
}

fn phrase_boundary_char(ch: char) -> bool {
    ch == '-'
}

pub(crate) fn source_stem(rel: &str) -> String {
    Path::new(rel)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .replace(".test", "")
        .replace(".spec", "")
        .to_ascii_lowercase()
}

pub(crate) fn test_name_matches_source_stem(test_rel: &str, source_stem: &str) -> bool {
    test_stem(test_rel) == source_stem
}

fn test_stem(test_rel: &str) -> String {
    let mut stem = Path::new(test_rel)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    for suffix in [".test", ".spec", "_test"] {
        if let Some(stripped) = stem.strip_suffix(suffix) {
            stem = stripped.to_string();
        }
    }
    stem
}

pub(crate) fn meaningful_stem(stem: &str) -> bool {
    !stem.is_empty() && !matches!(stem, "index" | "mod" | "main" | "lib" | "types")
}

pub(crate) fn same_parent_or_test_scope(source: &str, test: &str) -> bool {
    let source_parent = Path::new(source)
        .parent()
        .map(|path| repo::normalize_rel_path(&path.to_string_lossy()))
        .unwrap_or_else(|| ".".to_string());
    if test.starts_with(&format!("{}/", source_parent.trim_end_matches('/'))) {
        return true;
    }
    let source_stem = source_stem(source);
    meaningful_stem(&source_stem) && test.to_ascii_lowercase().contains(&source_stem)
}

pub(crate) fn surface_priority(kind: &str) -> usize {
    if kind == "domain" {
        return 0;
    }
    if kind.starts_with("package:") {
        return 1;
    }
    if kind == "dir" {
        return 2;
    }
    if kind == "script" {
        return 3;
    }
    if kind.starts_with("recursive:") {
        return 11;
    }
    if kind.starts_with("support_package:") {
        return 10;
    }
    match kind {
        "schema_contract" | "public_boundary" => 4,
        "runtime_state" | "persistence" | "adapter" | "parser" | "renderer_ui" => 5,
        "test" | "e2e_test" | "test_support" => 6,
        "source" => 7,
        "config" | "build_ci" => 8,
        _ => 9,
    }
}
