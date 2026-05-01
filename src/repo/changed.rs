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

