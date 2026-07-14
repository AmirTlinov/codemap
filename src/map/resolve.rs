// Responsibility: map-resolve
use crate::map::root_domain;
use crate::model::{Domain, Project};
use crate::repo;
use std::collections::BTreeSet;

pub fn resolve_anchor_path(project: &Project, pattern: &str) -> String {
    let domain = root_domain(project);
    resolve_domain_pattern(&domain, pattern)
}

pub(crate) fn resolve_domain_pattern(domain: &Domain, pattern: &str) -> String {
    let p = pattern.trim().trim_start_matches("./");
    if p.starts_with('/') {
        return repo::normalize_rel_path(p);
    }
    if domain.path == "." {
        return repo::normalize_rel_path(p);
    }
    let domain_path = domain.path.trim_end_matches('/');
    if p == domain_path || p.starts_with(&format!("{domain_path}/")) {
        return repo::normalize_rel_path(p);
    }
    if p.starts_with("domains/")
        || p.starts_with("packages/")
        || p.starts_with("apps/")
        || p.starts_with("services/")
        || p.starts_with("libs/")
        || p.starts_with("crates/")
        || p.starts_with("modules/")
        || p.starts_with("cmd/")
        || p.starts_with("components/")
    {
        repo::normalize_rel_path(p)
    } else {
        repo::normalize_rel_path(&format!("{domain_path}/{p}"))
    }
}

pub(crate) fn glob_match(pattern: &str, value: &str) -> bool {
    glob_match_parts(
        &pattern.split('/').collect::<Vec<_>>(),
        &value.split('/').collect::<Vec<_>>(),
    )
}

fn glob_match_parts(pattern: &[&str], value: &[&str]) -> bool {
    if pattern.is_empty() {
        return value.is_empty();
    }
    if pattern[0] == "**" {
        return glob_match_parts(&pattern[1..], value)
            || (!value.is_empty() && glob_match_parts(pattern, &value[1..]));
    }
    if value.is_empty() {
        return false;
    }
    segment_match(pattern[0], value[0]) && glob_match_parts(&pattern[1..], &value[1..])
}

fn segment_match(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if !pattern.contains('*') {
        return pattern == value;
    }
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut rest = value;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 && !rest.starts_with(part) {
            return false;
        }
        if let Some(pos) = rest.find(part) {
            rest = &rest[pos + part.len()..];
        } else {
            return false;
        }
    }
    pattern.ends_with('*')
        || parts
            .last()
            .map(|last| value.ends_with(last))
            .unwrap_or(true)
}

pub(crate) fn unique(items: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for item in items {
        if !item.is_empty() && seen.insert(item.clone()) {
            out.push(item);
        }
    }
    out
}

pub(crate) fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}
