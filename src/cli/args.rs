// Responsibility: cli-args
use crate::cli::{ChangedSection, ConeSection, LsSection, ProofSection, WhereArgs};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

pub(crate) const DEFAULT_PROOF_LIMIT: usize = 12;

#[derive(Debug, Parser)]
#[command(name = "codemap")]
#[command(about = "Structural code map CLI for AI coding agents")]
#[command(before_help = "Primary map workflow:
  codemap ls [scope]
  codemap cone <anchor>
  codemap changed
  codemap proof <anchor|changed>

Diagnostics and deeper lenses stay available as exact expand targets.
")]
#[command(after_help = "Diagnostics:
  doctor, status, files, schema, bootstrap, init, anchors, boundaries

Focused map lenses:
  runtime, contract, flow, boundary-map, siblings, place, delete, diff-map, impact, proof-map, graph

Machine output:
  readable text is the agent default; JSON remains schema-backed for integrations.
")]
#[command(version)]
pub struct Cli {
    #[arg(long, global = true)]
    pub(crate) root: Option<PathBuf>,
    /// Compact agent output: collapse the repo prelude and drop repeated disclaimers.
    #[arg(long, global = true)]
    pub(crate) brief: bool,
    #[command(subcommand)]
    pub(crate) command: CommandKind,
}

#[derive(Debug, Subcommand)]
pub(crate) enum CommandKind {
    #[command(about = "Show structural surfaces for an exact file or directory anchor")]
    Ls(LsArgs),
    #[command(about = "Show a bounded structural edge cone around an exact anchor")]
    Cone(ConeArgs),
    #[command(
        about = "Show one compact after-edit structural map: observed facts, links, surface hints, verification surfaces, unknown gaps"
    )]
    Changed(ChangedArgs),
    #[command(
        about = "Print a verification plan (smallest justified command surface) for a target or changed set; runs commands only with --run"
    )]
    Proof(ProofArgs),
    #[command(hide = true)]
    #[command(about = "Check environment, repo detection, cache path, and safety defaults")]
    Doctor(FormatArgs),
    #[command(hide = true)]
    #[command(about = "Report structural blast-radius clusters for a diff or explicit files")]
    Impact(ImpactArgs),
    #[command(hide = true)]
    #[command(about = "Show structural map changes for a diff without printing textual diff")]
    DiffMap(DiffMapArgs),
    #[command(hide = true)]
    #[command(about = "Show public/schema/export contract surface for an exact anchor")]
    Contract(ContractArgs),
    #[command(hide = true)]
    #[command(about = "Show runtime entrypoints, routes, scripts, and env surfaces for a scope")]
    Runtime(RuntimeArgs),
    #[command(hide = true)]
    #[command(
        about = "Show a verification sensor inventory (all observed surfaces, bucketed) for a scope or diff; not a runnable plan"
    )]
    ProofMap(ProofMapArgs),
    #[command(hide = true)]
    #[command(about = "Show structural blockers and cleanup map before deleting an anchor")]
    Delete(DeleteArgs),
    #[command(hide = true)]
    #[command(about = "Show read-only package/domain boundary crossings for a scope")]
    BoundaryMap(BoundaryMapArgs),
    #[command(hide = true)]
    #[command(about = "Show a bounded structural flow from an exact anchor")]
    Flow(FlowArgs),
    #[command(hide = true)]
    #[command(about = "Locate every exact definition of a symbol name across the indexed map")]
    Where(WhereArgs),
    #[command(hide = true)]
    #[command(about = "Show same-scope structural siblings and local conventions")]
    Siblings(SiblingsArgs),
    #[command(hide = true)]
    #[command(about = "Show existing placement conventions for a scope and kind")]
    Place(PlaceArgs),
    #[command(hide = true)]
    #[command(about = "Render a small graph lens as Mermaid, Markdown, or JSON")]
    Graph(GraphArgs),
    #[command(hide = true)]
    #[command(alias = "check-boundaries")]
    #[command(about = "Check explicit forbidden boundaries and generated-file edits")]
    Boundaries(BoundariesArgs),
    #[command(hide = true)]
    #[command(about = "Show repo, cache, language, domain, and verification status")]
    Status(FormatArgs),
    #[command(hide = true)]
    #[command(
        about = "Print a read-only .codemap.yml dialect draft from deterministic repo patterns"
    )]
    Teach(FormatArgs),
    #[command(hide = true)]
    #[command(about = "List indexed project files without writing to the project")]
    Files(FilesArgs),
    #[command(hide = true)]
    #[command(about = "Print a bundled stable JSON schema or schema manifest")]
    Schema(SchemaArgs),
    #[command(hide = true)]
    #[command(about = "Print one-time global agent instruction text")]
    Bootstrap(BootstrapArgs),
    #[command(hide = true)]
    #[command(about = "Print or explicitly write optional codemap bootloader/config files")]
    Init(InitArgs),
    #[command(hide = true)]
    #[command(about = "Validate optional .codemap.yml semantic anchors")]
    Anchors(AnchorsArgs),
}

#[derive(Debug, Args)]
pub(crate) struct FormatArgs {
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Args)]
pub(crate) struct FilesArgs {
    #[arg(long)]
    pub(crate) path: Option<String>,
    #[arg(long, default_value_t = 200, hide = true)]
    pub(crate) limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Args)]
pub(crate) struct LsArgs {
    #[arg(default_value = ".")]
    pub(crate) path: String,
    #[arg(long, default_value_t = 1, hide = true)]
    pub(crate) depth: usize,
    #[arg(long, value_enum)]
    pub(crate) section: Option<LsSection>,
    #[arg(long = "all", alias = "include-hidden")]
    pub(crate) include_hidden: bool,
    #[arg(long, default_value_t = 20, hide = true)]
    pub(crate) limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    pub(crate) format: OutputFormat,
    #[arg(long, hide = true)]
    pub(crate) json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct ConeArgs {
    pub(crate) path: String,
    #[arg(long, default_value_t = 1)]
    pub(crate) depth: usize,
    #[arg(long, value_enum)]
    pub(crate) section: Option<ConeSection>,
    #[arg(long = "all", alias = "include-hidden")]
    pub(crate) include_hidden: bool,
    #[arg(long, default_value_t = 20, hide = true)]
    pub(crate) limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    pub(crate) format: OutputFormat,
    #[arg(long, hide = true)]
    pub(crate) json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct InitArgs {
    #[arg(long)]
    pub(crate) agents: bool,
    #[arg(long)]
    pub(crate) print: bool,
    #[arg(long, alias = "write")]
    pub(crate) write_minimal: bool,
    #[arg(long)]
    pub(crate) path: Option<String>,
    #[arg(long)]
    pub(crate) force: bool,
}

#[derive(Debug, Args)]
pub(crate) struct BootstrapArgs {
    #[arg(long)]
    pub(crate) global_instruction: bool,
}

#[derive(Debug, Args)]
pub(crate) struct SchemaArgs {
    #[arg(value_enum)]
    pub(crate) kind: SchemaKind,
}

#[derive(Debug, Args)]
pub(crate) struct ImpactArgs {
    #[arg(long, hide = true)]
    pub(crate) changed: bool,
    #[arg(long, hide = true)]
    pub(crate) staged: bool,
    #[arg(long, hide = true)]
    pub(crate) since: Option<String>,
    #[arg(long, hide = true)]
    pub(crate) files: Option<String>,
    #[arg(hide = true)]
    pub(crate) positional_files: Vec<String>,
    #[arg(long, default_value_t = 1)]
    pub(crate) depth: usize,
    #[arg(long, default_value_t = 30, hide = true)]
    pub(crate) limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Args)]
pub(crate) struct DiffMapArgs {
    #[arg(long, hide = true)]
    pub(crate) changed: bool,
    #[arg(long, hide = true)]
    pub(crate) staged: bool,
    #[arg(long, hide = true)]
    pub(crate) since: Option<String>,
    #[arg(long, hide = true)]
    pub(crate) files: Option<String>,
    #[arg(hide = true)]
    pub(crate) positional_files: Vec<String>,
    #[arg(long, default_value_t = 30, hide = true)]
    pub(crate) limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Args)]
pub(crate) struct ChangedArgs {
    #[arg(long, hide = true)]
    pub(crate) changed: bool,
    #[arg(long, hide = true)]
    pub(crate) staged: bool,
    #[arg(long, hide = true)]
    pub(crate) since: Option<String>,
    #[arg(long, hide = true)]
    pub(crate) files: Option<String>,
    #[arg(hide = true)]
    pub(crate) positional_files: Vec<String>,
    #[arg(long, default_value_t = 1, hide = true)]
    pub(crate) depth: usize,
    #[arg(long, value_enum)]
    pub(crate) section: Option<ChangedSection>,
    #[arg(long = "all", alias = "include-hidden")]
    pub(crate) include_hidden: bool,
    #[arg(long, default_value_t = 30, hide = true)]
    pub(crate) limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    pub(crate) format: OutputFormat,
    #[arg(long, hide = true)]
    pub(crate) json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct ContractArgs {
    pub(crate) path: String,
    #[arg(long = "all", alias = "include-hidden")]
    pub(crate) include_hidden: bool,
    #[arg(long, default_value_t = 20, hide = true)]
    pub(crate) limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Args)]
pub(crate) struct RuntimeArgs {
    #[arg(default_value = ".")]
    pub(crate) scope: String,
    #[arg(long = "all", alias = "include-hidden")]
    pub(crate) include_hidden: bool,
    #[arg(long, default_value_t = 20, hide = true)]
    pub(crate) limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Args)]
pub(crate) struct ProofArgs {
    pub(crate) target: Option<String>,
    #[arg(long, hide = true)]
    pub(crate) changed: bool,
    #[arg(long, hide = true)]
    pub(crate) staged: bool,
    #[arg(long, hide = true)]
    pub(crate) since: Option<String>,
    #[arg(long, hide = true)]
    pub(crate) files: Option<String>,
    #[arg(long, default_value_t = 1)]
    pub(crate) depth: usize,
    #[arg(long, value_enum)]
    pub(crate) section: Option<ProofSection>,
    #[arg(long = "all", alias = "include-hidden")]
    pub(crate) include_hidden: bool,
    #[arg(long, default_value_t = DEFAULT_PROOF_LIMIT, hide = true)]
    pub(crate) limit: usize,
    #[arg(long)]
    pub(crate) run: bool,
    #[arg(long, value_enum, default_value_t = default_output_format(), help = "Output format; markdown is the agent default, json is an integration escape hatch")]
    pub(crate) format: OutputFormat,
    #[arg(long, hide = true)]
    pub(crate) json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct ProofMapArgs {
    pub(crate) target: Option<String>,
    #[arg(long)]
    pub(crate) changed: bool,
    #[arg(long)]
    pub(crate) staged: bool,
    #[arg(long)]
    pub(crate) since: Option<String>,
    #[arg(long)]
    pub(crate) files: Option<String>,
    #[arg(long, help = "Show ungrouped per-seed verification sensors")]
    pub(crate) raw_sensors: bool,
    #[arg(long, default_value_t = 20, hide = true)]
    pub(crate) limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Args)]
pub(crate) struct DeleteArgs {
    pub(crate) path: String,
    #[arg(long = "all", alias = "include-hidden")]
    pub(crate) include_hidden: bool,
    #[arg(long, default_value_t = 20, hide = true)]
    pub(crate) limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Args)]
pub(crate) struct BoundaryMapArgs {
    #[arg(default_value = ".")]
    pub(crate) scope: String,
    #[arg(long)]
    pub(crate) changed: bool,
    #[arg(long = "all", alias = "include-hidden")]
    pub(crate) include_hidden: bool,
    #[arg(long, default_value_t = 20, hide = true)]
    pub(crate) limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Args)]
pub(crate) struct FlowArgs {
    pub(crate) path: String,
    #[arg(long = "all", alias = "include-hidden")]
    pub(crate) include_hidden: bool,
    #[arg(long, default_value_t = 20, hide = true)]
    pub(crate) limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Args)]
pub(crate) struct SiblingsArgs {
    #[arg(default_value = ".")]
    pub(crate) scope: String,
    #[arg(long = "all", alias = "include-hidden")]
    pub(crate) include_hidden: bool,
    #[arg(long, default_value_t = 20, hide = true)]
    pub(crate) limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Args)]
pub(crate) struct PlaceArgs {
    #[arg(default_value = ".")]
    pub(crate) scope: String,
    #[arg(long, default_value = "source")]
    pub(crate) kind: String,
    #[arg(long = "all", alias = "include-hidden")]
    pub(crate) include_hidden: bool,
    #[arg(long, default_value_t = 20, hide = true)]
    pub(crate) limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Args)]
pub(crate) struct GraphArgs {
    #[arg(long)]
    pub(crate) path: Option<String>,
    #[arg(long, default_value = "causal")]
    pub(crate) lens: String,
    #[arg(long)]
    pub(crate) changed: bool,
    #[arg(long, default_value_t = 12, hide = true)]
    pub(crate) limit: usize,
    #[arg(long, value_enum, default_value_t = default_graph_output_format(), hide = true)]
    pub(crate) format: GraphOutputFormat,
}

#[derive(Debug, Args)]
pub(crate) struct BoundariesArgs {
    #[arg(long)]
    pub(crate) changed: bool,
    #[arg(long)]
    pub(crate) strict_warnings: bool,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Args)]
pub(crate) struct AnchorsArgs {
    #[command(subcommand)]
    pub(crate) action: AnchorAction,
}

#[derive(Debug, Subcommand)]
pub(crate) enum AnchorAction {
    Validate(FormatArgs),
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub(crate) enum OutputFormat {
    Markdown,
    Json,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub(crate) enum GraphOutputFormat {
    Markdown,
    Json,
    Mermaid,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub(crate) enum SchemaKind {
    Manifest,
    Doctor,
    Status,
    Files,
    Ls,
    Cone,
    Impact,
    Changed,
    DiffMap,
    Contract,
    Runtime,
    Proof,
    ProofMap,
    Delete,
    BoundaryMap,
    Flow,
    Siblings,
    Place,
    Where,
    Anchors,
    AnchorValidation,
    Graph,
    Boundaries,
    Teach,
}

pub(crate) fn default_output_format() -> OutputFormat {
    match std::env::var("CODEMAP_FORMAT")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "json" => OutputFormat::Json,
        _ => OutputFormat::Markdown,
    }
}

pub(crate) fn output_format_with_json_alias(format: OutputFormat, json: bool) -> OutputFormat {
    if json { OutputFormat::Json } else { format }
}

fn default_graph_output_format() -> GraphOutputFormat {
    match std::env::var("CODEMAP_FORMAT")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "json" => GraphOutputFormat::Json,
        _ => GraphOutputFormat::Markdown,
    }
}
