fn proof(project: &crate::model::Project, args: ProofArgs) -> Result<()> {
    ensure_valid_config(project)?;
    let (target, changed, selector) = proof_inputs(project, &args)?;
    let report = map::proof_report(project, target, changed, selector, args.depth, args.limit);
    if args.run {
        render::proof(&report);
        return run_proof_plan(project, &report);
    }
    output(args.format, &report, || render::proof(&report))
}

fn run_proof_plan(
    project: &crate::model::Project,
    report: &crate::model::ProofReport,
) -> Result<()> {
    let proof_commands = report
        .proofs
        .iter()
        .filter_map(|proof| proof.command.clone())
        .collect::<Vec<_>>();
    let commands = if proof_commands.is_empty() {
        report.fallback.clone()
    } else {
        proof_commands
    };
    let plan = crate::model::VerificationPlan {
        minimal: commands,
        supplemental: Vec::new(),
        full_only_if_triggered: Vec::new(),
    };
    run_plan(project, &plan, false)
}

fn run_plan(
    project: &crate::model::Project,
    plan: &crate::model::VerificationPlan,
    include_supplemental: bool,
) -> Result<()> {
    for command in planned_run_commands(plan, include_supplemental)? {
        let command = resolve_run_command(&command)?;
        println!("\n$ {command}");
        let status = Command::new("sh")
            .arg("-lc")
            .arg(&command)
            .current_dir(&project.root)
            .status()?;
        if !status.success() {
            bail!("verification command failed: {command}");
        }
    }
    Ok(())
}

fn planned_run_commands(
    plan: &crate::model::VerificationPlan,
    include_supplemental: bool,
) -> Result<Vec<String>> {
    let mut commands = plan.minimal.clone();
    if include_supplemental {
        commands.extend(plan.supplemental.clone());
    }
    commands = commands
        .into_iter()
        .map(|command| command.trim().to_string())
        .filter(|command| !command.is_empty())
        .collect();
    if commands.is_empty() {
        bail!("no verification commands inferred; refusing to treat --run as successful");
    }
    let placeholders: Vec<String> = commands
        .iter()
        .filter(|command| !is_runnable_verification_command(command))
        .cloned()
        .collect();
    if !placeholders.is_empty() {
        for command in placeholders {
            eprintln!("codemap: cannot run placeholder proof command: {command}");
        }
        bail!(
            "verification plan contains non-runnable placeholder commands for the selected scope"
        );
    }
    Ok(unique_preserve_order(commands))
}

fn resolve_run_command(command: &str) -> Result<String> {
    let trimmed = command.trim();
    if trimmed == "codemap" || trimmed.starts_with("codemap ") {
        let exe = env::current_exe()?;
        let suffix = trimmed.strip_prefix("codemap").unwrap_or_default();
        return Ok(format!("{}{}", shell_quote_path(&exe), suffix));
    }
    Ok(trimmed.to_string())
}

fn unique_preserve_order(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            out.push(value);
        }
    }
    out
}

fn shell_quote_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-' | ':'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn is_runnable_verification_command(command: &str) -> bool {
    !command.trim().is_empty() && !command.contains("nearest domain tests")
}
