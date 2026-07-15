// Responsibility: runtime-scope-cache-identity
use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use super::{fingerprint, hex_prefix};
use crate::model::{IndexedBoundary, Project};
use crate::repo::{GitIndexInventory, GitIndexKind};

pub fn runtime_scope_fingerprint(project: &Project, scope: &str) -> String {
    let project_snapshot = fingerprint(project, None);
    runtime_scope_fingerprint_from_project_snapshot(&project.root, scope, &project_snapshot)
}

pub(crate) fn runtime_scope_fingerprint_from_project_snapshot(
    root: &Path,
    scope: &str,
    project_snapshot: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"runtime_scope_v3");
    hasher.update([0]);
    hasher.update(project_snapshot.as_bytes());
    hasher.update([0]);
    hasher.update(scope.as_bytes());
    hasher.update([0]);
    hasher.update(runtime_scope_physical_identity(root, scope));
    hex_prefix(&hasher.finalize(), 16)
}

pub fn runtime_scope_is_logically_empty(root: &Path, scope: &str) -> bool {
    runtime_scope_physical_identity(root, scope) == b"directory_empty"
}

pub fn runtime_scope_has_unindexed_entries(project: &Project, scope: &str) -> bool {
    let path = scope_path(&project.root, scope);
    let Ok(metadata) = fs::symlink_metadata(&path) else {
        return false;
    };
    metadata.is_dir() && runtime_directory_has_unindexed_entries(project, scope, &path)
}

fn runtime_directory_has_unindexed_entries(project: &Project, scope: &str, path: &Path) -> bool {
    let Ok(entries) = fs::read_dir(path) else {
        return true;
    };
    for entry in entries {
        let Ok(entry) = entry else {
            return true;
        };
        let rel = child_rel(scope, &entry.file_name().to_string_lossy());
        if runtime_scope_entry_is_disjoint(&rel) {
            continue;
        }
        let Ok(kind) = entry.file_type() else {
            return true;
        };
        if kind.is_dir() {
            if project.files.get(&rel).is_some_and(|file| {
                matches!(
                    file.indexed_boundary,
                    Some(IndexedBoundary::ExternalTree | IndexedBoundary::ExternalGitlink)
                )
            }) {
                continue;
            }
            if runtime_directory_has_unindexed_entries(project, &rel, &entry.path()) {
                return true;
            }
        } else if !project.files.contains_key(&rel)
            && crate::repo::is_cache_candidate_file(&project.root, &rel)
        {
            return true;
        }
    }
    false
}

fn runtime_scope_physical_identity(root: &Path, scope: &str) -> Vec<u8> {
    let path = scope_path(root, scope);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return b"missing".to_vec(),
        Err(_) => return b"unavailable".to_vec(),
    };
    if metadata.file_type().is_symlink() {
        return b"symlink".to_vec();
    }
    if metadata.is_file() {
        return b"file".to_vec();
    }
    if !metadata.is_dir() {
        return b"other".to_vec();
    }
    let git_index = crate::repo::git_index_inventory(root);
    if scope != "." && indexed_gitlink(git_index.as_ref(), scope) {
        return b"external_tree".to_vec();
    }
    let mut entries = Vec::new();
    if collect_semantic_entries(root, scope, git_index.as_ref(), &mut entries).is_err() {
        return b"directory_unavailable".to_vec();
    }
    if entries.is_empty() {
        return b"directory_empty".to_vec();
    }
    entries.sort();
    let mut hasher = Sha256::new();
    hasher.update(b"runtime_directory_entries_v2");
    for entry in entries {
        hasher.update([0]);
        hasher.update(entry.as_bytes());
    }
    format!("directory_entries:{}", hex_prefix(&hasher.finalize(), 32)).into_bytes()
}

fn collect_semantic_entries(
    root: &Path,
    scope: &str,
    git_index: Option<&GitIndexInventory>,
    out: &mut Vec<String>,
) -> Result<(), ()> {
    let entries = fs::read_dir(scope_path(root, scope)).map_err(|_| ())?;
    for entry in entries {
        let entry = entry.map_err(|_| ())?;
        let rel = child_rel(scope, &entry.file_name().to_string_lossy());
        if runtime_scope_entry_is_disjoint(&rel) {
            continue;
        }
        let kind = entry.file_type().map_err(|_| ())?;
        if kind.is_dir() {
            if indexed_gitlink(git_index, &rel) {
                out.push(format!("{rel}\0external_tree"));
            } else {
                collect_semantic_entries(root, &rel, git_index, out)?;
            }
        } else if kind.is_symlink() {
            out.push(format!("{rel}\0symlink"));
        } else if crate::repo::is_cache_candidate_file(root, &rel) {
            let kind = if kind.is_file() { "file" } else { "other" };
            out.push(format!("{rel}\0{kind}"));
        }
    }
    Ok(())
}

fn indexed_gitlink(git_index: Option<&GitIndexInventory>, rel: &str) -> bool {
    git_index.and_then(|index| index.kind(rel)) == Some(GitIndexKind::Gitlink)
}

fn scope_path(root: &Path, scope: &str) -> std::path::PathBuf {
    if scope == "." {
        root.to_path_buf()
    } else {
        root.join(scope)
    }
}

fn child_rel(scope: &str, name: &str) -> String {
    if scope == "." {
        name.to_string()
    } else {
        format!("{scope}/{name}")
    }
}

fn runtime_scope_entry_is_disjoint(rel: &str) -> bool {
    rel.split('/').any(|part| part == ".codemap") || crate::repo::should_ignore_rel(rel)
}
