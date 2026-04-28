use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::model::VerificationPlan;
use crate::{render, repo, route};

#[derive(Debug, Parser)]
#[command(name = "ctx")]
#[command(about = "External task-specific context kernel for AI coding agents")]
#[command(version)]
pub struct Cli {
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    #[command(subcommand)]
    command: CommandKind,
}

#[derive(Debug, Subcommand)]
enum CommandKind {
    #[command(about = "Check environment, repo detection, cache path, and safety defaults")]
    Doctor(FormatArgs),
    #[command(about = "Show repo, cache, language, domain, and verification status")]
    Status(FormatArgs),
    #[command(about = "List indexed project files without writing to the project")]
    Files(FilesArgs),
    #[command(about = "Print or explicitly write optional ctx bootloader/config files")]
    Init(InitArgs),
    #[command(about = "Print one-time global agent instruction text")]
    Bootstrap(BootstrapArgs),
    #[command(about = "Find likely domain or package for a task")]
    Locate(LocateArgs),
    #[command(about = "Return a task-specific context capsule")]
    Start(StartArgs),
    #[command(about = "Report affected files and verification plan for a diff")]
    Impact(ImpactArgs),
    #[command(about = "Print verification commands, or run them only with --run")]
    Verify(VerifyArgs),
    #[command(about = "Explain a file or anchored concept")]
    Explain(ExplainArgs),
    #[command(about = "Add the next bounded context layer when an expansion trigger fires")]
    Widen(WidenArgs),
    #[command(about = "Render a small graph lens as Mermaid, Markdown, or JSON")]
    Graph(GraphArgs),
    #[command(alias = "check-boundaries")]
    #[command(about = "Check explicit forbidden boundaries and generated-file edits")]
    Boundaries(BoundariesArgs),
    #[command(about = "Validate optional .ctx.yml semantic anchors")]
    Anchors(AnchorsArgs),
}

#[derive(Debug, Args)]
struct FormatArgs {
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct FilesArgs {
    #[arg(long)]
    path: Option<String>,
    #[arg(long, default_value_t = 200)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct InitArgs {
    #[arg(long)]
    agents: bool,
    #[arg(long)]
    print: bool,
    #[arg(long, alias = "write")]
    write_minimal: bool,
    #[arg(long)]
    path: Option<String>,
    #[arg(long)]
    force: bool,
}

#[derive(Debug, Args)]
struct BootstrapArgs {
    #[arg(long)]
    global_instruction: bool,
}

#[derive(Debug, Args)]
struct LocateArgs {
    #[arg(long)]
    task: String,
    #[arg(long, default_value_t = 5)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct StartArgs {
    #[arg(long)]
    task: String,
    #[arg(long)]
    path: Option<String>,
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,
    #[arg(long, default_value_t = 7)]
    limit: usize,
    #[arg(long)]
    for_agent: bool,
}

#[derive(Debug, Args)]
struct ImpactArgs {
    #[arg(long)]
    changed: bool,
    #[arg(long)]
    staged: bool,
    #[arg(long)]
    since: Option<String>,
    #[arg(long)]
    files: Option<String>,
    #[arg()]
    positional_files: Vec<String>,
    #[arg(long, default_value_t = 1)]
    depth: usize,
    #[arg(long, default_value_t = 30)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,
    #[arg(long)]
    for_agent: bool,
}

#[derive(Debug, Args)]
struct VerifyArgs {
    #[arg(long)]
    changed: bool,
    #[arg(long)]
    staged: bool,
    #[arg(long)]
    since: Option<String>,
    #[arg(long)]
    files: Option<String>,
    #[arg()]
    positional_files: Vec<String>,
    #[arg(long)]
    run: bool,
    #[arg(long)]
    recommended: bool,
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,
    #[arg(long)]
    for_agent: bool,
}

#[derive(Debug, Args)]
struct ExplainArgs {
    target: String,
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct WidenArgs {
    #[arg(long, default_value = "")]
    task: String,
    #[arg(long)]
    path: Option<String>,
    #[arg(long)]
    reason: String,
    #[arg(long)]
    already: Vec<String>,
    #[arg(long, default_value_t = 7)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct GraphArgs {
    #[arg(long)]
    path: Option<String>,
    #[arg(long, default_value = "causal")]
    lens: String,
    #[arg(long)]
    changed: bool,
    #[arg(long, default_value_t = 12)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = OutputFormat::Mermaid)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct BoundariesArgs {
    #[arg(long)]
    changed: bool,
    #[arg(long)]
    strict_warnings: bool,
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct AnchorsArgs {
    #[command(subcommand)]
    action: AnchorAction,
}

#[derive(Debug, Subcommand)]
enum AnchorAction {
    Validate(FormatArgs),
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
enum OutputFormat {
    Markdown,
    Json,
    Mermaid,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let root_hint = cli.root.clone().or_else(|| command_root_hint(&cli.command));

    if let CommandKind::Bootstrap(args) = &cli.command {
        if args.global_instruction {
            print!("{}", render::global_instruction());
        } else {
            println!("Use `ctx bootstrap --global-instruction`.");
        }
        return Ok(());
    }

    let project = repo::load_project(root_hint)?;
    match cli.command {
        CommandKind::Doctor(args) => {
            let report = route::status_report(&project);
            output(args.format, &report, || render::status(&report, true))
        }
        CommandKind::Status(args) => {
            let report = route::status_report(&project);
            output(args.format, &report, || render::status(&report, false))
        }
        CommandKind::Files(args) => {
            let report = files_report(&project, args.path.as_deref(), args.limit);
            output(args.format, &report, || files_markdown(&report))
        }
        CommandKind::Init(args) => init(&project, args),
        CommandKind::Bootstrap(_) => Ok(()),
        CommandKind::Locate(args) => {
            let report = route::locate_report(&project, &args.task, args.limit);
            output(args.format, &report, || render::locate(&report))
        }
        CommandKind::Start(args) => {
            let capsule =
                route::start_capsule(&project, &args.task, args.path.as_deref(), args.limit);
            output(args.format, &capsule, || render::start(&capsule))
        }
        CommandKind::Impact(args) => {
            let changed = changed_from_args(&project, &args);
            let report = route::impact_report(&project, changed, args.depth, args.limit);
            output(args.format, &report, || render::impact(&report))
        }
        CommandKind::Verify(args) => verify(&project, args),
        CommandKind::Explain(args) => {
            let report = route::explain_target(&project, &args.target);
            output(args.format, &report, || render::explain(&report))
        }
        CommandKind::Widen(args) => {
            let report = route::widen_context(
                &project,
                &args.task,
                args.path.as_deref(),
                &args.reason,
                &args.already,
                args.limit,
            );
            output(args.format, &report, || render::widen(&report))
        }
        CommandKind::Graph(args) => {
            let changed = if args.changed {
                repo::changed_files(&project.root, false, None)
            } else {
                Vec::new()
            };
            let graph = route::graph_lens(
                &project,
                args.path.as_deref(),
                &args.lens,
                args.limit,
                &changed,
            );
            match args.format {
                OutputFormat::Json => render::print_json(&graph),
                OutputFormat::Mermaid => {
                    render::graph_mermaid(&graph);
                    Ok(())
                }
                OutputFormat::Markdown => {
                    render::graph_markdown(&graph);
                    Ok(())
                }
            }
        }
        CommandKind::Boundaries(args) => {
            let changed = if args.changed {
                Some(
                    repo::changed_files(&project.root, false, None)
                        .into_iter()
                        .collect::<BTreeSet<_>>(),
                )
            } else {
                None
            };
            let findings = route::boundary_findings(&project, changed.as_ref());
            let hard = findings
                .iter()
                .any(|f| f.status != "warn" && f.status != "warning");
            let warns = findings
                .iter()
                .any(|f| f.status == "warn" || f.status == "warning");
            output(args.format, &findings, || render::boundaries(&findings))?;
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

fn command_root_hint(command: &CommandKind) -> Option<PathBuf> {
    match command {
        CommandKind::Start(args) => absolute_path_hint(args.path.as_deref()),
        CommandKind::Widen(args) => absolute_path_hint(args.path.as_deref()),
        _ => None,
    }
}

fn absolute_path_hint(path: Option<&str>) -> Option<PathBuf> {
    path.map(PathBuf::from).filter(|path| path.is_absolute())
}

fn init(project: &crate::model::Project, args: InitArgs) -> Result<()> {
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
        let target = if let Some(path) = args.path.as_deref() {
            project.root.join(path).join(".ctx.yml")
        } else {
            project.root.join(".ctx.yml")
        };
        if target.exists() && !args.force {
            bail!(
                "{} already exists. Use --force to overwrite.",
                target.display()
            );
        }
        fs::write(&target, body)?;
        println!("Wrote `{}`.", target.display());
        return Ok(());
    }
    if args.print {
        render::init_suggestion(args.path.as_deref());
        return Ok(());
    }
    println!("`ctx init` writes nothing by default.");
    println!("Use one of:");
    println!("  ctx init --agents");
    println!("  ctx init --print [--path <scope>]");
    println!("  ctx init --write-minimal [--path <scope>]");
    Ok(())
}

fn verify(project: &crate::model::Project, args: VerifyArgs) -> Result<()> {
    let changed = changed_from_verify_args(project, &args);
    let impacted = Vec::new();
    let plan = route::verification_plan(project, &changed, &impacted);
    if args.run {
        render::verify(&changed, &plan);
        return run_plan(project, &plan, args.recommended);
    }
    output(args.format, &(changed.clone(), plan.clone()), || {
        render::verify(&changed, &plan)
    })
}

fn run_plan(
    project: &crate::model::Project,
    plan: &VerificationPlan,
    include_recommended: bool,
) -> Result<()> {
    let mut commands = plan.minimal.clone();
    if include_recommended {
        commands.extend(plan.recommended.clone());
    }
    for command in commands {
        if command.contains("nearest domain tests") {
            eprintln!("ctx: cannot run placeholder verification: {command}");
            continue;
        }
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

fn changed_from_args(project: &crate::model::Project, args: &ImpactArgs) -> Vec<String> {
    if args.changed {
        return repo::changed_files(&project.root, false, None);
    }
    if args.staged {
        return repo::changed_files(&project.root, true, None);
    }
    if let Some(since) = &args.since {
        return repo::changed_files(&project.root, false, Some(since));
    }
    parse_files(args.files.as_deref(), &args.positional_files)
}

fn changed_from_verify_args(project: &crate::model::Project, args: &VerifyArgs) -> Vec<String> {
    if args.changed {
        return repo::changed_files(&project.root, false, None);
    }
    if args.staged {
        return repo::changed_files(&project.root, true, None);
    }
    if let Some(since) = &args.since {
        return repo::changed_files(&project.root, false, Some(since));
    }
    parse_files(args.files.as_deref(), &args.positional_files)
}

fn parse_files(files: Option<&str>, positional: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(files) = files {
        out.extend(files.split(',').map(repo::normalize_rel_path));
    }
    out.extend(positional.iter().map(|s| repo::normalize_rel_path(s)));
    out.into_iter().filter(|s| s != ".").collect()
}

fn output<T: serde::Serialize>(
    format: OutputFormat,
    value: &T,
    markdown: impl FnOnce(),
) -> Result<()> {
    match format {
        OutputFormat::Json => render::print_json(value),
        OutputFormat::Markdown | OutputFormat::Mermaid => {
            markdown();
            Ok(())
        }
    }
}

#[derive(serde::Serialize)]
struct FilesReport {
    kind: &'static str,
    path: String,
    files: Vec<String>,
    count: usize,
}

fn files_report(project: &crate::model::Project, path: Option<&str>, limit: usize) -> FilesReport {
    let prefix = path
        .map(repo::normalize_rel_path)
        .filter(|p| p != ".")
        .map(|p| format!("{}/", p.trim_end_matches('/')));
    let mut files: Vec<String> = project
        .files
        .keys()
        .filter(|rel| prefix.as_ref().map(|p| rel.starts_with(p)).unwrap_or(true))
        .cloned()
        .collect();
    files.sort();
    let count = files.len();
    files.truncate(limit);
    FilesReport {
        kind: "files",
        path: path.unwrap_or(".").to_string(),
        files,
        count,
    }
}

fn files_markdown(report: &FilesReport) {
    println!("# Files\n");
    println!("Path: `{}`", report.path);
    println!("Shown: `{}` of `{}`\n", report.files.len(), report.count);
    if report.files.is_empty() {
        println!("- none");
    } else {
        for file in &report.files {
            println!("- `{file}`");
        }
    }
}

#[derive(serde::Serialize)]
struct AnchorValidation {
    kind: &'static str,
    ok: bool,
    problems: Vec<String>,
}

fn validate_anchors(project: &crate::model::Project) -> AnchorValidation {
    let mut problems = Vec::new();
    for (id, concept) in &project.anchors.concepts {
        for file in &concept.files {
            let rel = route::resolve_anchor_path(project, file);
            if !project.files.contains_key(&rel) {
                problems.push(format!("concept `{id}` declares missing file `{rel}`"));
            }
        }
    }
    for edge in &project.anchors.boundaries.forbidden {
        if edge.reason.is_empty() {
            problems.push("forbidden boundary without reason".to_string());
        }
    }
    AnchorValidation {
        kind: "anchor_validation",
        ok: problems.is_empty(),
        problems,
    }
}

fn anchors_markdown(report: &AnchorValidation) {
    println!("# Anchor Validation\n");
    if report.ok {
        println!("No anchor problems found.");
    } else {
        for problem in &report.problems {
            println!("- {problem}");
        }
    }
}
