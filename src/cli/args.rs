use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use globset::GlobBuilder;

use crate::{map, render, repo};

const DEFAULT_PROOF_LIMIT: usize = 12;

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
    root: Option<PathBuf>,
    #[command(subcommand)]
    command: CommandKind,
}

#[derive(Debug, Subcommand)]
enum CommandKind {
    #[command(about = "Show structural surfaces for an exact file or directory anchor")]
    Ls(LsArgs),
    #[command(about = "Show a bounded structural edge cone around an exact anchor")]
    Cone(ConeArgs),
    #[command(about = "Show one compact after-edit structural map: observed facts, links, roles, proof, unknown gaps")]
    Changed(ChangedArgs),
    #[command(about = "Print structural proof surfaces, or run them only with --run")]
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
    #[command(about = "Show proof coverage surfaces around a scope or diff")]
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
    #[command(about = "Validate optional .ctx.yml semantic anchors")]
    Anchors(AnchorsArgs),
}

#[derive(Debug, Args)]
struct FormatArgs {
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct FilesArgs {
    #[arg(long)]
    path: Option<String>,
    #[arg(long, default_value_t = 200, hide = true)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct LsArgs {
    #[arg(default_value = ".")]
    path: String,
    #[arg(long = "all", alias = "include-hidden")]
    include_hidden: bool,
    #[arg(long, default_value_t = 20, hide = true)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct ConeArgs {
    path: String,
    #[arg(long, default_value_t = 1)]
    depth: usize,
    #[arg(long, value_enum)]
    section: Option<ConeSection>,
    #[arg(long = "all", alias = "include-hidden")]
    include_hidden: bool,
    #[arg(long, default_value_t = 20, hide = true)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
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
struct SchemaArgs {
    #[arg(value_enum)]
    kind: SchemaKind,
}

#[derive(Debug, Args)]
struct ImpactArgs {
    #[arg(long, hide = true)]
    changed: bool,
    #[arg(long, hide = true)]
    staged: bool,
    #[arg(long, hide = true)]
    since: Option<String>,
    #[arg(long, hide = true)]
    files: Option<String>,
    #[arg(hide = true)]
    positional_files: Vec<String>,
    #[arg(long, default_value_t = 1)]
    depth: usize,
    #[arg(long, default_value_t = 30, hide = true)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct DiffMapArgs {
    #[arg(long, hide = true)]
    changed: bool,
    #[arg(long, hide = true)]
    staged: bool,
    #[arg(long, hide = true)]
    since: Option<String>,
    #[arg(long, hide = true)]
    files: Option<String>,
    #[arg(hide = true)]
    positional_files: Vec<String>,
    #[arg(long, default_value_t = 30, hide = true)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct ChangedArgs {
    #[arg(long, hide = true)]
    changed: bool,
    #[arg(long, hide = true)]
    staged: bool,
    #[arg(long, hide = true)]
    since: Option<String>,
    #[arg(long, hide = true)]
    files: Option<String>,
    #[arg(hide = true)]
    positional_files: Vec<String>,
    #[arg(long, value_enum)]
    section: Option<ChangedSection>,
    #[arg(long = "all", alias = "include-hidden")]
    include_hidden: bool,
    #[arg(long, default_value_t = 30, hide = true)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    format: OutputFormat,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
enum ChangedSection {
    #[value(alias = "overview", alias = "diff")]
    Observed,
    #[value(alias = "impact")]
    Links,
    Roles,
    Proof,
    #[value(alias = "unknowns")]
    Unknown,
    Hidden,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
enum ConeSection {
    Observed,
    Links,
    Roles,
    Proof,
    #[value(alias = "unknowns")]
    Unknown,
    Hidden,
}

#[derive(Debug, Args)]
struct ContractArgs {
    path: String,
    #[arg(long = "all", alias = "include-hidden")]
    include_hidden: bool,
    #[arg(long, default_value_t = 20, hide = true)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct RuntimeArgs {
    #[arg(default_value = ".")]
    scope: String,
    #[arg(long = "all", alias = "include-hidden")]
    include_hidden: bool,
    #[arg(long, default_value_t = 20, hide = true)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct ProofArgs {
    target: Option<String>,
    #[arg(long, hide = true)]
    changed: bool,
    #[arg(long, hide = true)]
    staged: bool,
    #[arg(long, hide = true)]
    since: Option<String>,
    #[arg(long, hide = true)]
    files: Option<String>,
    #[arg(long, default_value_t = 1)]
    depth: usize,
    #[arg(long, default_value_t = DEFAULT_PROOF_LIMIT, hide = true)]
    limit: usize,
    #[arg(long)]
    run: bool,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct ProofMapArgs {
    target: Option<String>,
    #[arg(long)]
    changed: bool,
    #[arg(long)]
    staged: bool,
    #[arg(long)]
    since: Option<String>,
    #[arg(long)]
    files: Option<String>,
    #[arg(long, help = "Show ungrouped per-seed proof sensors")]
    raw_sensors: bool,
    #[arg(long, default_value_t = 20, hide = true)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct DeleteArgs {
    path: String,
    #[arg(long = "all", alias = "include-hidden")]
    include_hidden: bool,
    #[arg(long, default_value_t = 20, hide = true)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct BoundaryMapArgs {
    #[arg(default_value = ".")]
    scope: String,
    #[arg(long)]
    changed: bool,
    #[arg(long = "all", alias = "include-hidden")]
    include_hidden: bool,
    #[arg(long, default_value_t = 20, hide = true)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct FlowArgs {
    path: String,
    #[arg(long = "all", alias = "include-hidden")]
    include_hidden: bool,
    #[arg(long, default_value_t = 20, hide = true)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct SiblingsArgs {
    #[arg(default_value = ".")]
    scope: String,
    #[arg(long = "all", alias = "include-hidden")]
    include_hidden: bool,
    #[arg(long, default_value_t = 20, hide = true)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct PlaceArgs {
    #[arg(default_value = ".")]
    scope: String,
    #[arg(long)]
    kind: String,
    #[arg(long = "all", alias = "include-hidden")]
    include_hidden: bool,
    #[arg(long, default_value_t = 20, hide = true)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
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
    #[arg(long, default_value_t = 12, hide = true)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = default_graph_output_format(), hide = true)]
    format: GraphOutputFormat,
}

#[derive(Debug, Args)]
struct BoundariesArgs {
    #[arg(long)]
    changed: bool,
    #[arg(long)]
    strict_warnings: bool,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
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
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
enum GraphOutputFormat {
    Markdown,
    Json,
    Mermaid,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
enum SchemaKind {
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
    Anchors,
    AnchorValidation,
    Graph,
    Boundaries,
}

fn default_output_format() -> OutputFormat {
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
