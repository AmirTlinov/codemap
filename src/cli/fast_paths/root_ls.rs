// Responsibility: cli-fast-paths-root-ls
use crate::cli::{
    CommandKind, accept_depth_compat, ls_section_name, output_format_with_json_alias,
    output_with_prelude, root_relative_arg, set_inventory_map_snapshot_with_fingerprint,
};
use crate::{map, render, repo};
use anyhow::Result;
use std::env;

const COLD_ROOT_LS_FILE_THRESHOLD: usize = 800;

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

    let files = repo::structural_inventory_candidate_files(&root);
    if files.len() < COLD_ROOT_LS_FILE_THRESHOLD {
        return Ok(None);
    }

    let fingerprint = crate::cache::inventory_fingerprint(&root, &files);
    let report = map::root_inventory_ls_report(&root, &files, include_hidden, limit);
    set_inventory_map_snapshot_with_fingerprint(&root, &fingerprint);
    let prelude = repo::map_prelude(&root);
    output_with_prelude(format, &report, &prelude, || {
        render::ls(&report, ls_section_name(args.section))
    })?;
    Ok(Some(()))
}
