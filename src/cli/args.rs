// Responsibility: cli-args
mod lens_args;
mod proof_args;
mod setup_args;

pub(crate) use lens_args::*;
pub(crate) use proof_args::*;
pub(crate) use setup_args::*;

use crate::cli::WhereArgs;
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

pub(crate) const DEFAULT_PROOF_LIMIT: usize = 12;

#[derive(Debug, Parser)]
#[command(name = "codemap")]
#[command(about = "Structural code map CLI for AI coding agents")]
#[command(before_help = "Choose one proportional map entry:
  known symbol:     codemap where <symbol>
  known anchor:     codemap cone <file-or-file#symbol>
  known scope:      codemap ls <file-or-directory>
  unfamiliar scope: codemap ls .

Use the narrowest known anchor; root orientation is only for an unknown scope.

After edits:
  codemap changed
  codemap proof changed

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
    #[command(about = "Show structural surfaces for an exact file, symbol, or directory anchor")]
    Ls(LsArgs),
    #[command(about = "Locate every exact definition of a symbol name across the indexed map")]
    Where(WhereArgs),
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
    #[command(about = "Inspect or explicitly maintain the external cache")]
    Cache(CacheArgs),
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
    #[command(about = "Generate shell completion source for codemap")]
    Completions(CompletionsArgs),
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
pub(crate) struct CacheArgs {
    #[command(subcommand)]
    pub(crate) action: CacheAction,
}

#[derive(Debug, Subcommand)]
pub(crate) enum CacheAction {
    #[command(about = "Inspect external cache contents, retention, privacy, and failures")]
    Status(FormatArgs),
    #[command(about = "Collect expired quarantine, diagnostics, and temporary files")]
    Gc(FormatArgs),
    #[command(about = "Delete this repository's external cache")]
    Clear(CacheClearArgs),
}

#[derive(Debug, Args)]
pub(crate) struct CacheClearArgs {
    #[arg(long, help = "Confirm deletion of this repository's external cache")]
    pub(crate) yes: bool,
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
    Cache,
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

pub(crate) fn default_graph_output_format() -> GraphOutputFormat {
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

pub(crate) fn positive_usize(value: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("`{value}` is not a positive integer"))?;
    if parsed == 0 {
        return Err("value must be at least 1".to_string());
    }
    Ok(parsed)
}
