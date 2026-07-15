// Responsibility: render-prelude
use crate::model::{BuildIdentity, MapPrelude};
use crate::render::{brief, map_snapshot_line, rewrite_expand_fields};
use serde::Serialize;
use std::sync::OnceLock;

static MAP_PRELUDE: OnceLock<MapPrelude> = OnceLock::new();
static BUILD_IDENTITY: OnceLock<BuildIdentity> = OnceLock::new();

pub fn set_map_prelude(prelude: MapPrelude, build_identity: BuildIdentity) {
    let _ = MAP_PRELUDE.set(prelude);
    let _ = BUILD_IDENTITY.set(build_identity);
}

pub(crate) fn current_map_prelude() -> Option<&'static MapPrelude> {
    MAP_PRELUDE.get()
}

pub(crate) fn map_prelude_line_or_snapshot_line() {
    if let Some(prelude) = MAP_PRELUDE.get() {
        println!("{}", compact_prelude_line(prelude));
        map_snapshot_line();
    } else {
        map_snapshot_line();
    }
}

pub(crate) fn map_prelude_block_or_snapshot_line() {
    if let Some(prelude) = MAP_PRELUDE.get() {
        if brief() {
            println!("{}", compact_prelude_line(prelude));
            map_snapshot_line();
            return;
        }
        println!("Repo:");
        println!("  root: `{}`", prelude.root);
        println!(
            "  cwd: `{}`{}",
            prelude.cwd_rel.as_deref().unwrap_or(&prelude.cwd),
            compact_build_identity()
        );
        if let Some(git_root) = &prelude.git_root {
            println!("  git_root: `{git_root}`");
        }
        if prelude.vcs.as_deref() != Some("git") {
            println!("  vcs: `none`");
            println!("  worktree: `not applicable`");
            return;
        }
        let branch = prelude_branch(prelude);
        println!("  branch: `{branch}`");
        println!(
            "  head: `{}`",
            prelude.head.short.as_deref().unwrap_or("none")
        );
        println!("  ahead/behind: `{}`", prelude_ahead_behind(prelude));
        println!("  worktree: `{}`", prelude_worktree(prelude));
        if let Some(remote) = prelude.remote.display.as_deref() {
            println!("  remote: `{remote}`");
        }
        println!("  remote_refs: `{}`", prelude_remote_refs(prelude));
        map_snapshot_line();
    } else {
        map_snapshot_line();
    }
}

fn compact_prelude_line(prelude: &MapPrelude) -> String {
    let build = compact_build_identity();
    if prelude.vcs.as_deref() != Some("git") {
        return format!(
            "Repo: root=`{}` cwd=`{}` vcs=`none` worktree=`not applicable`{build}",
            prelude.root,
            prelude.cwd_rel.as_deref().unwrap_or(&prelude.cwd)
        );
    }
    let remote = prelude
        .remote
        .display
        .as_deref()
        .map(|remote| format!(" remote=`{remote}`"))
        .unwrap_or_default();
    format!(
        "Repo: root=`{}` cwd=`{}` branch=`{}` head=`{}` ahead/behind=`{}` worktree=`{}`{} remote_refs=`{}`{build}",
        prelude.root,
        prelude.cwd_rel.as_deref().unwrap_or(&prelude.cwd),
        prelude_branch(prelude),
        prelude.head.short.as_deref().unwrap_or("none"),
        prelude_ahead_behind(prelude),
        prelude_worktree(prelude),
        remote,
        prelude_remote_refs(prelude)
    )
}

fn compact_build_identity() -> String {
    BUILD_IDENTITY
        .get()
        .map(|identity| {
            let source = identity
                .source_commit
                .as_deref()
                .map(|commit| commit.get(..12).unwrap_or(commit))
                .unwrap_or("unknown");
            let dirty = identity
                .dirty_build
                .map(|dirty| if dirty { "dirty" } else { "clean" })
                .unwrap_or("unknown");
            format!(
                " codemap=`{}` executable=`{}` source=`{};{}`",
                identity.semver, identity.executable_path, source, dirty
            )
        })
        .unwrap_or_default()
}

fn prelude_branch(prelude: &MapPrelude) -> String {
    if prelude.head.detached {
        return "detached".to_string();
    }
    let name = prelude.branch.name.as_deref().unwrap_or("unknown");
    if let Some(upstream) = prelude.branch.upstream.as_deref() {
        format!("{name} -> {upstream}")
    } else {
        name.to_string()
    }
}

fn prelude_ahead_behind(prelude: &MapPrelude) -> String {
    match (prelude.branch.ahead, prelude.branch.behind) {
        (Some(ahead), Some(behind)) => format!("{ahead}/{behind}"),
        _ => "unknown".to_string(),
    }
}

fn prelude_worktree(prelude: &MapPrelude) -> String {
    if prelude.worktree.clean {
        "clean; staged=0 unstaged=0 untracked=0 conflicts=0".to_string()
    } else {
        format!(
            "dirty; staged={} unstaged={} untracked={} conflicts={}",
            prelude.worktree.staged,
            prelude.worktree.unstaged,
            prelude.worktree.untracked,
            prelude.worktree.conflicted
        )
    }
}

fn prelude_remote_refs(prelude: &MapPrelude) -> String {
    if prelude.freshness.network_used {
        format!(
            "{}; currentness {}; network used",
            prelude.freshness.remote_refs, prelude.freshness.remote_currentness
        )
    } else {
        format!(
            "{}; currentness {}; no network used",
            prelude.freshness.remote_refs, prelude.freshness.remote_currentness
        )
    }
}

pub fn print_json_with_prelude<T: Serialize>(
    value: &T,
    prelude: &MapPrelude,
    build_identity: &BuildIdentity,
) -> anyhow::Result<()> {
    let mut value = serde_json::to_value(value)?;
    rewrite_expand_fields(&mut value);
    if let serde_json::Value::Object(map) = &mut value {
        map.insert("prelude".to_string(), serde_json::to_value(prelude)?);
        map.insert(
            "build_identity".to_string(),
            serde_json::to_value(build_identity)?,
        );
    }
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
