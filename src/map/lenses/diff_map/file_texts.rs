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
        DiffMapMode::WorkingTree | DiffMapMode::Since(_) | DiffMapMode::Snapshot(_) => rels
            .iter()
            .filter_map(|rel| diff_worktree_blob_text(project, rel).map(|text| (rel.clone(), text)))
            .collect(),
    }
}

/// Reads the content Git assigns to a working-tree path without following links.
/// Regular bodies retain the indexed-readable boundary; a symlink's blob is its
/// target path, not the contents of that target.
pub(crate) fn diff_worktree_blob_text(project: &Project, rel: &str) -> Option<String> {
    let path = project.root.join(rel);
    let metadata = std::fs::symlink_metadata(&path).ok()?;
    if metadata.file_type().is_symlink() {
        return std::fs::read_link(path)
            .ok()
            .map(|target| target.to_string_lossy().into_owned());
    }
    project.read_indexed_text(rel)
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
    if let DiffMapMode::Snapshot(snapshot) = mode {
        return rels
            .iter()
            .filter_map(|rel| {
                snapshot
                    .texts
                    .get(rel)
                    .map(|text| (rel.clone(), text.clone()))
            })
            .collect();
    }
    let revision = match mode {
        DiffMapMode::WorkingTree | DiffMapMode::Staged => "HEAD",
        DiffMapMode::Since(base) => base.as_str(),
        DiffMapMode::Snapshot(_) => unreachable!("snapshot returned above"),
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
