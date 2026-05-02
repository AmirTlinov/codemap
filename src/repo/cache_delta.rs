fn git_status_cache_delta(
    root: &Path,
    cache_dir: &Path,
    version: &str,
) -> Option<cache::fingerprints::CacheFileDelta> {
    if cache::cached_git_head_matches(root, cache_dir, version) != Some(true) {
        return None;
    }
    let (changed_or_added, removed) = git_status_cache_change_sets(root)?;
    cache::file_delta_for_known_changes(root, cache_dir, version, &changed_or_added, &removed)
}

fn git_head_cache_delta(
    root: &Path,
    cache_dir: &Path,
    version: &str,
) -> Option<cache::fingerprints::CacheFileDelta> {
    let cached_head = cache::cached_git_head(root, cache_dir, version)?;
    let current_head = current_git_head(root)?;
    if cached_head == current_head {
        return None;
    }
    let (mut changed_or_added, mut removed) =
        git_head_cache_change_sets(root, &cached_head, &current_head)?;
    if let Some((status_changed_or_added, status_removed)) = git_status_cache_change_sets(root) {
        changed_or_added.extend(status_changed_or_added);
        removed.extend(status_removed);
    }
    cache::file_delta_for_head_change(root, cache_dir, version, &changed_or_added, &removed)
}

fn cached_index_cache_delta(
    root: &Path,
    cache_dir: &Path,
    version: &str,
) -> Option<cache::fingerprints::CacheFileDelta> {
    let empty = BTreeSet::new();
    let status_changes = git_status_cache_change_sets(root);
    let (changed_or_added, removed) = status_changes
        .as_ref()
        .map(|(changed_or_added, removed)| (changed_or_added, removed))
        .unwrap_or((&empty, &empty));
    cache::file_delta_by_rechecking_cached_files(
        root,
        cache_dir,
        version,
        changed_or_added,
        removed,
    )
}

pub(crate) fn git_status_cache_change_sets(
    root: &Path,
) -> Option<(BTreeSet<String>, BTreeSet<String>)> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain=v1", "-z", "-uall", "--", "."])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let root_prefix = git_status_root_prefix(root);
    let mut changed_or_added = BTreeSet::new();
    let mut removed = BTreeSet::new();
    let mut entries = output.stdout.split(|byte| *byte == 0).filter(|entry| !entry.is_empty());
    while let Some(entry) = entries.next() {
        if entry.len() < 4 {
            continue;
        }
        let record = String::from_utf8_lossy(entry);
        let index_status = record.chars().next().unwrap_or(' ');
        let worktree_status = record.chars().nth(1).unwrap_or(' ');
        let status = porcelain_status(index_status, worktree_status);
        if status == "conflicted" {
            return None;
        }
        let raw = &record[3..];
        let path = normalize_status_path(raw, root_prefix.as_deref());
        let old_path = if index_status == 'R' || index_status == 'C' {
            entries.next().and_then(|entry| {
                normalize_status_path(&String::from_utf8_lossy(entry), root_prefix.as_deref())
            })
        } else {
            None
        };
        if index_status == 'R'
            && let Some(old_path) = old_path
            && !should_ignore_rel(&old_path)
        {
            removed.insert(old_path);
        }
        let Some(path) = path else {
            continue;
        };
        if status == "deleted" {
            if !should_ignore_rel(&path) {
                removed.insert(path);
            }
        } else if should_ignore_rel(&path) {
            continue;
        } else if is_cache_candidate_file(root, &path) {
            changed_or_added.insert(path);
        } else {
            removed.insert(path);
        }
    }
    Some((changed_or_added, removed))
}

fn git_head_cache_change_sets(
    root: &Path,
    cached_head: &str,
    current_head: &str,
) -> Option<(BTreeSet<String>, BTreeSet<String>)> {
    let root_prefix = git_status_root_prefix(root);
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "diff",
            "--relative",
            "--name-status",
            "-z",
            cached_head,
            current_head,
            "--",
            ".",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let mut changed_or_added = BTreeSet::new();
    let mut removed = BTreeSet::new();
    let mut entries = output.stdout.split(|byte| *byte == 0).filter(|entry| !entry.is_empty());
    while let Some(status_entry) = entries.next() {
        let status = String::from_utf8_lossy(status_entry);
        let Some(status_kind) = status.chars().next() else {
            continue;
        };
        match status_kind {
            'D' => {
                if let Some(path) = entries.next().and_then(|entry| diff_path(entry, root_prefix.as_deref())) {
                    record_removed_candidate(&mut removed, path);
                }
            }
            'R' => {
                if let Some(old_path) =
                    entries.next().and_then(|entry| diff_path(entry, root_prefix.as_deref()))
                {
                    record_removed_candidate(&mut removed, old_path);
                }
                if let Some(new_path) =
                    entries.next().and_then(|entry| diff_path(entry, root_prefix.as_deref()))
                {
                    record_changed_candidate(root, &mut changed_or_added, &mut removed, new_path);
                }
            }
            'C' => {
                let _ = entries.next();
                if let Some(new_path) =
                    entries.next().and_then(|entry| diff_path(entry, root_prefix.as_deref()))
                {
                    record_changed_candidate(root, &mut changed_or_added, &mut removed, new_path);
                }
            }
            _ => {
                if let Some(path) = entries.next().and_then(|entry| diff_path(entry, root_prefix.as_deref())) {
                    record_changed_candidate(root, &mut changed_or_added, &mut removed, path);
                }
            }
        }
    }
    Some((changed_or_added, removed))
}

fn diff_path(bytes: &[u8], root_prefix: Option<&str>) -> Option<String> {
    let rel = normalize_rel_path(&String::from_utf8_lossy(bytes));
    if let Some(prefix) = root_prefix
        && let Some(stripped) = rel.strip_prefix(prefix)
    {
        return Some(normalize_rel_path(stripped));
    }
    (!rel.is_empty()).then_some(rel)
}

fn record_removed_candidate(removed: &mut BTreeSet<String>, path: String) {
    if !should_ignore_rel(&path) {
        removed.insert(path);
    }
}

fn record_changed_candidate(
    root: &Path,
    changed_or_added: &mut BTreeSet<String>,
    removed: &mut BTreeSet<String>,
    path: String,
) {
    if should_ignore_rel(&path) {
        return;
    }
    if is_cache_candidate_file(root, &path) {
        changed_or_added.insert(path);
    } else {
        removed.insert(path);
    }
}

fn current_git_head(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--verify", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let head = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!head.is_empty()).then_some(head)
}
