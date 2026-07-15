// Responsibility: cli-fast-paths-root-ls
use crate::cli::{
    CommandKind, accept_depth_compat, ls_section_name, output_format_with_json_alias,
    output_with_prelude, root_relative_arg, set_inventory_map_snapshot_with_fingerprint,
};
use crate::{map, render, repo};
use anyhow::Result;
use std::env;

const ROOT_ATLAS_FAST_PATH_FILE_THRESHOLD: usize = 800;

pub(crate) fn try_cold_root_ls_fast_path(
    command: &CommandKind,
    root_selection: &repo::RootSelection,
) -> Result<Option<()>> {
    let CommandKind::Ls(args) = command else {
        return Ok(None);
    };
    accept_depth_compat(args.depth, "ls")?;
    let format = output_format_with_json_alias(args.format, args.json);
    if args.include_hidden {
        return Ok(None);
    }
    let include_hidden = format == crate::cli::OutputFormat::Json;
    let limit = if include_hidden {
        usize::MAX / 2
    } else {
        args.limit
    };

    let cwd = env::current_dir()?;
    let root = repo::resolve_root(root_selection, &cwd)?;
    let path = root_relative_arg(&root, &args.path)?;
    if path != "." {
        return Ok(None);
    }

    let files = repo::list_visible_candidate_files(&root);
    if crate::cli::root_inventory_has_codemap_config(&root, &files) {
        return Ok(None);
    }
    if files.len() < ROOT_ATLAS_FAST_PATH_FILE_THRESHOLD {
        return Ok(None);
    }

    let fingerprint = crate::cache::inventory_fingerprint(&root, &files);
    let remote = repo::git_remote(&root);
    let cache_dir = crate::cache::project_cache_dir(&root, remote.as_deref(), repo::VERSION);
    let lens_key = || crate::cache::LsLensKey {
        cache_dir: &cache_dir,
        version: repo::VERSION,
        root: &root,
        path: ".",
        include_hidden,
        limit,
        complete_file_projection: false,
        complete_directory_projection: false,
    };
    if crate::cache::cache_enabled()
        && let Some(report) = crate::cache::read_inventory_ls_report(lens_key(), &fingerprint)
    {
        render::set_cached_map_snapshot_parts(&root, Some(&fingerprint), &cache_dir);
        let prelude = repo::map_prelude(&root);
        output_with_prelude(format, &report, &prelude, || {
            render::ls(&report, ls_section_name(args.section))
        })?;
        return Ok(Some(()));
    }
    let report = map::root_inventory_ls_report(&root, &files, include_hidden, limit);
    set_inventory_map_snapshot_with_fingerprint(&root, &fingerprint);
    if crate::cache::cache_enabled() {
        let _ = crate::cache::write_inventory_ls_report(lens_key(), &report, &fingerprint);
    }
    let prelude = repo::map_prelude(&root);
    output_with_prelude(format, &report, &prelude, || {
        render::ls(&report, ls_section_name(args.section))
    })?;
    Ok(Some(()))
}
