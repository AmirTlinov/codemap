// Responsibility: cli-fast-paths-runtime-root
use crate::cli::{CommandKind, lens_cache_matches_current, output, set_cached_map_snapshot};
use crate::{render, repo};
use anyhow::Result;
use std::env;

pub(crate) fn try_runtime_root_fast_path(
    command: &CommandKind,
    root_selection: &repo::RootSelection,
) -> Result<Option<()>> {
    let CommandKind::Runtime(args) = command else {
        return Ok(None);
    };
    if args.scope != "." || args.include_hidden || args.limit != 20 {
        return Ok(None);
    }
    let cwd = env::current_dir()?;
    let root = repo::resolve_root(root_selection, &cwd)?;
    let remote = repo::git_remote(&root);
    let cache_dir = crate::cache::project_cache_dir(&root, remote.as_deref(), repo::VERSION);
    let git_state = repo::git_changes(&root, false, None);
    if !lens_cache_matches_current(&root, &cache_dir, &git_state) {
        return Ok(None);
    }
    set_cached_map_snapshot(&root, &cache_dir);
    let Some(report) = crate::cache::read_runtime_root_report(&cache_dir, repo::VERSION, &root)
    else {
        return Ok(None);
    };
    output(args.format, &report, || render::runtime(&report))?;
    Ok(Some(()))
}
