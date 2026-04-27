use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::render;
use crate::repo;

#[derive(Debug, Parser)]
#[command(name = "ctx")]
#[command(about = "Task-specific context router for coding agents")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Doctor,
    Status(StatusArgs),
    Init(InitArgs),
    Scan,
    Locate(TaskArgs),
    Start(StartArgs),
    Impact(ImpactArgs),
    Verify(VerifyArgs),
    Explain(ExplainArgs),
    Graph(GraphArgs),
    Boundaries,
    Widen(WidenArgs),
}

#[derive(Debug, Args)]
struct StatusArgs {
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct InitArgs {
    #[arg(long)]
    agents: bool,
    #[arg(long)]
    print: bool,
}

#[derive(Debug, Args)]
struct TaskArgs {
    #[arg(long)]
    task: String,
}

#[derive(Debug, Args)]
struct StartArgs {
    #[arg(long)]
    task: String,
    #[arg(long)]
    path: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct ImpactArgs {
    #[arg(long)]
    changed: bool,
    #[arg(long)]
    staged: bool,
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct VerifyArgs {
    #[arg(long)]
    changed: bool,
    #[arg(long)]
    run: bool,
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct ExplainArgs {
    target: String,
}

#[derive(Debug, Args)]
struct GraphArgs {
    #[arg(long, value_enum, default_value_t = GraphLens::Ownership)]
    lens: GraphLens,
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct WidenArgs {
    #[arg(long)]
    reason: String,
    #[arg(long)]
    task: Option<String>,
    #[arg(long)]
    path: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    Markdown,
    Json,
    Mermaid,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum GraphLens {
    Ownership,
    Impact,
    Boundary,
    Verification,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Doctor => doctor(),
        Command::Status(args) => status(args),
        Command::Init(args) => init(args),
        Command::Scan => planned("scan"),
        Command::Locate(args) => locate(args),
        Command::Start(args) => start(args),
        Command::Impact(args) => impact(args),
        Command::Verify(args) => verify(args),
        Command::Explain(args) => explain(args),
        Command::Graph(args) => graph(args),
        Command::Boundaries => planned("boundaries"),
        Command::Widen(args) => widen(args),
    }
}

fn doctor() -> Result<()> {
    let status = repo::detect_status()?;
    render::doctor(&status);
    Ok(())
}

fn status(args: StatusArgs) -> Result<()> {
    let status = repo::detect_status()?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&status)?);
    } else {
        render::status(&status);
    }
    Ok(())
}

fn init(args: InitArgs) -> Result<()> {
    if args.print || args.agents {
        render::init_suggestion(args.agents);
    } else {
        render::planned(
            "init",
            "Use --print or --agents once write semantics are implemented.",
        );
    }
    Ok(())
}

fn locate(args: TaskArgs) -> Result<()> {
    render::planned(
        "locate",
        &format!(
            "Task received: `{}`. Location scoring is planned for the routing engine slice.",
            args.task
        ),
    );
    Ok(())
}

fn start(args: StartArgs) -> Result<()> {
    match args.format {
        OutputFormat::Json => {
            let capsule = render::stub_capsule_json(&args.task, args.path.as_ref());
            println!("{}", serde_json::to_string_pretty(&capsule)?);
        }
        OutputFormat::Markdown | OutputFormat::Mermaid => {
            render::stub_capsule_markdown(&args.task, args.path.as_ref());
        }
    }
    Ok(())
}

fn impact(args: ImpactArgs) -> Result<()> {
    render::planned(
        "impact",
        &format!(
            "changed={}, staged={}. Git diff impact mapping is planned for the impact engine slice.",
            args.changed, args.staged
        ),
    );
    Ok(())
}

fn verify(args: VerifyArgs) -> Result<()> {
    let mode = if args.run { "execute" } else { "print-only" };
    render::planned(
        "verify",
        &format!(
            "changed={}, mode={}. Verification planning is planned; no project scripts are run yet.",
            args.changed, mode
        ),
    );
    Ok(())
}

fn explain(args: ExplainArgs) -> Result<()> {
    render::planned(
        "explain",
        &format!(
            "Target received: `{}`. File/concept explanation is planned.",
            args.target
        ),
    );
    Ok(())
}

fn graph(args: GraphArgs) -> Result<()> {
    match args.format {
        OutputFormat::Mermaid => {
            println!("graph TD");
            println!("  Ctx[\"ctx\"] --> Lens[\"{:?} lens\"]", args.lens);
            println!("  Lens --> Planned[\"graph renderer planned\"]");
        }
        OutputFormat::Json => {
            let value = serde_json::json!({
                "kind": "graph_lens",
                "lens": format!("{:?}", args.lens),
                "status": "planned"
            });
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        OutputFormat::Markdown => {
            render::planned(
                "graph",
                &format!(
                    "{:?} lens selected. Graph construction is planned.",
                    args.lens
                ),
            );
        }
    }
    Ok(())
}

fn widen(args: WidenArgs) -> Result<()> {
    render::planned(
        "widen",
        &format!(
            "Reason received: `{}`. Controlled widening is planned.",
            args.reason
        ),
    );
    Ok(())
}

fn planned(command: &str) -> Result<()> {
    render::planned(
        command,
        "Command surface is reserved; implementation slice pending.",
    );
    Ok(())
}
