// Responsibility: repo-scan-file-filters
use crate::repo::{
    BINARY_EXTS, SOURCE_EXTS, TEXT_EXTS, is_asset_ext, is_build_ci_surface, is_env_surface_name,
    is_lockfile_name, is_script_ext, is_snapshot_ext, path_tokens,
    runtime_worker_or_job_convention,
};
use std::path::Path;

pub(crate) fn scan_file_rejection(path: &Path, rel: &str, size: u64) -> Option<&'static str> {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if is_env_surface_name(&name) || is_lockfile_name(&name) {
        return None;
    }
    if size > 900_000 && !is_asset_ext(&ext) {
        return Some("too_large");
    }
    if size > 5_000_000 && is_asset_ext(&ext) {
        return Some("too_large_asset");
    }
    if is_structural_file_name(&name) {
        return None;
    }
    if is_runtime_path_surface(rel, &name, &ext) {
        return None;
    }
    if BINARY_EXTS.iter().any(|x| x == &ext) && !is_asset_ext(&ext) {
        return Some("binary_extension");
    }
    if SOURCE_EXTS.iter().any(|x| x == &ext)
        || TEXT_EXTS.iter().any(|x| x == &ext)
        || is_asset_ext(&ext)
        || is_snapshot_ext(&ext)
    {
        None
    } else {
        Some("unsupported_extension")
    }
}

fn is_structural_file_name(name: &str) -> bool {
    matches!(
        name,
        "package.json"
            | "pyproject.toml"
            | "cargo.toml"
            | "go.mod"
            | "go.work"
            | "agents.md"
            | "readme.md"
            | "makefile"
            | "gnumakefile"
            | "justfile"
            | "jenkinsfile"
            | "dockerfile"
            | "earthfile"
            | "taskfile"
            | "taskfile.yml"
            | "taskfile.yaml"
            | ".env.example"
            | ".env.sample"
            | ".codemap.yml"
            | ".codemap.yaml"
            | ".codemap.json"
    )
}

pub(crate) fn scan_rejection_keeps_placeholder(path: &Path, rel: &str, reason: &str) -> bool {
    reason == "too_large" && structural_placeholder_path(path, rel)
}

pub(crate) fn structural_placeholder_path(path: &Path, rel: &str) -> bool {
    if supported_source_path(path) {
        return true;
    }
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if TEXT_EXTS.iter().any(|candidate| candidate == &ext)
        || is_env_surface_name(&name)
        || is_lockfile_name(&name)
    {
        return true;
    }
    is_structural_file_name(&name) || is_runtime_path_surface(rel, &name, &ext)
}

fn is_runtime_path_surface(rel: &str, name: &str, ext: &str) -> bool {
    let rel = rel.to_ascii_lowercase();
    runtime_worker_or_job_convention(&rel)
        || is_build_ci_surface(&rel, name, ext, &path_tokens(&rel))
}

pub(crate) fn source_parser_requires_placeholder(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "mts" | "cts"
    )
}

fn supported_source_path(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    SOURCE_EXTS.iter().any(|candidate| candidate == &ext) || is_script_ext(&ext)
}

pub(crate) fn language_for(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if is_env_surface_name(&name) {
        return "env".to_string();
    }
    if is_lockfile_name(&name) {
        return "lockfile".to_string();
    }
    if matches!(name.as_str(), "dockerfile" | "jenkinsfile" | "earthfile") {
        return "config".to_string();
    }
    match ext.as_str() {
        "ts" | "tsx" | "mts" | "cts" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte" => {
            "javascript/typescript"
        }
        "py" => "python",
        "rs" => "rust",
        "go" => "go",
        "swift" => "swift",
        "json" | "toml" | "yaml" | "yml" => "config",
        "prisma" | "graphql" | "gql" | "proto" | "avsc" => "schema",
        "sql" => "sql",
        "css" | "scss" | "sass" | "less" => "style",
        ext if is_script_ext(ext) => "shell",
        ext if is_asset_ext(ext) => "asset",
        ext if is_snapshot_ext(ext) => "snapshot",
        "md" => "markdown",
        _ => "unknown",
    }
    .to_string()
}
