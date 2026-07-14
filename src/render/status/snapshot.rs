// Responsibility: map-snapshot-line
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

static MAP_SNAPSHOT: OnceLock<String> = OnceLock::new();
static BRIEF: OnceLock<bool> = OnceLock::new();

pub fn set_brief(value: bool) {
    let _ = BRIEF.set(value);
}

pub(crate) fn brief() -> bool {
    BRIEF.get().copied().unwrap_or(false)
}

pub fn set_map_snapshot(project: &crate::model::Project) {
    let fingerprint = crate::cache::fingerprint(project, None);
    set_map_snapshot_full(
        &project.root,
        Some(&fingerprint),
        Some(&project.cache_state),
        Some(&project.cache_strategy),
        Some(project.cache_dir.to_string_lossy().as_ref()),
    );
}

pub fn set_cached_map_snapshot_parts(root: &Path, fingerprint: Option<&str>, cache_dir: &Path) {
    set_map_snapshot_full(
        root,
        fingerprint,
        Some("hit"),
        Some("cached_lens"),
        Some(cache_dir.to_string_lossy().as_ref()),
    );
}

pub fn set_inventory_map_snapshot_parts(root: &Path, fingerprint: Option<&str>, cache_dir: &Path) {
    let cache_state = if !crate::cache::cache_enabled() {
        "disabled"
    } else if crate::cache::cached_status_fingerprint(cache_dir).is_some() {
        "stale"
    } else {
        "cold"
    };
    set_map_snapshot_full(
        root,
        fingerprint,
        Some(cache_state),
        Some("inventory_fast_path"),
        Some(cache_dir.to_string_lossy().as_ref()),
    );
}

// Cache telemetry (state/strategy/external_cache/location) is debug-only: agents
// never need the cache path or fingerprint provenance. It is opt-in on the snapshot
// line via CODEMAP_CACHE_TELEMETRY=1 and otherwise stays only in the `doctor` and
// `status` tables.
fn cache_telemetry_enabled() -> bool {
    std::env::var("CODEMAP_CACHE_TELEMETRY").is_ok_and(|value| !value.is_empty() && value != "0")
}

// The snapshot token is the full per-file fingerprint; it doubles as the
// `--since` delta token (see cache::snapshots).
fn set_map_snapshot_full(
    root: &Path,
    fingerprint: Option<&str>,
    cache_state: Option<&str>,
    cache_strategy: Option<&str>,
    cache_location: Option<&str>,
) {
    let fingerprint = fingerprint.unwrap_or("unknown");
    let snapshot = fingerprint.get(..16).unwrap_or(fingerprint);
    let line = if brief() {
        format!("Map Snapshot: snapshot=`{snapshot}`; schema=`structural:5`; repo_footprint=`zero`")
    } else {
        let head = map_snapshot_git_head(root).unwrap_or_else(|| "none".to_string());
        let branch = map_snapshot_git_branch(root).unwrap_or_else(|| "unknown".to_string());
        let dirty = map_snapshot_dirty_count(root).unwrap_or(0);
        if cache_telemetry_enabled() {
            let short_fingerprint = fingerprint.get(..12).unwrap_or(fingerprint);
            let external_cache = if crate::cache::cache_enabled() {
                "enabled"
            } else {
                "disabled"
            };
            format!(
                "Map Snapshot: root=`{}`; head=`{}`; branch=`{}`; dirty=`{}`; snapshot=`{}`; fingerprint=`{}`; cache=`{}` strategy=`{}` external_cache=`{}` location=`{}`; schema=`structural:5`; repo_footprint=`zero`",
                root.to_string_lossy(),
                head,
                branch,
                dirty,
                snapshot,
                short_fingerprint,
                cache_state.unwrap_or("unknown"),
                cache_strategy.unwrap_or("unknown"),
                external_cache,
                cache_location.unwrap_or("unknown"),
            )
        } else {
            format!(
                "Map Snapshot: root=`{}`; head=`{}`; branch=`{}`; dirty=`{}`; snapshot=`{}`; schema=`structural:5`; repo_footprint=`zero`",
                root.to_string_lossy(),
                head,
                branch,
                dirty,
                snapshot
            )
        }
    };
    let _ = MAP_SNAPSHOT.set(line);
}

fn map_snapshot_git_head(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--short=12", "--verify", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let head = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!head.is_empty()).then_some(head)
}

fn map_snapshot_git_branch(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!branch.is_empty()).then_some(branch)
}

fn map_snapshot_dirty_count(root: &Path) -> Option<usize> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).lines().count())
}

pub(crate) fn map_snapshot_line() {
    if let Some(snapshot) = MAP_SNAPSHOT.get() {
        println!("{snapshot}");
    }
}
