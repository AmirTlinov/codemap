// Responsibility: cli-fast-paths-root-graph
use crate::cli::{CommandKind, GraphOutputFormat, ensure_graph_lens, root_relative_arg};
use crate::{map, render, repo};
use anyhow::Result;
use std::env;

const COLD_ROOT_GRAPH_FILE_THRESHOLD: usize = 800;

pub(crate) fn try_cold_root_graph_fast_path(
    command: &CommandKind,
    root_selection: &repo::RootSelection,
) -> Result<Option<()>> {
    let CommandKind::Graph(args) = command else {
        return Ok(None);
    };
    ensure_graph_lens(&args.lens)?;
    if args.changed || !args.lens.eq_ignore_ascii_case("causal") {
        return Ok(None);
    }

    let cwd = env::current_dir()?;
    let root = repo::resolve_root(root_selection, &cwd)?;
    let graph_path = args
        .path
        .as_deref()
        .map(|path| root_relative_arg(&root, path))
        .transpose()?;
    if graph_path.as_deref().unwrap_or(".") != "." {
        return Ok(None);
    }

    let files = repo::structural_inventory_candidate_files(&root);
    if files.len() < COLD_ROOT_GRAPH_FILE_THRESHOLD {
        return Ok(None);
    }

    let graph = map::root_inventory_graph_lens(&root, &files, args.limit, &args.lens);
    match args.format {
        GraphOutputFormat::Json => render::print_json(&graph, &crate::cli::build_identity(false)),
        GraphOutputFormat::Mermaid => {
            render::graph_mermaid(&graph);
            Ok(())
        }
        GraphOutputFormat::Markdown => {
            render::graph_markdown(&graph);
            Ok(())
        }
    }?;
    Ok(Some(()))
}
