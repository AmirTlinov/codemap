pub fn resolve_root(root_selection: &RootSelection, cwd: &Path) -> Result<PathBuf> {
    match root_selection {
        RootSelection::Exact(path) => {
            Ok(path.canonicalize().unwrap_or_else(|_| path.to_path_buf()))
        }
        RootSelection::Discover(path) => {
            let base = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            if let Some(git_root) = git_root(&base) {
                return Ok(git_root);
            }
            Ok(marker_root(&base).unwrap_or(base))
        }
        RootSelection::Auto => {
            if let Some(git_root) = git_root(cwd) {
                return Ok(git_root);
            }
            Ok(marker_root(cwd).unwrap_or_else(|| cwd.to_path_buf()))
        }
    }
}

pub fn ambient_root(start: &Path) -> Option<PathBuf> {
    git_root(start).or_else(|| marker_root(start))
}

fn git_root(start: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(start)
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if raw.is_empty() {
        None
    } else {
        Some(PathBuf::from(raw))
    }
}

pub fn git_remote(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .output()
        .ok()?;
    if output.status.success() {
        let remote = String::from_utf8_lossy(&output.stdout).trim().to_string();
        (!remote.is_empty()).then_some(remote)
    } else {
        None
    }
}

fn is_git_repo(root: &Path) -> bool {
    root.join(".git").exists() || git_root(root).is_some()
}

fn marker_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(path) = current {
        if ROOT_MARKERS.iter().any(|marker| path.join(marker).exists()) {
            return Some(path.to_path_buf());
        }
        current = path.parent();
    }
    None
}

fn nearest_agents(cwd: &Path, root: &Path) -> Option<String> {
    let mut current = Some(cwd);
    while let Some(path) = current {
        let candidate = path.join("AGENTS.md");
        if candidate.exists() {
            return candidate
                .strip_prefix(root)
                .ok()
                .map(|p| normalize_rel_path(&p.to_string_lossy()));
        }
        if path == root {
            break;
        }
        current = path.parent();
    }
    None
}
