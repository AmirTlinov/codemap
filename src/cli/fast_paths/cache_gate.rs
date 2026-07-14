// Responsibility: cli-fast-paths-cache-gate
use crate::cli::git_change_sets;
use crate::repo;
use std::path::Path;

// A snapshot-token `--since` must skip the git-status fast paths: they would treat
// the token as an empty git diff and report "clean". Fall through to the full path
// so snapshot_delta runs.
pub(crate) fn since_is_snapshot_token(since: Option<&str>) -> bool {
    since.is_some_and(crate::cache::looks_like_snapshot_token)
}

pub(crate) fn lens_cache_matches_current(
    root: &Path,
    cache_dir: &Path,
    git_state: &[crate::model::GitChange],
) -> bool {
    let (changed_or_added, removed) =
        repo::git_status_cache_change_sets(root).unwrap_or_else(|| git_change_sets(git_state));
    crate::cache::file_delta_for_known_changes(
        root,
        cache_dir,
        repo::VERSION,
        &changed_or_added,
        &removed,
    )
    .is_some_and(|delta| delta.is_exact_hit())
}
