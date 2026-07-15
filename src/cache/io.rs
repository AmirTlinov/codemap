// Responsibility: cache-atomic-io-and-failure-receipts
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CacheDiagnostic {
    pub unix_seconds: u64,
    pub operation: String,
    pub artifact: String,
    pub outcome: String,
    pub detail: String,
}

pub fn atomic_write(path: &Path, body: impl AsRef<[u8]>) -> Result<()> {
    inject_write_failure(path)?;
    let parent = path
        .parent()
        .with_context(|| format!("cache artifact has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("create cache directory {}", parent.display()))?;
    let temp = temporary_path(path);
    let result = (|| -> Result<()> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&temp)
            .with_context(|| format!("create temporary cache artifact {}", temp.display()))?;
        file.write_all(body.as_ref())
            .with_context(|| format!("write temporary cache artifact {}", temp.display()))?;
        file.sync_all()
            .with_context(|| format!("sync temporary cache artifact {}", temp.display()))?;
        fs::rename(&temp, path).with_context(|| {
            format!(
                "atomically publish cache artifact {} -> {}",
                temp.display(),
                path.display()
            )
        })?;
        sync_directory(parent);
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

pub fn write_cache_path(cache_dir: &Path, path: &Path, body: impl AsRef<[u8]>) -> Result<()> {
    let result = atomic_write(path, body);
    if let Err(error) = &result {
        let artifact = path
            .strip_prefix(cache_dir)
            .unwrap_or(path)
            .to_string_lossy();
        record_event(
            cache_dir,
            "write",
            &artifact,
            "failed",
            &format!("{error:#}"),
        );
    }
    result
}

pub fn record_event(
    cache_dir: &Path,
    operation: &str,
    artifact: &str,
    outcome: &str,
    detail: &str,
) {
    let event = CacheDiagnostic {
        unix_seconds: now_unix_seconds(),
        operation: operation.to_string(),
        artifact: artifact.to_string(),
        outcome: outcome.to_string(),
        detail: detail.to_string(),
    };
    let Ok(body) = serde_json::to_string_pretty(&event) else {
        return;
    };
    let name = format!(
        "{}-{}-{}-{}.json",
        event.unix_seconds,
        std::process::id(),
        now_unix_nanos(),
        stable_name(artifact)
    );
    let _ = atomic_write(&cache_dir.join("events").join(name), format!("{body}\n"));
}

pub fn diagnostics(cache_dir: &Path) -> Vec<CacheDiagnostic> {
    let Ok(entries) = fs::read_dir(cache_dir.join("events")) else {
        return Vec::new();
    };
    let mut events = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| fs::read_to_string(entry.path()).ok())
        .filter_map(|body| serde_json::from_str(&body).ok())
        .collect::<Vec<_>>();
    events.sort_by(|left: &CacheDiagnostic, right| {
        left.unix_seconds
            .cmp(&right.unix_seconds)
            .then_with(|| left.artifact.cmp(&right.artifact))
    });
    let keep_from = events.len().saturating_sub(32);
    events.split_off(keep_from)
}

pub fn quarantine_artifact(cache_dir: &Path, path: &Path, reason: &str) -> Option<PathBuf> {
    if !path.starts_with(cache_dir) || !path.exists() {
        return None;
    }
    let name = path.file_name()?.to_string_lossy();
    let quarantine_dir = cache_dir.join("quarantine").join(format!(
        "{}-{}-{}-{}",
        now_unix_seconds(),
        std::process::id(),
        now_unix_nanos(),
        stable_name(&name)
    ));
    if fs::create_dir_all(&quarantine_dir).is_err() {
        record_event(cache_dir, "quarantine", &name, "failed", reason);
        return None;
    }
    let destination = quarantine_dir.join(name.as_ref());
    if fs::rename(path, &destination).is_err() {
        record_event(cache_dir, "quarantine", &name, "failed", reason);
        return None;
    }
    let receipt = CacheDiagnostic {
        unix_seconds: now_unix_seconds(),
        operation: "quarantine".to_string(),
        artifact: name.to_string(),
        outcome: "moved".to_string(),
        detail: reason.to_string(),
    };
    if let Ok(body) = serde_json::to_string_pretty(&receipt) {
        let _ = atomic_write(&quarantine_dir.join("receipt.json"), format!("{body}\n"));
    }
    record_event(cache_dir, "quarantine", &name, "moved", reason);
    Some(destination)
}

fn inject_write_failure(path: &Path) -> Result<()> {
    let Ok(selector) = std::env::var("CODEMAP_TEST_CACHE_WRITE_FAILURE") else {
        return Ok(());
    };
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if selector == "*" || selector == file_name {
        bail!("injected cache write failure for {}", path.display());
    }
    Ok(())
}

fn temporary_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("cache");
    path.with_file_name(format!(
        ".{name}.tmp-{}-{}",
        std::process::id(),
        now_unix_nanos()
    ))
}

fn stable_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect()
}

fn sync_directory(path: &Path) {
    if let Ok(directory) = fs::File::open(path) {
        let _ = directory.sync_all();
    }
}

fn now_unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn now_unix_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}
