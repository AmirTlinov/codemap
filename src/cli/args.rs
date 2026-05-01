use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use globset::GlobBuilder;

use crate::{map, render, repo};

#[derive(Debug, Parser)]
#[command(name = "codemap")]
#[command(about = "Structural code map CLI for AI coding agents")]
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
    #[command(about = "Show structural surfaces for an exact file or directory anchor")]
    Ls(LsArgs),
    #[command(about = "Show a bounded structural edge cone around an exact anchor")]
    Cone(ConeArgs),
    #[command(about = "List indexed project files without writing to the project")]
    Files(FilesArgs),
    #[command(about = "Print or explicitly write optional codemap bootloader/config files")]
    Init(InitArgs),
    #[command(about = "Print one-time global agent instruction text")]
    Bootstrap(BootstrapArgs),
    #[command(about = "Print a bundled stable JSON schema or schema manifest")]
    Schema(SchemaArgs),
    #[command(about = "Report structural blast-radius clusters for a diff or explicit files")]
    Impact(ImpactArgs),
    #[command(about = "Show structural map changes for a diff without printing textual diff")]
    DiffMap(DiffMapArgs),
    #[command(about = "Show public/schema/export contract surface for an exact anchor")]
    Contract(ContractArgs),
    #[command(about = "Show runtime entrypoints, routes, scripts, and env surfaces for a scope")]
    Runtime(RuntimeArgs),
    #[command(about = "Print structural proof surfaces, or run them only with --run")]
    Proof(ProofArgs),
    #[command(about = "Show proof coverage surfaces around a scope or diff")]
    ProofMap(ProofMapArgs),
    #[command(about = "Show structural blockers and cleanup map before deleting an anchor")]
    Delete(DeleteArgs),
    #[command(about = "Show read-only package/domain boundary crossings for a scope")]
    BoundaryMap(BoundaryMapArgs),
    #[command(about = "Show a bounded structural flow from an exact anchor")]
    Flow(FlowArgs),
    #[command(about = "Show same-scope structural siblings and local conventions")]
    Siblings(SiblingsArgs),
    #[command(about = "Show existing placement conventions for a scope and kind")]
    Place(PlaceArgs),
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
struct LsArgs {
    path: String,
    #[arg(long)]
    include_hidden: bool,
    #[arg(long, default_value_t = 20)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct ConeArgs {
    path: String,
    #[arg(long, default_value_t = 1)]
    depth: usize,
    #[arg(long)]
    include_hidden: bool,
    #[arg(long, default_value_t = 20)]
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
struct SchemaArgs {
    #[arg(value_enum)]
    kind: SchemaKind,
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
}

#[derive(Debug, Args)]
struct DiffMapArgs {
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
    #[arg(long, default_value_t = 30)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct ContractArgs {
    path: String,
    #[arg(long)]
    include_hidden: bool,
    #[arg(long, default_value_t = 20)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct RuntimeArgs {
    #[arg(default_value = ".")]
    scope: String,
    #[arg(long)]
    include_hidden: bool,
    #[arg(long, default_value_t = 20)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct ProofArgs {
    target: Option<String>,
    #[arg(long)]
    changed: bool,
    #[arg(long)]
    staged: bool,
    #[arg(long)]
    since: Option<String>,
    #[arg(long)]
    files: Option<String>,
    #[arg(long, default_value_t = 1)]
    depth: usize,
    #[arg(long, default_value_t = 12)]
    limit: usize,
    #[arg(long)]
    run: bool,
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
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
    #[arg(long, default_value_t = 20)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct DeleteArgs {
    path: String,
    #[arg(long)]
    include_hidden: bool,
    #[arg(long, default_value_t = 20)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct BoundaryMapArgs {
    #[arg(default_value = ".")]
    scope: String,
    #[arg(long)]
    changed: bool,
    #[arg(long)]
    include_hidden: bool,
    #[arg(long, default_value_t = 20)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct FlowArgs {
    path: String,
    #[arg(long)]
    include_hidden: bool,
    #[arg(long, default_value_t = 20)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct SiblingsArgs {
    #[arg(default_value = ".")]
    scope: String,
    #[arg(long)]
    include_hidden: bool,
    #[arg(long, default_value_t = 20)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct PlaceArgs {
    #[arg(default_value = ".")]
    scope: String,
    #[arg(long)]
    kind: String,
    #[arg(long)]
    include_hidden: bool,
    #[arg(long, default_value_t = 20)]
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
    #[arg(long, value_enum, default_value_t = GraphOutputFormat::Mermaid)]
    format: GraphOutputFormat,
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
    Status,
    Files,
    Ls,
    Cone,
    Impact,
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
