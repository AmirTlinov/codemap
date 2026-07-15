// Responsibility: repo-prelude
use crate::repo::{git_remote, git_root, git_status_snapshot, normalize_rel_path};
use std::env;
use std::fs;
use std::path::Path;

pub fn map_prelude(root: &Path) -> crate::model::MapPrelude {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let cwd = env::current_dir().unwrap_or_else(|_| root.clone());
    let cwd_rel = cwd
        .strip_prefix(&root)
        .ok()
        .map(|path| normalize_rel_path(&path.to_string_lossy()))
        .map(|path| {
            if path.is_empty() {
                ".".to_string()
            } else {
                path
            }
        });
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

    if let Some(snapshot) = git_status_snapshot(&root) {
        prelude.head = snapshot.head;
        prelude.branch = snapshot.branch;
        prelude.worktree = snapshot.worktree;
    } else {
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
    let output = crate::repo::read_only_git_command()
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--verify", "--quiet", "@{upstream}"])
        .output();
    !output.is_ok_and(|output| output.status.success())
}

fn git_fetch_head_mtime(root: &Path) -> Option<String> {
    let output = crate::repo::read_only_git_command()
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
