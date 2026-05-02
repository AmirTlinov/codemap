fn proof(project: &crate::model::Project, args: ProofArgs) -> Result<()> {
    ensure_valid_config(project)?;
    let (target, changed, selector) = proof_inputs(project, &args)?;
    let report = map::proof_report(project, target, changed, selector, args.depth, args.limit);
    maybe_write_proof_changed_lens_cache(project, &args, &report);
    if args.run {
        render::proof(&report);
        return run_proof_plan(project, &report);
    }
    output(args.format, &report, || render::proof(&report))
}

fn maybe_write_proof_changed_lens_cache(
    project: &crate::model::Project,
    args: &ProofArgs,
    report: &crate::model::ProofReport,
) {
    if args.run
        || args.target.is_some()
        || args
            .files
            .as_deref()
            .is_some_and(|files| !files.trim().is_empty())
    {
        return;
    }
    if !args.changed && !args.staged && args.since.is_none() {
        return;
    }
    let selector = if args.staged {
        "--staged".to_string()
    } else if let Some(since) = args.since.as_deref() {
        format!("--since {}", shell_quote_arg(since))
    } else {
        "--changed".to_string()
    };
    let _ = crate::cache::write_proof_changed_report(
        &project.cache_dir,
        repo::VERSION,
        &project.root,
        &selector,
        args.depth,
        args.limit,
        report,
    );
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
    let rejected: Vec<(String, ProofCommandRejection)> = commands
        .iter()
        .filter_map(|command| {
            proof_command_rejection(command).map(|reason| (command.clone(), reason))
        })
        .collect();
    if !rejected.is_empty() {
        for (command, reason) in &rejected {
            match reason {
                ProofCommandRejection::Placeholder => {
                    eprintln!("codemap: cannot run placeholder proof command: {command}");
                }
                ProofCommandRejection::Unsafe(reason) => {
                    eprintln!("codemap: refusing unsafe proof command: {command} ({reason})");
                }
                ProofCommandRejection::Unknown => {
                    eprintln!("codemap: refusing unknown proof command: {command}");
                }
            }
        }
        if rejected
            .iter()
            .all(|(_, reason)| matches!(reason, ProofCommandRejection::Placeholder))
        {
            bail!(
                "verification plan contains non-runnable placeholder commands for the selected scope"
            );
        }
        bail!("verification plan contains proof commands that codemap will not run by default");
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProofCommandRejection {
    Placeholder,
    Unsafe(&'static str),
    Unknown,
}

fn proof_command_rejection(command: &str) -> Option<ProofCommandRejection> {
    let command = command.trim();
    if command.is_empty() || command.contains("nearest domain tests") {
        return Some(ProofCommandRejection::Placeholder);
    }
    if let Some(reason) = unsafe_shell_syntax_reason(command) {
        return Some(ProofCommandRejection::Unsafe(reason));
    }
    if safe_proof_command(command) {
        return None;
    }
    if let Some(reason) = unsafe_proof_command_reason(command) {
        return Some(ProofCommandRejection::Unsafe(reason));
    }
    Some(ProofCommandRejection::Unknown)
}

fn unsafe_proof_command_reason(command: &str) -> Option<&'static str> {
    let lower = command.to_ascii_lowercase();
    let deny = [
        ("deploy", "deploy command"),
        ("release", "release command"),
        ("publish", "publish command"),
        ("migrate", "migration command"),
        ("db push", "database mutation"),
        ("kubectl", "cluster mutation"),
        ("helm", "cluster mutation"),
        ("terraform", "infrastructure mutation"),
        ("pulumi", "infrastructure mutation"),
        ("ansible", "remote mutation"),
        ("ssh ", "remote shell"),
        ("scp ", "remote copy"),
        ("rsync ", "remote/local sync"),
        ("curl ", "network mutation risk"),
        ("wget ", "network mutation risk"),
        ("rm -", "destructive file operation"),
        ("dropdb", "database mutation"),
        ("psql ", "database mutation"),
        ("mysql ", "database mutation"),
        ("docker compose up", "service startup"),
        ("docker-compose up", "service startup"),
    ];
    deny.iter()
        .find_map(|(needle, reason)| lower.contains(needle).then_some(*reason))
}

fn unsafe_shell_syntax_reason(command: &str) -> Option<&'static str> {
    let mut chars = command.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\n' | '\r' => return Some("multi-line shell command"),
            ';' | '|' | '`' | '$' | '>' | '<' => return Some("shell control syntax"),
            '&' if chars.peek() == Some(&'&') => {
                chars.next();
            }
            '&' => return Some("shell background syntax"),
            _ => {}
        }
    }
    let and_count = command.match_indices("&&").count();
    if and_count > 1 {
        return Some("multiple shell command separators");
    }
    if and_count == 1 {
        let Some((prefix, _tail)) = command.split_once("&&") else {
            return Some("shell command separator");
        };
        if !safe_cd_prefix(prefix) {
            return Some("only scoped cd prefix may compose proof commands");
        }
    }
    None
}

fn safe_proof_command(command: &str) -> bool {
    let command = command.trim();
    if let Some((prefix, tail)) = command.split_once("&&") {
        return safe_cd_prefix(prefix) && safe_proof_command(tail);
    }
    let lower = command.to_ascii_lowercase();
    let safe_prefixes = [
        "cargo test",
        "cargo nextest",
        "cargo clippy",
        "cargo check",
        "cargo build",
        "go test",
        "pytest",
        "python -m pytest",
        "swift test",
        "npm test",
        "pnpm test",
        "yarn test",
        "bun test",
        "pnpm exec vitest",
        "pnpm exec jest",
        "pnpm exec uvu",
        "pnpm exec ava",
        "pnpm exec mocha",
        "pnpm exec node --test",
        "pnpm exec tsx",
        "npm exec vitest",
        "npm exec jest",
        "npm exec uvu",
        "npm exec ava",
        "npm exec mocha",
        "npm exec node --test",
        "npm exec tsx",
        "npx vitest",
        "npx jest",
        "npx uvu",
        "npx ava",
        "npx mocha",
        "npx node --test",
        "npx tsx",
        "yarn vitest",
        "yarn jest",
        "yarn uvu",
        "yarn ava",
        "yarn mocha",
        "yarn node --test",
        "yarn tsx",
        "bunx vitest",
        "bunx jest",
        "bunx uvu",
        "bunx ava",
        "bunx mocha",
        "bunx node --test",
        "bunx tsx",
        "pnpm exec playwright test",
        "npm exec playwright test",
        "npx playwright test",
        "yarn playwright test",
        "bunx playwright test",
        "codemap boundaries",
        "codemap proof-map",
        "codemap changed",
    ];
    if safe_prefixes
        .iter()
        .any(|prefix| command_has_prefix(&lower, prefix))
    {
        return true;
    }
    safe_package_script_command(&lower) || safe_make_or_just_command(&lower)
}

fn safe_cd_prefix(prefix: &str) -> bool {
    let prefix = prefix.trim();
    let Some(path) = cd_prefix_path(prefix) else {
        return false;
    };
    !path.is_empty()
        && path != "~"
        && !path.starts_with('/')
        && !path.starts_with("~/")
        && !path.starts_with('-')
        && !path.split('/').any(|part| part == "..")
}

fn cd_prefix_path(prefix: &str) -> Option<String> {
    let rest = prefix.strip_prefix("cd ")?.trim();
    if rest.is_empty() {
        return None;
    }
    if let Some(quote) = rest.chars().next().filter(|ch| *ch == '\'' || *ch == '"') {
        let suffix = rest.strip_prefix(quote)?.strip_suffix(quote)?;
        return Some(suffix.to_string());
    }
    if rest.chars().any(char::is_whitespace) {
        return None;
    }
    Some(rest.to_string())
}

fn command_has_prefix(command: &str, prefix: &str) -> bool {
    command == prefix
        || command
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with(char::is_whitespace))
}

fn safe_package_script_command(command: &str) -> bool {
    let parts = command.split_whitespace().collect::<Vec<_>>();
    let Some((runner, script)) = package_runner_and_script(&parts) else {
        return false;
    };
    matches!(runner, "npm" | "pnpm" | "yarn" | "bun") && safe_script_name(script)
}

fn package_runner_and_script<'a>(parts: &'a [&str]) -> Option<(&'a str, &'a str)> {
    match parts {
        ["npm", "run", script, ..] => Some(("npm", *script)),
        ["pnpm", "run", script, ..] => Some(("pnpm", *script)),
        ["yarn", script, ..] => Some(("yarn", *script)),
        ["bun", "run", script, ..] => Some(("bun", *script)),
        _ => None,
    }
}

fn safe_script_name(script: &str) -> bool {
    let script = script.trim_matches('\'').trim_matches('"');
    let lower = script.to_ascii_lowercase();
    if ["deploy", "release", "publish", "migrate", "db:push"]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        return false;
    }
    script.contains("test")
        || script.starts_with("typecheck")
        || script == "check"
        || script.starts_with("check:")
        || script == "lint"
        || script.starts_with("lint:")
        || script == "build"
        || script.starts_with("build:")
}

fn safe_make_or_just_command(command: &str) -> bool {
    let parts = command.split_whitespace().collect::<Vec<_>>();
    matches!(
        parts.as_slice(),
        ["make", target, ..] | ["just", target, ..]
            if safe_script_name(target)
    )
}
