// Responsibility: batched-git-index-inventory
use crate::repo::normalize_rel_path;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GitIndexKind {
    Regular,
    Symlink,
    Gitlink,
}

#[derive(Debug, Clone, Copy)]
struct GitIndexEntry {
    kind: GitIndexKind,
    needs_content_recheck: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct GitIndexInventory {
    entries: BTreeMap<String, GitIndexEntry>,
}

impl GitIndexInventory {
    pub(crate) fn kind(&self, rel: &str) -> Option<GitIndexKind> {
        self.entries.get(rel).map(|entry| entry.kind)
    }

    pub(crate) fn needs_content_recheck(&self, rel: &str) -> bool {
        self.entries
            .get(rel)
            .is_some_and(|entry| entry.needs_content_recheck)
    }

    pub(crate) fn paths(&self) -> impl Iterator<Item = String> + '_ {
        self.entries.keys().cloned()
    }

    pub(super) fn gitlinks(&self) -> BTreeSet<String> {
        self.entries
            .iter()
            .filter(|(_, entry)| entry.kind == GitIndexKind::Gitlink)
            .map(|(path, _)| path.clone())
            .collect()
    }
}

pub(crate) enum GitIndexProbe {
    NotRepository,
    Available(GitIndexInventory),
    Unavailable,
}

pub(crate) fn git_index_inventory(root: &Path) -> Option<GitIndexInventory> {
    match git_index_probe(root) {
        GitIndexProbe::Available(index) => Some(index),
        GitIndexProbe::NotRepository | GitIndexProbe::Unavailable => None,
    }
}

pub(crate) fn git_index_probe(root: &Path) -> GitIndexProbe {
    let output = crate::repo::read_only_git_command()
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--stage", "-v", "-z"])
        .output();
    let Ok(output) = output else {
        return repository_probe_failure(root);
    };
    if !output.status.success() {
        return repository_probe_failure(root);
    }
    GitIndexProbe::Available(parse_index_entries(&output.stdout))
}

fn repository_probe_failure(root: &Path) -> GitIndexProbe {
    if crate::repo::is_git_repo(root) {
        GitIndexProbe::Unavailable
    } else {
        GitIndexProbe::NotRepository
    }
}

fn parse_index_entries(bytes: &[u8]) -> GitIndexInventory {
    let mut entries = BTreeMap::<String, GitIndexEntry>::new();
    for record in bytes.split(|byte| *byte == 0) {
        let Some(tab) = record.iter().position(|byte| *byte == b'\t') else {
            continue;
        };
        let metadata = String::from_utf8_lossy(&record[..tab]);
        let mut fields = metadata.split_ascii_whitespace();
        let Some(flag) = fields.next() else {
            continue;
        };
        let Some(mode) = fields.next() else {
            continue;
        };
        let rel = normalize_rel_path(&String::from_utf8_lossy(&record[tab + 1..]));
        if rel.is_empty() {
            continue;
        }
        let entry = GitIndexEntry {
            kind: match mode {
                "160000" => GitIndexKind::Gitlink,
                "120000" => GitIndexKind::Symlink,
                _ => GitIndexKind::Regular,
            },
            needs_content_recheck: flag != "H",
        };
        entries
            .entry(rel)
            .and_modify(|current| {
                current.needs_content_recheck |= entry.needs_content_recheck;
                if entry.kind == GitIndexKind::Gitlink || current.kind == GitIndexKind::Regular {
                    current.kind = entry.kind;
                }
            })
            .or_insert(entry);
    }
    GitIndexInventory { entries }
}
