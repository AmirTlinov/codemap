pub fn changed_files(root: &Path, staged: bool, since: Option<&str>) -> Vec<String> {
    if let Some(since) = since
        && let Some(files) = git_name_only(root, &["diff", "--name-only", "--relative", since])
    {
        return files;
    }
    if staged {
        return git_name_only(root, &["diff", "--name-only", "--relative", "--cached"])
            .unwrap_or_default();
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain", "-uall", "--", "."])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let root_prefix = git_status_root_prefix(root);
    let mut files = BTreeSet::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if line.len() < 4 {
            continue;
        }
        let mut path = line[3..].trim().to_string();
        if let Some((_, new_path)) = path.split_once(" -> ") {
            path = new_path.to_string();
        }
        let rel = normalize_rel_path(&path);
        let rel = if let Some(prefix) = root_prefix.as_deref() {
            let Some(stripped) = rel.strip_prefix(prefix) else {
                continue;
            };
            normalize_rel_path(stripped)
        } else {
            rel
        };
        if !rel.is_empty() && !should_ignore_rel(&rel) {
            files.insert(rel);
        }
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
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain", "-uall", "--", "."])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let root_prefix = git_status_root_prefix(root);
    let mut out = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if line.len() < 4 {
            continue;
        }
        let index_status = line.chars().next().unwrap_or(' ');
        let worktree_status = line.chars().nth(1).unwrap_or(' ');
        let raw = line[3..].trim();
        let (old_path, path) = if let Some((old_path, new_path)) = raw.split_once(" -> ") {
            (Some(old_path.to_string()), new_path.to_string())
        } else {
            (None, raw.to_string())
        };
        let Some(path) = normalize_status_path(&path, root_prefix.as_deref()) else {
            continue;
        };
        if should_ignore_rel(&path) {
            continue;
        }
        let old_path = old_path
            .as_deref()
            .and_then(|old| normalize_status_path(old, root_prefix.as_deref()));
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
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(name_status_line)
        .filter(|change| !should_ignore_rel(&change.path))
        .collect()
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

fn git_name_only(root: &Path, args: &[&str]) -> Option<Vec<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(normalize_rel_path)
            .filter(|rel| !rel.is_empty() && !should_ignore_rel(rel))
            .collect(),
    )
}
