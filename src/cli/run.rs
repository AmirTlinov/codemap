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

    let cache_write = match &cli.command {
        CommandKind::Doctor(_) | CommandKind::Status(_) => repo::CacheWriteMode::ReadOnly,
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
        CommandKind::Files(args) => {
            let report = files_report(&project, args.path.as_deref(), args.limit)?;
            output(args.format, &report, || files_markdown(&report))
        }
        CommandKind::Ls(args) => {
            ensure_valid_config(&project)?;
            let path = project_relative_arg(&project, &args.path)?;
            let report = map::ls_report(&project, &path, args.include_hidden, args.limit);
            output(args.format, &report, || render::ls(&report))
        }
        CommandKind::Cone(args) => {
            ensure_valid_config(&project)?;
            let path = project_relative_arg(&project, &args.path)?;
            let report =
                map::cone_report(&project, &path, args.depth, args.include_hidden, args.limit);
            output(args.format, &report, || render::cone(&report))
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
        CommandKind::Proof(args) => proof(&project, args),
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

