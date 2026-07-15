// Responsibility: repo-scripts-make
use crate::model::{FileInfo, ScriptInfo};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub(crate) fn makefile_scripts(root: &Path, files: &BTreeMap<String, FileInfo>) -> Vec<ScriptInfo> {
    ["GNUmakefile", "makefile", "Makefile"]
        .iter()
        .find(|name| files.contains_key(**name))
        .and_then(|name| {
            if !indexed_readable(files, name) {
                return None;
            }
            let path = root.join(name);
            let text = fs::read_to_string(&path).ok()?;
            Some(make_like_scripts_from_text(
                &text,
                "make",
                "Makefile target",
                name,
            ))
        })
        .unwrap_or_default()
}

pub(crate) fn justfile_scripts(root: &Path, files: &BTreeMap<String, FileInfo>) -> Vec<ScriptInfo> {
    ["justfile", "Justfile"]
        .iter()
        .find(|name| files.contains_key(**name))
        .and_then(|name| {
            if !indexed_readable(files, name) {
                return None;
            }
            let path = root.join(name);
            let text = fs::read_to_string(&path).ok()?;
            Some(make_like_scripts_from_text(
                &text,
                "just",
                "justfile target",
                name,
            ))
        })
        .unwrap_or_default()
}

fn indexed_readable(files: &BTreeMap<String, FileInfo>, path: &str) -> bool {
    files
        .get(path)
        .is_some_and(|file| file.content_hash.is_some())
}

fn make_like_scripts_from_text(
    text: &str,
    runner: &str,
    reason: &str,
    path: &str,
) -> Vec<ScriptInfo> {
    let mut scripts = Vec::new();
    let mut in_make_define = false;
    for (index, line) in text.lines().enumerate() {
        if runner == "make" {
            let trimmed = line.trim();
            if in_make_define {
                if trimmed == "endef" {
                    in_make_define = false;
                }
                continue;
            }
            if trimmed == "define" || trimmed.starts_with("define ") {
                in_make_define = true;
                continue;
            }
        }
        let Some(targets) = make_like_targets(line) else {
            continue;
        };
        for target in targets {
            scripts.push(ScriptInfo {
                name: target.clone(),
                command: format!("{runner} {}", shell_quote_script_target(&target)),
                reason: format!("{reason}: {target}"),
                path: Some(path.to_string()),
                line_start: Some(index + 1),
            });
        }
    }
    scripts
}

fn make_like_targets(line: &str) -> Option<Vec<String>> {
    if line.starts_with(char::is_whitespace) {
        return None;
    }
    let code = line.split('#').next()?.trim();
    if code.is_empty() || code.starts_with('.') || code.contains(":=") || code.contains("?=") {
        return None;
    }
    let (left, _) = code.split_once(':')?;
    if left.contains('=') {
        return None;
    }
    let targets = left
        .split_whitespace()
        .filter(|target| {
            !target.is_empty()
                && !target.contains('%')
                && !target.starts_with('.')
                && target.chars().all(|ch| {
                    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':' | '/')
                })
        })
        .map(str::to_string)
        .collect::<Vec<_>>();
    (!targets.is_empty()).then_some(targets)
}

fn shell_quote_script_target(target: &str) -> String {
    if target
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':' | '/'))
    {
        target.to_string()
    } else {
        format!("'{}'", target.replace('\'', "'\\''"))
    }
}
