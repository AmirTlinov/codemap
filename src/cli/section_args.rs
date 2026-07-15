// Responsibility: cli-section-args
use crate::cli::{OutputFormat, default_output_format, positive_usize};
use clap::{Args, ValueEnum};

pub(crate) fn accept_depth_compat(depth: usize, command: &str) -> anyhow::Result<()> {
    if depth <= 1 {
        return Ok(());
    }
    anyhow::bail!(
        "codemap {command} currently keeps depth fixed at 1; use `codemap cone <anchor> --depth {depth}` or `codemap proof <anchor|changed> --depth {depth}` for expanded neighborhoods"
    );
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub(crate) enum ChangedSection {
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
pub(crate) enum LsSection {
    Observed,
    Links,
    Roles,
    Proof,
    #[value(alias = "unknowns")]
    Unknown,
    Hidden,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub(crate) enum ConeSection {
    Observed,
    Links,
    Roles,
    Proof,
    #[value(alias = "unknowns")]
    Unknown,
    Hidden,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
pub(crate) enum ProofSection {
    Observed,
    Links,
    Roles,
    Proof,
    #[value(alias = "unknowns")]
    Unknown,
    Hidden,
}

#[derive(Debug, Args)]
pub(crate) struct WhereArgs {
    /// Exact symbol name to locate across the indexed map.
    pub(crate) query: String,
    /// Optional symbol-kind filter (function, class, struct, ...).
    #[arg(long)]
    pub(crate) kind: Option<String>,
    #[arg(long = "all", alias = "include-hidden")]
    pub(crate) include_hidden: bool,
    #[arg(
        long,
        default_value_t = 20,
        value_parser = positive_usize,
        hide = true
    )]
    pub(crate) limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    pub(crate) format: OutputFormat,
    #[arg(long, hide = true)]
    pub(crate) json: bool,
}
