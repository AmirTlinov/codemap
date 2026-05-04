pub fn map_prelude(root: &Path) -> crate::model::MapPrelude {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let cwd = env::current_dir().unwrap_or_else(|_| root.clone());
    let cwd_rel = cwd
        .strip_prefix(&root)
        .ok()
        .map(|path| normalize_rel_path(&path.to_string_lossy()))
        .map(|path| if path.is_empty() { ".".to_string() } else { path });
    let git_root = git_root(&root);
    let Some(git_root_path) = git_root else {
        return crate::model::MapPrelude {
            root: root.to_string_lossy().to_string(),
            cwd: cwd.to_string_lossy().to_string(),
            cwd_rel,
            git_root: None,
            vcs: None,
            head: crate::model::HeadPrelude::default(),
            branch: crate::model::BranchPrelude::default(),
            remote: crate::model::RemotePrelude::default(),
            worktree: crate::model::WorktreePrelude {
                clean: true,
                ..Default::default()
            },
            freshness: crate::model::FreshnessPrelude {
                network_used: false,
                remote_refs: "not_applicable".to_string(),
                remote_currentness: "unknown".to_string(),
                fetch_head_mtime: None,
                evidence: vec!["git rev-parse --show-toplevel failed".to_string()],
            },
            unknowns: Vec::new(),
        };
    };

    let mut prelude = crate::model::MapPrelude {
        root: root.to_string_lossy().to_string(),
        cwd: cwd.to_string_lossy().to_string(),
        cwd_rel,
        git_root: Some(git_root_path.to_string_lossy().to_string()),
        vcs: Some("git".to_string()),
        head: crate::model::HeadPrelude::default(),
        branch: crate::model::BranchPrelude::default(),
        remote: remote_prelude(&root),
        worktree: crate::model::WorktreePrelude::default(),
        freshness: crate::model::FreshnessPrelude {
            network_used: false,
            remote_refs: "local_only".to_string(),
            remote_currentness: "unknown".to_string(),
            fetch_head_mtime: git_fetch_head_mtime(&root),
            evidence: vec![
                "git status --porcelain=v2 --branch -z -uall".to_string(),
                "git remote get-url origin".to_string(),
            ],
        },
        unknowns: Vec::new(),
    };

    if !parse_git_status_v2(&root, &mut prelude) {
        prelude.unknowns.push(crate::model::PreludeUnknown {
            kind: "git_status_unavailable".to_string(),
            effect: "worktree, branch, head, and ahead/behind are unknown from local git status"
                .to_string(),
        });
    }
    if prelude.branch.upstream.is_none() {
        prelude.unknowns.push(crate::model::PreludeUnknown {
            kind: "upstream_not_configured".to_string(),
            effect: "ahead/behind cannot be computed from local tracking refs".to_string(),
        });
    } else if prelude.branch.ahead.is_none() || prelude.branch.behind.is_none() {
        prelude.unknowns.push(crate::model::PreludeUnknown {
            kind: "ahead_behind_not_reported".to_string(),
            effect: "ahead/behind were not reported by local git status".to_string(),
        });
    }
    prelude.branch.upstream_gone = prelude
        .branch
        .upstream
        .as_deref()
        .is_some_and(|_| git_upstream_gone(&root));
    prelude.worktree.clean = prelude.worktree.staged == 0
        && prelude.worktree.unstaged == 0
        && prelude.worktree.untracked == 0
        && prelude.worktree.conflicted == 0;
    prelude
}

fn parse_git_status_v2(root: &Path, prelude: &mut crate::model::MapPrelude) -> bool {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain=v2", "--branch", "-z", "-uall"])
        .output();
    let Ok(output) = output else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let mut entries = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .peekable();
    while let Some(entry) = entries.next() {
        let record = String::from_utf8_lossy(entry);
        if let Some(header) = record.strip_prefix("# ") {
            parse_status_header(header, prelude);
            continue;
        }
        if record.starts_with("2 ") {
            count_status_record(&record, &mut prelude.worktree);
            let _ = entries.next();
            continue;
        }
        count_status_record(&record, &mut prelude.worktree);
    }
    true
}

fn parse_status_header(header: &str, prelude: &mut crate::model::MapPrelude) {
    if let Some(oid) = header.strip_prefix("branch.oid ") {
        if oid == "(initial)" {
            prelude.head.unborn = true;
            prelude.head.oid = None;
            prelude.head.short = None;
        } else {
            prelude.head.oid = Some(oid.to_string());
            prelude.head.short = Some(oid.chars().take(12).collect());
        }
    } else if let Some(head) = header.strip_prefix("branch.head ") {
        if head == "(detached)" {
            prelude.head.detached = true;
            prelude.branch.name = None;
        } else {
            prelude.branch.name = Some(head.to_string());
        }
    } else if let Some(upstream) = header.strip_prefix("branch.upstream ") {
        prelude.branch.upstream = Some(upstream.to_string());
    } else if let Some(ab) = header.strip_prefix("branch.ab ") {
        let mut parts = ab.split_whitespace();
        prelude.branch.ahead = parse_ab_count(parts.next(), '+');
        prelude.branch.behind = parse_ab_count(parts.next(), '-');
    }
}

fn parse_ab_count(value: Option<&str>, prefix: char) -> Option<u32> {
    let value = value?;
    value.strip_prefix(prefix)?.parse().ok()
}

fn count_status_record(record: &str, worktree: &mut crate::model::WorktreePrelude) {
    if let Some(path) = record.strip_prefix("? ") {
        if !should_ignore_rel(&normalize_rel_path(path)) {
            worktree.untracked += 1;
        }
        return;
    }
    if record.starts_with("! ") {
        return;
    }
    let mut parts = record.split_whitespace();
    let kind = parts.next().unwrap_or_default();
    let xy = parts.next().unwrap_or_default();
    if kind == "u" {
        worktree.conflicted += 1;
        return;
    }
    let mut chars = xy.chars();
    let index = chars.next().unwrap_or('.');
    let tree = chars.next().unwrap_or('.');
    if matches!((index, tree), ('U', _) | (_, 'U') | ('A', 'A') | ('D', 'D')) {
        worktree.conflicted += 1;
    }
    if index != '.' && index != '?' {
        worktree.staged += 1;
    }
    if tree != '.' {
        worktree.unstaged += 1;
    }
    if index == 'R' {
        worktree.renamed += 1;
    }
    if index == 'D' || tree == 'D' {
        worktree.deleted += 1;
    }
    if index == 'T' || tree == 'T' {
        worktree.typechanged += 1;
    }
}

fn remote_prelude(root: &Path) -> crate::model::RemotePrelude {
    let Some(url) = git_remote(root) else {
        return crate::model::RemotePrelude::default();
    };
    let sanitized_url = sanitize_remote_url(&url);
    crate::model::RemotePrelude {
        name: Some("origin".to_string()),
        display: remote_display(&sanitized_url),
        sanitized_url: Some(sanitized_url),
    }
}

fn sanitize_remote_url(url: &str) -> String {
    if let Some((scheme, rest)) = url.split_once("://") {
        let slash_index = rest.find('/').unwrap_or(rest.len());
        let (authority, path) = rest.split_at(slash_index);
        if let Some((_, host)) = authority.rsplit_once('@') {
            return format!("{scheme}://{host}{path}");
        }
    }
    url.to_string()
}

fn remote_display(sanitized_url: &str) -> Option<String> {
    let mut value = sanitized_url.trim().to_string();
    if let Some(rest) = value.strip_prefix("git@")
        && let Some((host, path)) = rest.split_once(':')
    {
        value = format!("{host}/{path}");
    } else if let Some((_, rest)) = value.split_once("://") {
        value = rest.to_string();
    }
    if let Some((_, rest)) = value.rsplit_once('@') {
        value = rest.to_string();
    }
    if let Some(stripped) = value.strip_suffix(".git") {
        value = stripped.to_string();
    }
    (!value.is_empty()).then_some(value)
}

fn git_upstream_gone(root: &Path) -> bool {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--verify", "--quiet", "@{upstream}"])
        .output();
    !output.is_ok_and(|output| output.status.success())
}

fn git_fetch_head_mtime(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--absolute-git-dir"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let git_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let mtime = fs::metadata(Path::new(&git_dir).join("FETCH_HEAD"))
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(format!("unix:{mtime}"))
}
