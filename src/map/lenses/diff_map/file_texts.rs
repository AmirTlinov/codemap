// Responsibility: diff-map-file-texts
use crate::map::{DiffMapMode, git_show_files};
use crate::model::Project;
use std::collections::BTreeMap;

pub(crate) fn diff_path_needs_runtime_scan(rel: &str) -> bool {
    let lower = rel.to_ascii_lowercase();
    if lower.ends_with("lock")
        || lower.ends_with(".lock")
        || lower.ends_with("-lock.yaml")
        || lower.ends_with("-lock.yml")
        || lower.ends_with("package-lock.json")
        || lower.ends_with("cargo.lock")
        || lower.ends_with("pnpm-lock.yaml")
        || lower.ends_with("yarn.lock")
        || lower.ends_with("bun.lockb")
    {
        return false;
    }
    matches!(
        std::path::Path::new(rel)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some(
            "ts" | "tsx"
                | "js"
                | "jsx"
                | "mjs"
                | "cjs"
                | "rs"
                | "go"
                | "py"
                | "swift"
                | "java"
                | "kt"
                | "kts"
                | "sh"
                | "bash"
                | "zsh"
        )
    )
}

pub(crate) fn diff_current_file_texts(
    project: &Project,
    rels: &[String],
    mode: &DiffMapMode,
) -> BTreeMap<String, String> {
    match mode {
        DiffMapMode::Staged => git_show_files(project, ":", rels),
        DiffMapMode::WorkingTree | DiffMapMode::Since(_) => rels
            .iter()
            .filter_map(|rel| {
                std::fs::read_to_string(project.root.join(rel))
                    .ok()
                    .map(|text| (rel.clone(), text))
            })
            .collect(),
    }
}

pub(crate) fn diff_current_file_text(
    project: &Project,
    rel: &str,
    mode: &DiffMapMode,
) -> Option<String> {
    diff_current_file_texts(project, &[rel.to_string()], mode).remove(rel)
}

pub(crate) fn diff_base_file_texts(
    project: &Project,
    rels: &[String],
    mode: &DiffMapMode,
) -> BTreeMap<String, String> {
    let revision = match mode {
        DiffMapMode::WorkingTree | DiffMapMode::Staged => "HEAD",
        DiffMapMode::Since(base) => base.as_str(),
    };
    git_show_files(project, revision, rels)
}

pub(crate) fn diff_base_file_text(
    project: &Project,
    rel: &str,
    mode: &DiffMapMode,
) -> Option<String> {
    diff_base_file_texts(project, &[rel.to_string()], mode).remove(rel)
}
