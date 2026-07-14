// Responsibility: cli-setup-args
use crate::cli::{FormatArgs, SchemaKind};
use clap::{Args, Subcommand};

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
pub(crate) struct AnchorsArgs {
    #[command(subcommand)]
    pub(crate) action: AnchorAction,
}

#[derive(Debug, Subcommand)]
pub(crate) enum AnchorAction {
    Validate(FormatArgs),
}
