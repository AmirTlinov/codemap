// Responsibility: transient exact-file hydration outside the bounded inventory
use crate::model::Project;
use crate::repo::{
    GitIndexInventory, GitIndexKind, ScanStatsBuilder, git_index_inventory, rebuild_project_facts,
    scan_regular_file, should_ignore_rel,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub(crate) fn is_explicit_tracked_file(root: &Path, rel: &str) -> bool {
    let Some(index) = git_index_inventory(root) else {
        return false;
    };
    is_regular_tracked_ignored_file(root, rel, &index)
}

pub(crate) fn hydrate_explicit_project_files(
    project: &mut Project,
    rels: &BTreeSet<String>,
) -> bool {
    let Some(index) = git_index_inventory(&project.root) else {
        return false;
    };
    let mut stats = ScanStatsBuilder::default();
    let mut hydrated = false;
    for rel in rels {
        if !is_regular_tracked_ignored_file(&project.root, rel, &index) {
            continue;
        }
        if let Some(file) = scan_regular_file(&project.root, rel, &mut stats) {
            project.files.insert(rel.clone(), file);
            hydrated = true;
        }
    }
    if !hydrated {
        return false;
    }
    rebuild_project_facts(project);
    true
}

fn is_regular_tracked_ignored_file(root: &Path, rel: &str, index: &GitIndexInventory) -> bool {
    should_ignore_rel(rel)
        && index.kind(rel) == Some(GitIndexKind::Regular)
        && fs::symlink_metadata(root.join(rel))
            .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
}
