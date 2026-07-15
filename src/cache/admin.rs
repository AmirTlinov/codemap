// Responsibility: external-cache-diagnostic-and-explicit-maintenance
use anyhow::{Result, bail};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

const QUARANTINE_RETENTION_DAYS: u64 = 7;
const EVENT_RETENTION_COUNT: usize = 32;

#[derive(Clone, Copy, Debug)]
pub enum CacheAdminAction {
    Status,
    Gc,
    Clear,
}

#[derive(Debug, Serialize)]
pub struct CacheAdminReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub action: &'static str,
    pub root: String,
    pub cache_dir: String,
    pub outside_repository: bool,
    pub exists: bool,
    pub files: usize,
    pub bytes: u64,
    pub snapshots: usize,
    pub quarantine_receipts: usize,
    pub diagnostic_events: Vec<super::io::CacheDiagnostic>,
    pub private_file_permissions: bool,
    pub removed_files: usize,
    pub removed_bytes: u64,
    pub contents: Vec<&'static str>,
    pub retention: Vec<&'static str>,
    pub privacy: Vec<&'static str>,
}

impl CacheAdminReport {
    pub const SCHEMA_VERSION: &'static str = "1";
}

pub fn run(root: &Path, cache_dir: &Path, action: CacheAdminAction) -> Result<CacheAdminReport> {
    let outside_repository =
        !canonical_for_compare(cache_dir).starts_with(canonical_for_compare(root));
    if !outside_repository && !matches!(action, CacheAdminAction::Status) {
        bail!(
            "refusing cache mutation inside repository: {}",
            cache_dir.display()
        );
    }
    let (removed_files, removed_bytes) = match action {
        CacheAdminAction::Status => (0, 0),
        CacheAdminAction::Gc => gc(cache_dir),
        CacheAdminAction::Clear => clear(cache_dir)?,
    };
    let measure = measure(cache_dir);
    let action_name = match action {
        CacheAdminAction::Status => "status",
        CacheAdminAction::Gc => "gc",
        CacheAdminAction::Clear => "clear",
    };
    Ok(CacheAdminReport {
        kind: "cache_report",
        schema_version: CacheAdminReport::SCHEMA_VERSION,
        action: action_name,
        root: root.to_string_lossy().to_string(),
        cache_dir: cache_dir.to_string_lossy().to_string(),
        outside_repository,
        exists: cache_dir.exists(),
        files: measure.files,
        bytes: measure.bytes,
        snapshots: count_json(cache_dir.join("snapshots")),
        quarantine_receipts: count_named(cache_dir.join("quarantine"), "receipt.json"),
        diagnostic_events: super::io::diagnostics(cache_dir),
        private_file_permissions: measure.private_permissions,
        removed_files,
        removed_bytes,
        contents: vec![
            "per-file extracted structural facts and file fingerprints",
            "reverse-import index and derived bounded lens reports",
            "up to 32 snapshot manifests and content blobs for --since",
            "quarantine receipts and cache failure diagnostics",
        ],
        retention: vec![
            "snapshot manifests: newest 32 per repository",
            "quarantine: 7 days until explicit cache gc",
            "diagnostic events: newest 32 per repository",
            "project cache: retained until cache clear or external OS cleanup",
        ],
        privacy: vec![
            "cache is external by default and never synchronized by codemap",
            "artifact files are created with owner-only permissions on Unix",
            "snapshot blobs may contain text from indexed repository files",
            "cache status reads metadata only; clear and gc require explicit verbs",
        ],
    })
}

fn gc(cache_dir: &Path) -> (usize, u64) {
    let mut removed = Removal::default();
    remove_older_than(
        &cache_dir.join("quarantine"),
        days(QUARANTINE_RETENTION_DAYS),
        &mut removed,
    );
    prune_newest(
        &cache_dir.join("events"),
        EVENT_RETENTION_COUNT,
        &mut removed,
    );
    remove_temporary_files(cache_dir, &mut removed);
    quarantine_invalid_top_level_json(cache_dir);
    (removed.files, removed.bytes)
}

fn clear(cache_dir: &Path) -> Result<(usize, u64)> {
    let before = measure(cache_dir);
    if cache_dir.exists() {
        fs::remove_dir_all(cache_dir)?;
    }
    write_clear_receipt(cache_dir, before.files, before.bytes);
    Ok((before.files, before.bytes))
}

fn write_clear_receipt(cache_dir: &Path, files: usize, bytes: u64) {
    let Some(base) = cache_dir.parent() else {
        return;
    };
    let receipt = serde_json::json!({
        "unix_seconds": now_unix_seconds(),
        "operation": "clear",
        "cache_dir": cache_dir,
        "removed_files": files,
        "removed_bytes": bytes,
    });
    let path = base.join("receipts").join(format!(
        "{}-clear-{}.json",
        now_unix_seconds(),
        std::process::id()
    ));
    if let Ok(body) = serde_json::to_string_pretty(&receipt) {
        let _ = super::io::atomic_write(&path, format!("{body}\n"));
    }
}

fn quarantine_invalid_top_level_json(cache_dir: &Path) {
    let Ok(entries) = fs::read_dir(cache_dir) else {
        return;
    };
    for entry in entries.filter_map(|entry| entry.ok()) {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let valid = fs::read_to_string(&path)
            .ok()
            .and_then(|body| serde_json::from_str::<serde_json::Value>(&body).ok())
            .is_some();
        if !valid {
            let _ = super::io::quarantine_artifact(cache_dir, &path, "cache gc: invalid json");
        }
    }
}

#[derive(Default)]
struct Measure {
    files: usize,
    bytes: u64,
    private_permissions: bool,
}

fn measure(root: &Path) -> Measure {
    let mut result = Measure {
        private_permissions: true,
        ..Measure::default()
    };
    walk_files(root, &mut |path, metadata| {
        result.files += 1;
        result.bytes += metadata.len();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                result.private_permissions = false;
            }
        }
        let _ = path;
    });
    result
}

fn walk_files(root: &Path, visit: &mut impl FnMut(&Path, &fs::Metadata)) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.filter_map(|entry| entry.ok()) {
        let path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.is_dir() {
            walk_files(&path, visit);
        } else if metadata.is_file() {
            visit(&path, &metadata);
        }
    }
}

#[derive(Default)]
struct Removal {
    files: usize,
    bytes: u64,
}

fn remove_older_than(root: &Path, age: std::time::Duration, removed: &mut Removal) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.filter_map(|entry| entry.ok()) {
        let path = entry.path();
        let old = entry
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|elapsed| elapsed >= age);
        if old {
            remove_path(&path, removed);
        }
    }
}

fn prune_newest(root: &Path, keep: usize, removed: &mut Removal) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let mut paths = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| Some((entry.path(), entry.metadata().ok()?.modified().ok()?)))
        .collect::<Vec<_>>();
    paths.sort_by_key(|(_, modified)| *modified);
    let count = paths.len().saturating_sub(keep);
    for (path, _) in paths.into_iter().take(count) {
        remove_path(&path, removed);
    }
}

fn remove_temporary_files(root: &Path, removed: &mut Removal) {
    let mut candidates = Vec::<PathBuf>::new();
    walk_files(root, &mut |path, _| {
        let temporary = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.contains(".tmp-"));
        if temporary {
            candidates.push(path.to_path_buf());
        }
    });
    for path in candidates {
        remove_path(&path, removed);
    }
}

fn remove_path(path: &Path, removed: &mut Removal) {
    let (before_files, before_bytes) = if path.is_dir() {
        let before = measure(path);
        (before.files, before.bytes)
    } else {
        (
            usize::from(path.is_file()),
            fs::metadata(path)
                .map(|metadata| metadata.len())
                .unwrap_or(0),
        )
    };
    let result = if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    };
    if result.is_ok() {
        removed.files += before_files;
        removed.bytes += before_bytes;
    }
}

fn count_json(root: PathBuf) -> usize {
    fs::read_dir(root)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
        .count()
}

fn count_named(root: PathBuf, name: &str) -> usize {
    let mut count = 0;
    walk_files(&root, &mut |path, _| {
        if path.file_name().and_then(|value| value.to_str()) == Some(name) {
            count += 1;
        }
    });
    count
}

fn days(value: u64) -> std::time::Duration {
    std::time::Duration::from_secs(value * 24 * 60 * 60)
}

fn now_unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn canonical_for_compare(path: &Path) -> PathBuf {
    if let Ok(canonical) = path.canonicalize() {
        return canonical;
    }
    let mut missing = Vec::new();
    let mut ancestor = path;
    while !ancestor.exists() {
        let Some(name) = ancestor.file_name() else {
            return path.to_path_buf();
        };
        missing.push(name.to_os_string());
        let Some(parent) = ancestor.parent() else {
            return path.to_path_buf();
        };
        ancestor = parent;
    }
    let mut canonical = ancestor
        .canonicalize()
        .unwrap_or_else(|_| ancestor.to_path_buf());
    for name in missing.into_iter().rev() {
        canonical.push(name);
    }
    canonical
}
