// Responsibility: repo-changed
use crate::model::GitChange;
use crate::repo::{git_status_snapshot, normalize_rel_path, should_ignore_rel};
use std::collections::BTreeSet;
use std::path::Path;

pub fn changed_files(root: &Path, staged: bool, since: Option<&str>) -> Vec<String> {
    if let Some(since) = since {
        return changed_paths(git_name_status(
            root,
            &["diff", "--name-status", "--relative", since],
        ));
    }
    if staged {
        return changed_paths(git_name_status(
            root,
            &["diff", "--name-status", "--relative", "--cached"],
        ));
    }
    let mut files = BTreeSet::new();
    for change in git_status_changes(root) {
        files.insert(change.path);
    }
    files.into_iter().collect()
}

pub fn git_ref_exists(root: &Path, reference: &str) -> bool {
    crate::repo::read_only_git_command()
        .arg("-C")
        .arg(root)
        .args([
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{reference}^{{commit}}"),
        ])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub fn git_changes(root: &Path, staged_only: bool, since: Option<&str>) -> Vec<GitChange> {
    if let Some(since) = since {
        return git_name_status(root, &["diff", "--name-status", "--relative", since]);
    }
    if staged_only {
        return git_name_status(root, &["diff", "--name-status", "--relative", "--cached"]);
    }
    git_status_changes(root)
}

fn git_status_changes(root: &Path) -> Vec<GitChange> {
    git_status_snapshot(root)
        .map(|snapshot| snapshot.changes)
        .unwrap_or_default()
}

fn git_name_status(root: &Path, args: &[&str]) -> Vec<GitChange> {
    let output = crate::repo::read_only_git_command()
        .arg("-C")
        .arg(root)
        .args(args)
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let mut changes = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(name_status_line)
        .filter_map(visible_or_degraded_change)
        .collect::<Vec<_>>();
    changes.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then_with(|| a.status.cmp(&b.status))
            .then_with(|| a.old_path.cmp(&b.old_path))
    });
    changes
}

pub(crate) fn changed_paths(changes: Vec<GitChange>) -> Vec<String> {
    changes
        .into_iter()
        .map(|change| change.path)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn visible_or_degraded_change(change: GitChange) -> Option<GitChange> {
    if !should_ignore_rel(&change.path) || change.status != "renamed" {
        return Some(change);
    }
    if change.status == "renamed"
        && let Some(old_path) = change.old_path.as_deref()
        && !should_ignore_rel(old_path)
    {
        return Some(GitChange {
            path: old_path.to_string(),
            old_path: None,
            status: "deleted".to_string(),
            staged: change.staged,
            unstaged: change.unstaged,
            provenance: change.provenance,
        });
    }
    None
}

fn name_status_line(line: &str) -> Option<GitChange> {
    let mut parts = line.split('\t');
    let status = parts.next()?.to_string();
    let first = parts.next()?;
    let second = parts.next();
    let (old_path, path) = if status.starts_with('R') {
        (Some(normalize_rel_path(first)), normalize_rel_path(second?))
    } else {
        (None, normalize_rel_path(first))
    };
    Some(GitChange {
        path,
        old_path,
        status: name_status_kind(&status).to_string(),
        staged: true,
        unstaged: false,
        provenance: "git_diff_name_status".to_string(),
    })
}

fn name_status_kind(status: &str) -> &str {
    match status.chars().next().unwrap_or('M') {
        'A' => "added",
        'D' => "deleted",
        'R' => "renamed",
        'T' => "typechanged",
        'U' => "conflicted",
        _ => "modified",
    }
}
