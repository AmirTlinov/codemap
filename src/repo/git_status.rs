// Responsibility: repo-git-status
use crate::model::GitChange;
use crate::repo::{git_root, normalize_rel_path, should_ignore_rel};
use std::path::Path;

#[derive(Debug, Clone)]
pub(crate) struct GitStatusSnapshot {
    pub head: crate::model::HeadPrelude,
    pub branch: crate::model::BranchPrelude,
    pub worktree: crate::model::WorktreePrelude,
    pub changes: Vec<GitChange>,
}

pub(crate) fn git_status_snapshot(root: &Path) -> Option<GitStatusSnapshot> {
    let output = crate::repo::read_only_git_command()
        .arg("-C")
        .arg(root)
        .args([
            "status",
            "--porcelain=v2",
            "--branch",
            "-z",
            "-uall",
            "--",
            ".",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let root_prefix = git_status_root_prefix(root);
    let mut snapshot = GitStatusSnapshot {
        head: crate::model::HeadPrelude::default(),
        branch: crate::model::BranchPrelude::default(),
        worktree: crate::model::WorktreePrelude::default(),
        changes: Vec::new(),
    };
    let mut entries = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .peekable();
    while let Some(entry) = entries.next() {
        let record = String::from_utf8_lossy(entry);
        if let Some(header) = record.strip_prefix("# ") {
            parse_status_header(header, &mut snapshot);
            continue;
        }
        let Some(mut change) = status_v2_change(&record, &mut entries, root_prefix.as_deref())
        else {
            continue;
        };
        count_status_change(&change, &mut snapshot.worktree);
        if should_ignore_rel(&change.path)
            && matches!(change.status.as_str(), "renamed" | "untracked")
        {
            if change.status == "renamed"
                && let Some(old_path) = change.old_path.take()
                && !should_ignore_rel(&old_path)
            {
                snapshot.changes.push(GitChange {
                    path: old_path,
                    old_path: None,
                    status: "deleted".to_string(),
                    staged: change.staged,
                    unstaged: change.unstaged,
                    provenance: "git_status".to_string(),
                });
            }
            continue;
        }
        snapshot.changes.push(change);
    }
    snapshot.changes.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then_with(|| a.status.cmp(&b.status))
            .then_with(|| a.old_path.cmp(&b.old_path))
    });
    Some(snapshot)
}

fn parse_status_header(header: &str, snapshot: &mut GitStatusSnapshot) {
    if let Some(oid) = header.strip_prefix("branch.oid ") {
        if oid == "(initial)" {
            snapshot.head.unborn = true;
            snapshot.head.oid = None;
            snapshot.head.short = None;
        } else {
            snapshot.head.oid = Some(oid.to_string());
            snapshot.head.short = Some(oid.chars().take(12).collect());
        }
    } else if let Some(head) = header.strip_prefix("branch.head ") {
        if head == "(detached)" {
            snapshot.head.detached = true;
            snapshot.branch.name = None;
        } else {
            snapshot.branch.name = Some(head.to_string());
        }
    } else if let Some(upstream) = header.strip_prefix("branch.upstream ") {
        snapshot.branch.upstream = Some(upstream.to_string());
    } else if let Some(ab) = header.strip_prefix("branch.ab ") {
        let mut parts = ab.split_whitespace();
        snapshot.branch.ahead = parse_ab_count(parts.next(), '+');
        snapshot.branch.behind = parse_ab_count(parts.next(), '-');
    }
}

fn parse_ab_count(value: Option<&str>, prefix: char) -> Option<u32> {
    let value = value?;
    value.strip_prefix(prefix)?.parse().ok()
}

fn status_v2_change<'a, I>(
    record: &str,
    entries: &mut std::iter::Peekable<I>,
    root_prefix: Option<&str>,
) -> Option<GitChange>
where
    I: Iterator<Item = &'a [u8]>,
{
    if let Some(path) = record.strip_prefix("? ") {
        return Some(GitChange {
            path: normalize_status_path(path, root_prefix)?,
            old_path: None,
            status: "untracked".to_string(),
            staged: false,
            unstaged: true,
            provenance: "git_status".to_string(),
        });
    }
    if record.starts_with("! ") {
        return None;
    }
    if let Some(rest) = record.strip_prefix("1 ") {
        let (xy, path) = status_v2_xy_path(rest, 7)?;
        return Some(GitChange {
            path: normalize_status_path(path, root_prefix)?,
            old_path: None,
            status: porcelain_status_pair(xy).to_string(),
            staged: status_index(xy) != '.',
            unstaged: status_worktree(xy) != '.',
            provenance: "git_status".to_string(),
        });
    }
    if let Some(rest) = record.strip_prefix("2 ") {
        let (xy, path) = status_v2_xy_path(rest, 8)?;
        let old_path = entries
            .next()
            .and_then(|entry| normalize_status_path(&String::from_utf8_lossy(entry), root_prefix));
        return Some(GitChange {
            path: normalize_status_path(path, root_prefix)?,
            old_path,
            status: porcelain_status_pair(xy).to_string(),
            staged: status_index(xy) != '.',
            unstaged: status_worktree(xy) != '.',
            provenance: "git_status".to_string(),
        });
    }
    if let Some(rest) = record.strip_prefix("u ") {
        let (_xy, path) = status_v2_xy_path(rest, 9)?;
        return Some(GitChange {
            path: normalize_status_path(path, root_prefix)?,
            old_path: None,
            status: "conflicted".to_string(),
            staged: true,
            unstaged: true,
            provenance: "git_status_conflict".to_string(),
        });
    }
    None
}

fn status_v2_xy_path(record: &str, fields_before_path: usize) -> Option<(&str, &str)> {
    let mut split = record.splitn(fields_before_path + 1, ' ');
    let xy = split.next()?;
    for _ in 1..fields_before_path {
        split.next()?;
    }
    let path = split.next()?;
    Some((xy, path))
}

fn count_status_change(change: &GitChange, worktree: &mut crate::model::WorktreePrelude) {
    match change.status.as_str() {
        "untracked" => {
            worktree.untracked += 1;
            return;
        }
        "conflicted" => worktree.conflicted += 1,
        "renamed" => worktree.renamed += 1,
        "deleted" => worktree.deleted += 1,
        "typechanged" => worktree.typechanged += 1,
        _ => {}
    }
    if change.staged {
        worktree.staged += 1;
    }
    if change.unstaged {
        worktree.unstaged += 1;
    }
}

fn status_index(xy: &str) -> char {
    xy.chars().next().unwrap_or('.')
}

fn status_worktree(xy: &str) -> char {
    xy.chars().nth(1).unwrap_or('.')
}

fn porcelain_status_pair(xy: &str) -> &'static str {
    let index_status = status_index(xy);
    let worktree_status = status_worktree(xy);
    porcelain_status(index_status, worktree_status)
}

fn porcelain_status(index_status: char, worktree_status: char) -> &'static str {
    if matches!(
        (index_status, worktree_status),
        ('U', _) | (_, 'U') | ('A', 'A') | ('D', 'D')
    ) {
        "conflicted"
    } else if index_status == '?' {
        "untracked"
    } else if index_status == 'R' {
        "renamed"
    } else if index_status == 'D' || worktree_status == 'D' {
        "deleted"
    } else if index_status == 'T' || worktree_status == 'T' {
        "typechanged"
    } else {
        "modified"
    }
}

fn normalize_status_path(path: &str, root_prefix: Option<&str>) -> Option<String> {
    let rel = normalize_rel_path(path);
    if let Some(prefix) = root_prefix {
        return rel.strip_prefix(prefix).map(normalize_rel_path);
    }
    (!rel.is_empty()).then_some(rel)
}

pub(crate) fn git_status_root_prefix(root: &Path) -> Option<String> {
    let git_root = git_root(root)?;
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let git_root = git_root
        .canonicalize()
        .unwrap_or_else(|_| git_root.to_path_buf());
    if root == git_root {
        return None;
    }
    let rel = root.strip_prefix(git_root).ok()?;
    let rel = normalize_rel_path(&rel.to_string_lossy());
    (!rel.is_empty()).then(|| format!("{}/", rel.trim_end_matches('/')))
}
