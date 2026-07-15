use std::collections::BTreeSet;
use std::path::Path;

pub(crate) fn current_git_head(root: &Path) -> Option<String> {
    let output = crate::repo::read_only_git_command()
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

pub(crate) fn current_git_status_has_untracked(root: &Path) -> bool {
    let output = crate::repo::read_only_git_command()
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain", "-uall", "--", "."])
        .output();
    let Ok(output) = output else {
        return true;
    };
    if !output.status.success() {
        return true;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|line| line.starts_with("?? "))
}

pub(crate) fn git_tracked_paths(root: &Path) -> Option<BTreeSet<String>> {
    let output = crate::repo::read_only_git_command()
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-c"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                let rel = line.trim();
                (!rel.is_empty()).then(|| rel.replace('\\', "/"))
            })
            .collect(),
    )
}

pub(crate) fn git_path_is_ignored(root: &Path, rel: &str) -> bool {
    let output = crate::repo::read_only_git_command()
        .arg("-C")
        .arg(root)
        .args(["check-ignore", "-q", "--", rel])
        .output();
    output.is_ok_and(|output| output.status.success())
}
