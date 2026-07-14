// Responsibility: repo-packages-rel-paths
use crate::repo::normalize_rel_path;
use std::path::Path;

pub(crate) fn resolve_repo_relative_path(base: &Path, path: &str) -> Option<String> {
    let raw = path.trim().replace('\\', "/");
    if raw.is_empty() || path_is_absolute_like(&raw) {
        return None;
    }
    let base = normalize_rel_path(&base.to_string_lossy());
    let mut parts: Vec<String> = if base == "." {
        Vec::new()
    } else {
        base.split('/').map(str::to_string).collect()
    };
    for part in raw.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other.to_string()),
        }
    }
    Some(if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    })
}

pub(crate) fn path_is_absolute_like(path: &str) -> bool {
    path.starts_with('/')
        || path.starts_with("//")
        || path
            .split('/')
            .next()
            .is_some_and(|part| part.ends_with(':'))
}
