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
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain=v1", "-z", "-uall", "--", "."])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let root_prefix = git_status_root_prefix(root);
    let mut out = Vec::new();
    let mut entries = output.stdout.split(|byte| *byte == 0).filter(|entry| !entry.is_empty());
    while let Some(entry) = entries.next() {
        if entry.len() < 4 {
            continue;
        }
        let record = String::from_utf8_lossy(entry);
        let index_status = record.chars().next().unwrap_or(' ');
        let worktree_status = record.chars().nth(1).unwrap_or(' ');
        let raw_path = &record[3..];
        let old_path = if index_status == 'R' || index_status == 'C' {
            entries.next().and_then(|entry| {
                normalize_status_path(&String::from_utf8_lossy(entry), root_prefix.as_deref())
            })
        } else {
            None
        };
        let Some(path) = normalize_status_path(raw_path, root_prefix.as_deref()) else {
            continue;
        };
        if should_ignore_rel(&path) {
            if matches!(index_status, 'R' | 'C')
                && let Some(old_path) = old_path
                && !should_ignore_rel(&old_path)
            {
                out.push(GitChange {
                    path: old_path,
                    old_path: None,
                    status: "deleted".to_string(),
                    staged: index_status != ' ' && index_status != '?',
                    unstaged: worktree_status != ' ' || index_status == '?',
                });
            }
            continue;
        }
        out.push(GitChange {
            path,
            old_path,
            status: porcelain_status(index_status, worktree_status).to_string(),
            staged: index_status != ' ' && index_status != '?',
            unstaged: worktree_status != ' ' || index_status == '?',
        });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path).then_with(|| a.status.cmp(&b.status)));
    out
}

fn git_name_status(root: &Path, args: &[&str]) -> Vec<GitChange> {
    let output = Command::new("git")
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

fn changed_paths(changes: Vec<GitChange>) -> Vec<String> {
    changes
        .into_iter()
        .map(|change| change.path)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn visible_or_degraded_change(change: GitChange) -> Option<GitChange> {
    if !should_ignore_rel(&change.path) {
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

fn normalize_status_path(path: &str, root_prefix: Option<&str>) -> Option<String> {
    let rel = normalize_rel_path(path);
    if let Some(prefix) = root_prefix {
        return rel.strip_prefix(prefix).map(normalize_rel_path);
    }
    (!rel.is_empty()).then_some(rel)
}

fn porcelain_status(index_status: char, worktree_status: char) -> &'static str {
    if matches!((index_status, worktree_status), ('U', _) | (_, 'U') | ('A', 'A') | ('D', 'D')) {
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

fn git_status_root_prefix(root: &Path) -> Option<String> {
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
