fn init(project: &crate::model::Project, args: InitArgs) -> Result<()> {
    let action_count = [args.agents, args.print, args.write_minimal]
        .into_iter()
        .filter(|enabled| *enabled)
        .count();
    if action_count > 1 {
        bail!("codemap init accepts only one of --agents, --print, or --write-minimal");
    }
    if args.agents && args.path.is_some() {
        bail!(
            "codemap init --agents writes the repository bootloader; use --root to select a different repository root"
        );
    }
    if args.agents {
        let target = project.root.join("AGENTS.md");
        if target.exists() && !args.force {
            bail!("AGENTS.md already exists. Use --force to overwrite.");
        }
        fs::write(&target, render::agents_bootloader())?;
        println!("Wrote `AGENTS.md` tiny bootloader.");
        return Ok(());
    }
    if args.write_minimal {
        let body = render::suggested_ctx_yml_for(args.path.as_deref());
        let target_dir = if let Some(path) = args.path.as_deref() {
            scoped_project_path(project, path)?
        } else {
            project.root.clone()
        };
        let target = target_dir.join(".ctx.yml");
        if target.exists() && !args.force {
            bail!(
                "{} already exists. Use --force to overwrite.",
                target.display()
            );
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, body)?;
        println!("Wrote `{}`.", target.display());
        return Ok(());
    }
    if args.print {
        let print_path = args
            .path
            .as_deref()
            .map(|path| project_relative_arg(project, path))
            .transpose()?;
        render::init_suggestion(print_path.as_deref());
        return Ok(());
    }
    println!("`codemap init` writes nothing by default.");
    println!("Use one of:");
    println!("  codemap init --agents");
    println!("  codemap init --print [--path <scope>]");
    println!("  codemap init --write-minimal [--path <scope>]");
    Ok(())
}

