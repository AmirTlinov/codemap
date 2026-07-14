// Responsibility: map-test-surface-proof-signals
use super::{semantic_name_terms, semantic_path_terms, source_stem};
use crate::map::{
    domain_by_rel, package_for_rel, same_symbol_reference_scope, scoped_domain_path_for_rel,
    swift_source_scope, swift_test_package_root,
};
use crate::model::{FileInfo, Project};
use std::collections::BTreeSet;

pub(crate) fn test_imports_support_consuming_anchor(
    project: &Project,
    rel: &str,
    test: &FileInfo,
) -> bool {
    let mut seen = BTreeSet::new();
    let mut frontier = test
        .resolved_imports
        .iter()
        .filter_map(|import| project.files.get(import))
        .filter(|file| file.has_role("test_support"))
        .map(|file| file.rel.clone())
        .collect::<Vec<_>>();
    for _ in 0..2 {
        if frontier.is_empty() {
            return false;
        }
        let mut next = Vec::new();
        for support_rel in frontier {
            if !seen.insert(support_rel.clone()) {
                continue;
            }
            let Some(support) = project.files.get(&support_rel) else {
                continue;
            };
            if support.resolved_imports.contains(rel) {
                return true;
            }
            next.extend(
                support
                    .resolved_imports
                    .iter()
                    .filter_map(|import| project.files.get(import))
                    .filter(|file| file.has_role("test_support"))
                    .map(|file| file.rel.clone()),
            );
        }
        frontier = next;
    }
    false
}

pub(crate) fn swift_test_can_prove_anchor(project: &Project, rel: &str, test: &FileInfo) -> bool {
    let Some(anchor) = project.files.get(rel) else {
        return true;
    };
    if anchor.ext != "swift" || test.ext != "swift" {
        return true;
    }
    let Some((root, target)) = swift_source_scope(&anchor.rel) else {
        return false;
    };
    swift_test_package_root(&test.rel)
        .map(|test_root| test_root == root)
        .unwrap_or(false)
        && test.imports.contains(&target)
}

pub(crate) fn test_references_anchor_symbol(project: &Project, rel: &str, test: &FileInfo) -> bool {
    let Some(anchor) = project.files.get(rel) else {
        return false;
    };
    if anchor_symbol_reference_names(anchor).is_empty() {
        return false;
    }
    let source_domain = scoped_domain_path_for_rel(project, rel, domain_by_rel(project, rel));
    let test_domain = scoped_domain_path_for_rel(project, &test.rel, domain_by_rel(project, rel));
    if source_domain.is_some() && source_domain != test_domain {
        return false;
    }
    let source_package = package_for_rel(project, rel).map(|package| package.path.clone());
    let test_package = package_for_rel(project, &test.rel).map(|package| package.path.clone());
    if source_package.is_some() && source_package != test_package {
        return false;
    }
    if !same_symbol_reference_scope(anchor, test) {
        return false;
    }
    anchor_symbol_reference_names(anchor)
        .iter()
        .any(|name| test.references.contains(name))
}

pub(crate) fn anchor_symbol_reference_names(anchor: &FileInfo) -> BTreeSet<String> {
    anchor
        .symbols
        .iter()
        .filter(|symbol| symbol.kind != "method")
        .filter(|symbol| symbol.exported || structural_anchor_symbol_kind(&symbol.kind))
        .map(|symbol| symbol.name.clone())
        .filter(|name| meaningful_symbol_reference_name(name))
        .collect()
}

fn meaningful_symbol_reference_name(name: &str) -> bool {
    if name == "default" || name.len() < 4 {
        return false;
    }
    let terms = semantic_name_terms(name);
    !terms.is_empty()
}

pub(crate) fn anchor_terms(project: &Project, rel: &str) -> BTreeSet<String> {
    let mut terms = semantic_path_terms(rel);
    if let Some(file) = project.files.get(rel) {
        for symbol in &file.symbols {
            if symbol.exported || structural_anchor_symbol_kind(&symbol.kind) {
                terms.extend(semantic_name_terms(&symbol.name));
            }
        }
        for export in &file.exports {
            terms.extend(semantic_name_terms(export));
        }
        terms.extend(file.surface_tokens.iter().cloned());
    }
    terms
}

pub(crate) fn anchor_core_terms(project: &Project, rel: &str) -> BTreeSet<String> {
    let mut terms = semantic_name_terms(&source_stem(rel));
    if let Some(file) = project.files.get(rel) {
        for symbol in &file.symbols {
            if symbol.exported {
                terms.extend(semantic_name_terms(&symbol.name));
            }
        }
        for export in &file.exports {
            terms.extend(semantic_name_terms(export));
        }
    }
    terms
}

fn structural_anchor_symbol_kind(kind: &str) -> bool {
    matches!(
        kind,
        "component"
            | "function"
            | "class"
            | "interface"
            | "type"
            | "struct"
            | "enum"
            | "trait"
            | "method"
    )
}
