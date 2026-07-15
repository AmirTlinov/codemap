// Responsibility: cache-artifact-status
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use serde_json::Value;

use super::{cache_enabled, expected_artifacts, lens_artifacts};
use crate::model::{CacheArtifactStatus, Project};

pub fn stale_lens_artifact_examples(
    cache_dir: &Path,
    version: &str,
    root: &Path,
    fingerprint: &str,
) -> Vec<String> {
    lens_artifacts::artifact_names()
        .iter()
        .filter_map(|name| {
            let path = cache_dir.join(name);
            if !path.exists() {
                return None;
            }
            stale_lens_artifact_reason(&path, version, root, fingerprint).map(|reason| {
                let _ = super::io::quarantine_artifact(cache_dir, &path, &reason);
                format!("{name} ({reason})")
            })
        })
        .collect()
}

fn stale_lens_artifact_reason(
    path: &Path,
    version: &str,
    root: &Path,
    fingerprint: &str,
) -> Option<String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => return Some("unreadable json".to_string()),
    };
    let value: Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(_) => return Some("invalid json".to_string()),
    };
    match value.get("format_version").and_then(Value::as_u64) {
        Some(found) if found == lens_artifacts::format_version() => {}
        Some(found) => {
            return Some(format!(
                "format {found} != {}",
                lens_artifacts::format_version()
            ));
        }
        None => return Some("format missing".to_string()),
    }
    match value.get("version").and_then(Value::as_str) {
        Some(found) if found == version => {}
        Some(found) => return Some(format!("version {found} != {version}")),
        None => return Some("version missing".to_string()),
    }
    let expected_root = root.to_string_lossy();
    match value.get("root").and_then(Value::as_str) {
        Some(found) if found == expected_root.as_ref() => {}
        Some(_) => return Some("root mismatch".to_string()),
        None => return Some("root missing".to_string()),
    }
    match value.get("fingerprint").and_then(Value::as_str) {
        Some(found) if found == fingerprint => None,
        Some(_) => None,
        None => Some("fingerprint missing".to_string()),
    }
}

pub fn artifact_statuses(project: &Project, fingerprint: &str) -> Vec<CacheArtifactStatus> {
    expected_artifacts()
        .iter()
        .map(|name| {
            let path = project.cache_dir.join(name);
            let mut meta = fs::metadata(&path).ok();
            let mut fingerprint_match = if meta.is_some() {
                cached_fingerprint(&path).map(|cached| cached == fingerprint)
            } else {
                None
            };
            if meta.is_some() && fingerprint_match.is_none() {
                let _ = super::io::quarantine_artifact(
                    &project.cache_dir,
                    &path,
                    "core cache artifact is unreadable or invalid",
                );
                meta = None;
                fingerprint_match = None;
            }
            CacheArtifactStatus {
                name: (*name).to_string(),
                path: path.to_string_lossy().to_string(),
                exists: meta.is_some(),
                bytes: meta.map(|m| m.len()),
                fingerprint_match,
            }
        })
        .collect()
}

pub fn cache_state(artifacts: &[CacheArtifactStatus]) -> String {
    if !cache_enabled() {
        return "disabled".to_string();
    }
    if artifacts.iter().all(|artifact| artifact.exists)
        && artifacts
            .iter()
            .all(|artifact| artifact.fingerprint_match == Some(true))
    {
        return "warm".to_string();
    }
    if artifacts.iter().any(|artifact| artifact.exists) {
        return "stale".to_string();
    }
    "cold".to_string()
}

fn cached_fingerprint(path: &Path) -> Option<String> {
    if let Some(fingerprint) = cached_fingerprint_from_header(path) {
        return Some(fingerprint);
    }
    let text = fs::read_to_string(path).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    value
        .get("fingerprint")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn cached_fingerprint_from_header(path: &Path) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    for line in BufReader::new(file).lines().take(64) {
        let line = line.ok()?;
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("\"fingerprint\"") else {
            continue;
        };
        let (_, value) = rest.split_once(':')?;
        let value = value.trim().trim_end_matches(',');
        return serde_json::from_str(value).ok();
    }
    None
}

pub fn cached_status_fingerprint(cache_dir: &Path) -> Option<String> {
    let path = cache_dir.join("status.json");
    let fingerprint = cached_fingerprint(&path);
    if path.exists() && fingerprint.is_none() {
        let _ = super::io::quarantine_artifact(
            cache_dir,
            &path,
            "status cache is unreadable or invalid",
        );
    }
    fingerprint
}
