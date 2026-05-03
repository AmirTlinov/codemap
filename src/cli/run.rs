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
    if let Some(()) = try_cached_ls_fast_path(&cli.command, &root_selection)? {
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
    match cli.command {
        CommandKind::Doctor(args) => {
            let report = map::status_report(&project);
            output(args.format, &report, || render::status(&report, true))
        }
        CommandKind::Status(args) => {
            let report = map::status_report(&project);
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
            let report = map::ls_report(&project, &path, args.include_hidden, args.limit);
            maybe_write_ls_lens_cache(&project, &path, &args, &report);
            output(args.format, &report, || {
                render::ls(&report, ls_section_name(args.section))
            })
        }
        CommandKind::Cone(args) => {
            ensure_valid_config(&project)?;
            let path = project_relative_arg(&project, &args.path)?;
            let report =
                map::cone_report(&project, &path, args.depth, args.include_hidden, args.limit);
            maybe_write_cone_lens_cache(&project, &path, &args, &report);
            output(args.format, &report, || {
                render::cone(&report, cone_section_name(args.section))
            })
        }
        CommandKind::Init(args) => init(&project, args),
        CommandKind::Bootstrap(_) => Ok(()),
        CommandKind::Schema(_) => Ok(()),
        CommandKind::Impact(args) => {
            ensure_valid_config(&project)?;
            let changed = changed_from_args(&project, &args)?;
            let report = map::impact_report(&project, changed, args.depth, args.limit);
            output(args.format, &report, || render::impact(&report))
        }
        CommandKind::Changed(args) => {
            ensure_valid_config(&project)?;
            let (changed, selector, mode, git_state) = changed_inputs(&project, &args)?;
            let limit = if args.include_hidden {
                usize::MAX / 2
            } else {
                args.limit
            };
            let report = map::changed_report(
                &project,
                changed,
                selector,
                mode,
                git_state,
                limit,
                DEFAULT_PROOF_LIMIT,
            );
            maybe_write_changed_lens_cache(&project, &args, limit, &report);
            maybe_write_proof_changed_lens_cache_from_changed(&project, &args, &report);
            output(args.format, &report, || {
                render::changed(&report, changed_section_name(args.section))
            })
        }
        CommandKind::DiffMap(args) => {
            ensure_valid_config(&project)?;
            let changed = changed_from_diff_map_args(&project, &args)?;
            let mode = if args.staged {
                map::DiffMapMode::Staged
            } else if let Some(since) = args.since.clone() {
                map::DiffMapMode::Since(since)
            } else {
                map::DiffMapMode::WorkingTree
            };
            let report = map::diff_map_report(&project, changed, args.limit, mode);
            output(args.format, &report, || render::diff_map(&report))
        }
        CommandKind::Contract(args) => {
            ensure_valid_config(&project)?;
            let path = project_relative_arg(&project, &args.path)?;
            let report = map::contract_report(&project, &path, args.include_hidden, args.limit);
            output(args.format, &report, || render::contract(&report))
        }
        CommandKind::Runtime(args) => {
            ensure_valid_config(&project)?;
            let scope = project_relative_arg(&project, &args.scope)?;
            let report = map::runtime_report(&project, &scope, args.include_hidden, args.limit);
            output(args.format, &report, || render::runtime(&report))
        }
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
                GraphOutputFormat::Json => render::print_json(&graph),
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
