// Responsibility: cache-admin-command-dispatch
use crate::cli::{CacheAction, CommandKind, output};
use crate::{cache, render, repo};
use anyhow::{Result, bail};

pub(crate) fn try_cache_admin(
    command: &CommandKind,
    root_selection: &repo::RootSelection,
) -> Result<Option<()>> {
    let CommandKind::Cache(args) = command else {
        return Ok(None);
    };
    let cwd = std::env::current_dir()?;
    let root = repo::resolve_root(root_selection, &cwd)?;
    let remote = repo::git_remote(&root);
    let cache_dir = cache::project_cache_dir(&root, remote.as_deref(), repo::VERSION);
    let (action, format) = match &args.action {
        CacheAction::Status(args) => (cache::CacheAdminAction::Status, args.format),
        CacheAction::Gc(args) => (cache::CacheAdminAction::Gc, args.format),
        CacheAction::Clear(args) => {
            if !args.yes {
                bail!("cache clear requires --yes");
            }
            (cache::CacheAdminAction::Clear, args.format)
        }
    };
    let report = cache::run_cache_admin(&root, &cache_dir, action)?;
    output(format, &report, || render::cache_admin(&report))?;
    Ok(Some(()))
}
