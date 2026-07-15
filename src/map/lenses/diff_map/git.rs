// Responsibility: diff-map-lens-git
mod line_delta;

use crate::map::{DiffMapMode, diff_worktree_blob_text};
use crate::model::Project;
use crate::repo;
use line_delta::text_line_delta;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

#[derive(Clone, Default)]
pub(crate) struct LineDelta {
    pub(crate) added: Vec<(usize, String)>,
    pub(crate) removed: Vec<(usize, String)>,
}

pub(crate) fn git_unified_zero_deltas(
    project: &Project,
    rels: &[String],
    mode: &DiffMapMode,
) -> BTreeMap<String, LineDelta> {
    if let DiffMapMode::Snapshot(snapshot) = mode {
        return rels
            .iter()
            .map(|rel| {
                let base = snapshot.texts.get(rel).map(String::as_str);
                let current = diff_worktree_blob_text(project, rel);
                (rel.clone(), text_line_delta(base, current.as_deref()))
            })
            .collect();
    }
    let mut deltas = BTreeMap::new();
    let mut tracked = Vec::new();
    let untracked = if matches!(mode, DiffMapMode::WorkingTree) {
        git_untracked_files(project, rels)
    } else {
        BTreeSet::new()
    };
    for rel in rels {
        if untracked.contains(rel) {
            deltas.insert(rel.clone(), file_as_added_delta(project, rel));
        } else {
            tracked.push(rel.clone());
        }
    }
    if tracked.is_empty() {
        return deltas;
    }
    let mut command = crate::repo::read_only_git_command();
    command.arg("-C").arg(&project.root).arg("diff");
    match mode {
        DiffMapMode::WorkingTree => {
            command.arg("HEAD");
        }
        DiffMapMode::Staged => {
            command.arg("--cached");
        }
        DiffMapMode::Since(base) => {
            command.arg(base);
        }
        DiffMapMode::Snapshot(_) => unreachable!("snapshot returned above"),
    }
    let Ok(output) = command.args(["--unified=0", "--"]).args(&tracked).output() else {
        return deltas;
    };
    if !output.status.success() {
        return deltas;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut current = None;
    let mut old_line = 0usize;
    let mut new_line = 0usize;
    for line in text.lines() {
        if let Some(rel) = parse_diff_file_line(line, &tracked) {
            current = Some(rel.clone());
            deltas.entry(rel).or_default();
            continue;
        }
        let Some(rel) = current.as_ref() else {
            continue;
        };
        if let Some((old_start, new_start)) = parse_diff_hunk_header(line) {
            old_line = old_start;
            new_line = new_start;
            continue;
        }
        if line.starts_with("+++") || line.starts_with("---") {
            continue;
        }
        if let Some(body) = line.strip_prefix('+') {
            deltas
                .entry(rel.clone())
                .or_default()
                .added
                .push((new_line.max(1), body.to_string()));
            new_line += 1;
        } else if let Some(body) = line.strip_prefix('-') {
            deltas
                .entry(rel.clone())
                .or_default()
                .removed
                .push((old_line.max(1), body.to_string()));
            old_line += 1;
        } else if !line.starts_with("diff ") && !line.starts_with("index ") {
            old_line += 1;
            new_line += 1;
        }
    }
    for rel in tracked {
        deltas
            .entry(rel.clone())
            .or_insert_with(|| git_unified_zero_delta(project, &rel, mode));
    }
    deltas
}

fn git_unified_zero_delta(project: &Project, rel: &str, mode: &DiffMapMode) -> LineDelta {
    if matches!(mode, DiffMapMode::WorkingTree) && git_file_is_untracked(project, rel) {
        return file_as_added_delta(project, rel);
    }
    let mut command = crate::repo::read_only_git_command();
    command.arg("-C").arg(&project.root).arg("diff");
    match mode {
        DiffMapMode::WorkingTree => {
            command.arg("HEAD");
        }
        DiffMapMode::Staged => {
            command.arg("--cached");
        }
        DiffMapMode::Since(base) => {
            command.arg(base);
        }
        DiffMapMode::Snapshot(_) => unreachable!("snapshot handled by batch path"),
    }
    let Ok(output) = command.args(["--unified=0", "--"]).arg(rel).output() else {
        return LineDelta::default();
    };
    if !output.status.success() {
        return LineDelta::default();
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut delta = LineDelta::default();
    let mut old_line = 0usize;
    let mut new_line = 0usize;
    for line in text.lines() {
        if let Some((old_start, new_start)) = parse_diff_hunk_header(line) {
            old_line = old_start;
            new_line = new_start;
            continue;
        }
        if line.starts_with("+++") || line.starts_with("---") {
            continue;
        }
        if let Some(body) = line.strip_prefix('+') {
            delta.added.push((new_line.max(1), body.to_string()));
            new_line += 1;
        } else if let Some(body) = line.strip_prefix('-') {
            delta.removed.push((old_line.max(1), body.to_string()));
            old_line += 1;
        } else if !line.starts_with("diff ") && !line.starts_with("index ") {
            old_line += 1;
            new_line += 1;
        }
    }
    delta
}

fn parse_diff_file_line(line: &str, tracked: &[String]) -> Option<String> {
    if let Some(rest) = line.strip_prefix("+++ b/") {
        let rel = repo::normalize_rel_path(rest);
        if tracked.iter().any(|tracked| tracked == &rel) {
            return Some(rel);
        }
    }
    if let Some((_, right)) = line.strip_prefix("diff --git ")?.rsplit_once(" b/") {
        let rel = repo::normalize_rel_path(right);
        if tracked.iter().any(|tracked| tracked == &rel) {
            return Some(rel);
        }
    }
    None
}

pub(crate) fn git_show_files(
    project: &Project,
    revision: &str,
    rels: &[String],
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut requests = Vec::new();
    for rel in rels {
        if rel.contains('\n') {
            if let Some(text) = git_show_file(project, revision, rel) {
                out.insert(rel.clone(), text);
            }
            continue;
        }
        let object = if revision == ":" {
            format!(":{rel}")
        } else {
            format!("{revision}:{rel}")
        };
        requests.push((rel.clone(), object));
    }
    if requests.is_empty() {
        return out;
    }
    let Ok(mut child) = crate::repo::read_only_git_command()
        .arg("-C")
        .arg(&project.root)
        .args(["cat-file", "--batch"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
    else {
        return git_show_files_individually(project, revision, requests, out);
    };
    if let Some(mut stdin) = child.stdin.take() {
        for (_, object) in &requests {
            let _ = std::io::Write::write_all(&mut stdin, object.as_bytes());
            let _ = std::io::Write::write_all(&mut stdin, b"\n");
        }
    }
    let mut stdout = Vec::new();
    if let Some(mut pipe) = child.stdout.take() {
        let _ = std::io::Read::read_to_end(&mut pipe, &mut stdout);
    }
    let Ok(status) = child.wait() else {
        return git_show_files_individually(project, revision, requests, out);
    };
    if !status.success() {
        return git_show_files_individually(project, revision, requests, out);
    }
    parse_git_cat_file_batch(&stdout, &requests, &mut out);
    out
}

fn git_show_files_individually(
    project: &Project,
    revision: &str,
    requests: Vec<(String, String)>,
    mut out: BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    for (rel, _) in requests {
        if let Some(text) = git_show_file(project, revision, &rel) {
            out.insert(rel, text);
        }
    }
    out
}

fn parse_git_cat_file_batch(
    stdout: &[u8],
    requests: &[(String, String)],
    out: &mut BTreeMap<String, String>,
) {
    let mut index = 0usize;
    for (rel, _) in requests {
        let Some(header_end) = stdout[index..].iter().position(|byte| *byte == b'\n') else {
            return;
        };
        let header_end = index + header_end;
        let header = String::from_utf8_lossy(&stdout[index..header_end]);
        index = header_end + 1;
        if header.ends_with(" missing") {
            continue;
        }
        let Some(size) = header
            .rsplit_once(' ')
            .and_then(|(_, size)| size.parse::<usize>().ok())
        else {
            return;
        };
        if index + size > stdout.len() {
            return;
        }
        let content = &stdout[index..index + size];
        index += size;
        if stdout.get(index) == Some(&b'\n') {
            index += 1;
        }
        out.insert(rel.clone(), String::from_utf8_lossy(content).to_string());
    }
}

fn git_show_file(project: &Project, revision: &str, rel: &str) -> Option<String> {
    let object = if revision == ":" {
        format!(":{rel}")
    } else {
        format!("{revision}:{rel}")
    };
    let exists = crate::repo::read_only_git_command()
        .arg("-C")
        .arg(&project.root)
        .arg("cat-file")
        .arg("-e")
        .arg(&object)
        .output()
        .ok()?;
    if !exists.status.success() {
        return None;
    }
    let output = crate::repo::read_only_git_command()
        .arg("-C")
        .arg(&project.root)
        .arg("show")
        .arg(object)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).to_string())
}

fn git_file_is_untracked(project: &Project, rel: &str) -> bool {
    if !working_tree_file_or_symlink(project, rel) {
        return false;
    }
    let Ok(output) = crate::repo::read_only_git_command()
        .arg("-C")
        .arg(&project.root)
        .args(["ls-files", "--error-unmatch", "--"])
        .arg(rel)
        .output()
    else {
        return false;
    };
    !output.status.success()
}

fn git_untracked_files(project: &Project, rels: &[String]) -> BTreeSet<String> {
    let candidates = rels
        .iter()
        .filter(|rel| working_tree_file_or_symlink(project, rel))
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return BTreeSet::new();
    }
    let Ok(output) = crate::repo::read_only_git_command()
        .arg("-C")
        .arg(&project.root)
        .args(["ls-files", "-z", "--others", "--exclude-standard", "--"])
        .args(candidates)
        .output()
    else {
        return BTreeSet::new();
    };
    if !output.status.success() {
        return BTreeSet::new();
    }
    output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|raw| !raw.is_empty())
        .map(|raw| repo::normalize_rel_path(&String::from_utf8_lossy(raw)))
        .collect()
}

fn file_as_added_delta(project: &Project, rel: &str) -> LineDelta {
    let Some(text) = diff_worktree_blob_text(project, rel) else {
        return LineDelta::default();
    };
    LineDelta {
        added: text
            .lines()
            .enumerate()
            .map(|(index, line)| (index + 1, line.to_string()))
            .collect(),
        removed: Vec::new(),
    }
}

fn working_tree_file_or_symlink(project: &Project, rel: &str) -> bool {
    std::fs::symlink_metadata(project.root.join(rel))
        .ok()
        .is_some_and(|metadata| metadata.is_file() || metadata.file_type().is_symlink())
}

fn parse_diff_hunk_header(line: &str) -> Option<(usize, usize)> {
    if !line.starts_with("@@ ") {
        return None;
    }
    let mut parts = line.split_whitespace();
    parts.next()?;
    let old = parts.next()?.trim_start_matches('-');
    let new = parts.next()?.trim_start_matches('+');
    Some((
        old.split(',').next()?.parse().ok()?,
        new.split(',').next()?.parse().ok()?,
    ))
}
