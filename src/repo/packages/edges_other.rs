// Responsibility: repo-packages-edges-other
use crate::repo::normalize_rel_path;
use std::path::Path;

mod cargo_toml;
mod go_mod;
mod python_swift;

pub(crate) use cargo_toml::*;
pub(crate) use go_mod::*;
pub(crate) use python_swift::*;

pub(crate) fn manifest_dir(rel: &str) -> String {
    Path::new(rel)
        .parent()
        .map(|p| normalize_rel_path(&p.to_string_lossy()))
        .filter(|p| p != ".")
        .unwrap_or_else(|| ".".to_string())
}

pub(crate) fn package_name_from_path(path: &str) -> String {
    if path == "." {
        "repo".to_string()
    } else {
        Path::new(path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(path)
            .to_string()
    }
}

pub(crate) fn unquote(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_end_matches(',');
    trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| {
            trimmed
                .strip_prefix('\'')
                .and_then(|s| s.strip_suffix('\''))
        })
        .map(str::to_string)
}
