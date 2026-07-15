// Responsibility: cli-run
use crate::cli::{
    AnchorAction, Cli, CommandKind, DEFAULT_PROOF_LIMIT, GraphOutputFormat, anchors_markdown,
    changed_inputs, changed_section_name, command_root_hint, cone_section_name, diff_map_inputs,
    ensure_graph_lens, ensure_valid_config, files_markdown, files_report, flow_anchor_arg,
    impact_inputs, init, ls_section_name, maybe_write_changed_lens_cache,
    maybe_write_cone_lens_cache, maybe_write_ls_lens_cache, maybe_write_place_lens_cache,
    maybe_write_proof_changed_lens_cache_from_changed, maybe_write_proof_map_lens_cache,
    maybe_write_proof_map_lens_cache_from_changed, maybe_write_siblings_lens_cache, output,
    output_format_with_json_alias, output_with_prelude, project_relative_arg, proof,
    proof_map_inputs, run_runtime, schema_text, try_cached_changed_fast_path,
    try_cached_cone_fast_path, try_cached_ls_fast_path, try_cached_place_fast_path,
    try_cached_proof_changed_fast_path, try_cached_proof_map_fast_path,
    try_cached_siblings_fast_path, try_clean_changed_fast_path, try_clean_proof_changed_fast_path,
    try_cold_root_graph_fast_path, try_cold_root_ls_fast_path, try_cold_root_proof_map_fast_path,
    try_runtime_root_fast_path, validate_anchors,
};
use crate::{map, render, repo};
use anyhow::Result;
use anyhow::bail;
use clap::Parser;
use std::collections::BTreeSet;
use std::env;

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    if let CommandKind::Bootstrap(args) = &cli.command {
        if args.global_instruction {
            print!("{}", render::global_instruction());
        } else {
            println!("Use `codemap bootstrap --global-instruction`.");
        }
        return Ok(());
    }
    if let CommandKind::Schema(args) = &cli.command {
        print!("{}", schema_text(args.kind));
        return Ok(());
    }

    let ambient_root = env::current_dir()
        .ok()
        .and_then(|cwd| repo::ambient_root(&cwd));
    let root_selection = if let Some(root) = cli.root.clone() {
        repo::RootSelection::Exact(root)
    } else if let Some(hint) = command_root_hint(&cli.command, ambient_root.as_deref()) {
        repo::RootSelection::Discover(hint)
    } else {
        repo::RootSelection::Auto
    };
    render::set_expand_root(cli.root.as_deref());
    render::set_brief(cli.brief || codemap_brief_env());
    if let Some(()) = try_cached_ls_fast_path(&cli.command, &root_selection)? {
        return Ok(());
    }
    if let Some(()) = try_cold_root_ls_fast_path(&cli.command, &root_selection)? {
        return Ok(());
    }
    if let Some(()) = try_cold_root_graph_fast_path(&cli.command, &root_selection)? {
        return Ok(());
    }
    if let Some(()) = try_cached_cone_fast_path(&cli.command, &root_selection)? {
        return Ok(());
    }
    if let Some(()) = try_clean_changed_fast_path(&cli.command, &root_selection)? {
        return Ok(());
    }
    if let Some(()) = try_clean_proof_changed_fast_path(&cli.command, &root_selection)? {
        return Ok(());
    }
    if let Some(()) = try_cached_changed_fast_path(&cli.command, &root_selection)? {
        return Ok(());
    }
    if let Some(()) = try_cached_proof_changed_fast_path(&cli.command, &root_selection)? {
        return Ok(());
    }
    if let Some(()) = try_cached_proof_map_fast_path(&cli.command, &root_selection)? {
        return Ok(());
    }
    if let Some(()) = try_cold_root_proof_map_fast_path(&cli.command, &root_selection)? {
        return Ok(());
    }
    if let Some(()) = try_cached_siblings_fast_path(&cli.command, &root_selection)? {
        return Ok(());
    }
    if let Some(()) = try_cached_place_fast_path(&cli.command, &root_selection)? {
        return Ok(());
    }
    if let Some(()) = try_runtime_root_fast_path(&cli.command, &root_selection)? {
        return Ok(());
    }

    let cache_write = match &cli.command {
        CommandKind::Doctor(_) | CommandKind::Status(_) | CommandKind::Teach(_) => {
            repo::CacheWriteMode::ReadOnly
        }
        _ => repo::CacheWriteMode::Enabled,
    };
    let project = repo::load_project_with_cache(root_selection, cache_write)?;
    render::set_map_snapshot(&project);
    match cli.command {
        CommandKind::Doctor(args) => {
            let report = map::status_report(&project, crate::cli::identity_diagnostics());
            output(args.format, &report, || render::status(&report, true))
        }
        CommandKind::Status(args) => {
            let report = map::status_report(&project, crate::cli::identity_diagnostics());
            output(args.format, &report, || render::status(&report, false))
        }
        CommandKind::Teach(args) => {
            let report = map::teach_report(&project);
            output(args.format, &report, || render::teach(&report))
        }
        CommandKind::Files(args) => {
            let report = files_report(&project, args.path.as_deref(), args.limit)?;
            output(args.format, &report, || files_markdown(&report))
        }
        CommandKind::Ls(args) => {
            ensure_valid_config(&project)?;
            let path = project_relative_arg(&project, &args.path)?;
            accept_depth_compat(args.depth, "ls")?;
            let limit = if args.include_hidden {
                usize::MAX / 2
            } else {
                args.limit
            };
            let report = map::ls_report(&project, &path, args.include_hidden, limit);
            maybe_write_ls_lens_cache(&project, &path, &args, &report);
            let prelude = repo::map_prelude(&project.root);
            output_with_prelude(
                output_format_with_json_alias(args.format, args.json),
                &report,
                &prelude,
                || render::ls(&report, ls_section_name(args.section)),
            )
        }
        CommandKind::Cone(args) => {
            ensure_valid_config(&project)?;
            let path = project_relative_arg(&project, &args.path)?;
            let format = output_format_with_json_alias(args.format, args.json);
            let include_hidden = args.include_hidden || format == crate::cli::OutputFormat::Json;
            let report = map::cone_report(&project, &path, args.depth, include_hidden, args.limit);
            maybe_write_cone_lens_cache(&project, &path, &args, include_hidden, &report);
            let prelude = repo::map_prelude(&project.root);
            output_with_prelude(format, &report, &prelude, || {
                render::cone(&report, cone_section_name(args.section))
            })
        }
        CommandKind::Where(args) => {
            ensure_valid_config(&project)?;
            let format = output_format_with_json_alias(args.format, args.json);
            let include_hidden = args.include_hidden || format == crate::cli::OutputFormat::Json;
            let limit = if include_hidden {
                usize::MAX / 2
            } else {
                args.limit
            };
            let report = map::where_report(
                &project,
                &args.query,
                args.kind.as_deref(),
                include_hidden,
                limit,
            );
            if let Err(error) = report.validate_observations() {
                bail!("invalid where observation ledger: {error:?}");
            }
            output(format, &report, || render::where_locator(&report))
        }
        CommandKind::Init(args) => init(&project, args),
        CommandKind::Bootstrap(_) => Ok(()),
        CommandKind::Schema(_) => Ok(()),
        CommandKind::Impact(args) => {
            ensure_valid_config(&project)?;
            let (changed, selector) = impact_inputs(&project, &args)?;
            let report = map::impact_report(&project, changed, selector, args.depth, args.limit);
            output(args.format, &report, || render::impact(&report))
        }
        CommandKind::Changed(args) => {
            ensure_valid_config(&project)?;
            accept_depth_compat(args.depth, "changed")?;
            let (changed, selector, mode, git_state, since_notice) =
                changed_inputs(&project, &args)?;
            let limit = if args.include_hidden {
                usize::MAX / 2
            } else {
                args.limit
            };
            let format = output_format_with_json_alias(args.format, args.json);
            let section = changed_section_name(args.section);
            let section_report =
                section.filter(|section| !args.include_hidden && *section != "hidden");
            let mut report = if let Some(section) = section_report {
                map::changed_report_for_section(
                    &project, changed, selector, mode, git_state, limit, section,
                )
            } else {
                map::changed_report(
                    &project,
                    changed,
                    selector,
                    mode,
                    git_state,
                    limit,
                    DEFAULT_PROOF_LIMIT,
                )
            };
            if section_report.is_none() {
                maybe_write_changed_lens_cache(&project, &args, limit, &report);
            }
            maybe_write_proof_changed_lens_cache_from_changed(&project, &args, &report);
            maybe_write_proof_map_lens_cache_from_changed(&project, &args, limit, &report);
            // Inject the fail-open snapshot notice after cache writes so it never
            // pollutes the cached report.
            if let Some(notice) = since_notice {
                report.unknowns.push(notice);
            }
            let prelude = repo::map_prelude(&project.root);
            output_with_prelude(format, &report, &prelude, || {
                render::changed(&report, section)
            })
        }
        CommandKind::DiffMap(args) => {
            ensure_valid_config(&project)?;
            let (changed, selector, mode) = diff_map_inputs(&project, &args)?;
            let report = map::diff_map_report(&project, changed, selector, args.limit, mode);
            output(args.format, &report, || render::diff_map(&report))
        }
        CommandKind::Contract(args) => {
            ensure_valid_config(&project)?;
            let path = project_relative_arg(&project, &args.path)?;
            let report = map::contract_report(&project, &path, args.include_hidden, args.limit);
            output(args.format, &report, || render::contract(&report))
        }
        CommandKind::Runtime(args) => run_runtime(&project, args),
        CommandKind::Proof(args) => proof(&project, args),
        CommandKind::ProofMap(args) => {
            ensure_valid_config(&project)?;
            let (target, changed, proof_selector) = proof_map_inputs(&project, &args)?;
            let report = map::proof_map_report(
                &project,
                target.clone(),
                changed,
                proof_selector.clone(),
                args.limit,
                args.raw_sensors,
            );
            maybe_write_proof_map_lens_cache(
                &project,
                target.as_deref(),
                &proof_selector,
                &args,
                &report,
            );
            output(args.format, &report, || render::proof_map(&report))
        }
        CommandKind::Delete(args) => {
            ensure_valid_config(&project)?;
            let path = project_relative_arg(&project, &args.path)?;
            let report = map::delete_report(&project, &path, args.include_hidden, args.limit);
            output(args.format, &report, || render::delete(&report))
        }
        CommandKind::BoundaryMap(args) => {
            ensure_valid_config(&project)?;
            let scope = project_relative_arg(&project, &args.scope)?;
            let changed = if args.changed {
                Some(
                    repo::changed_files(&project.root, false, None)
                        .into_iter()
                        .collect::<BTreeSet<_>>(),
                )
            } else {
                None
            };
            let report = map::boundary_map_report(
                &project,
                &scope,
                changed.as_ref(),
                args.include_hidden,
                args.limit,
            );
            output(args.format, &report, || render::boundary_map(&report))
        }
        CommandKind::Flow(args) => {
            ensure_valid_config(&project)?;
            let path = flow_anchor_arg(&project, &args.path)?;
            let report = map::flow_report(&project, &path, args.include_hidden, args.limit);
            output(args.format, &report, || render::flow(&report))
        }
        CommandKind::Siblings(args) => {
            ensure_valid_config(&project)?;
            let scope = project_relative_arg(&project, &args.scope)?;
            let report = map::siblings_report(&project, &scope, args.include_hidden, args.limit);
            maybe_write_siblings_lens_cache(&project, &scope, &args, &report);
            output(args.format, &report, || render::siblings(&report))
        }
        CommandKind::Place(args) => {
            ensure_valid_config(&project)?;
            let scope = project_relative_arg(&project, &args.scope)?;
            let report = map::place_report(
                &project,
                &scope,
                &args.kind,
                args.include_hidden,
                args.limit,
            );
            maybe_write_place_lens_cache(&project, &scope, &args, &report);
            output(args.format, &report, || render::place(&report))
        }
        CommandKind::Graph(args) => {
            ensure_valid_config(&project)?;
            ensure_graph_lens(&args.lens)?;
            let changed = if args.changed {
                repo::changed_files(&project.root, false, None)
            } else {
                Vec::new()
            };
            let graph_path = args
                .path
                .as_deref()
                .map(|path| project_relative_arg(&project, path))
                .transpose()?;
            let graph = map::graph_lens(
                &project,
                graph_path.as_deref(),
                &args.lens,
                args.limit,
                args.changed.then_some(changed.as_slice()),
            );
            match args.format {
                GraphOutputFormat::Json => {
                    render::print_json(&graph, &crate::cli::build_identity(false))
                }
                GraphOutputFormat::Mermaid => {
                    render::graph_mermaid(&graph);
                    Ok(())
                }
                GraphOutputFormat::Markdown => {
                    render::graph_markdown(&graph);
                    Ok(())
                }
            }
        }
        CommandKind::Boundaries(args) => {
            ensure_valid_config(&project)?;
            let changed = if args.changed {
                Some(
                    repo::changed_files(&project.root, false, None)
                        .into_iter()
                        .collect::<BTreeSet<_>>(),
                )
            } else {
                None
            };
            let report = map::boundary_report(&project, changed.as_ref());
            let hard = report
                .findings
                .iter()
                .any(|f| f.status != "warn" && f.status != "warning");
            let warns = report
                .findings
                .iter()
                .any(|f| f.status == "warn" || f.status == "warning");
            output(args.format, &report, || {
                render::boundaries(&report.findings)
            })?;
            if hard || (args.strict_warnings && warns) {
                bail!("boundary findings detected");
            }
            Ok(())
        }
        CommandKind::Anchors(args) => match args.action {
            AnchorAction::Validate(format) => {
                let report = validate_anchors(&project);
                output(format.format, &report, || anchors_markdown(&report))
            }
        },
    }
}

fn codemap_brief_env() -> bool {
    env::var("CODEMAP_BRIEF").is_ok_and(|value| !value.is_empty() && value != "0")
}

pub(crate) fn accept_depth_compat(depth: usize, command: &str) -> Result<()> {
    if depth <= 1 {
        return Ok(());
    }
    bail!(
        "codemap {command} currently keeps depth fixed at 1; use `codemap cone <anchor> --depth {depth}` or `codemap proof <anchor|changed> --depth {depth}` for expanded neighborhoods"
    );
}
