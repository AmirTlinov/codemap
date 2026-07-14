// Responsibility: cli-proof-args
use crate::cli::{DEFAULT_PROOF_LIMIT, OutputFormat, ProofSection, default_output_format};
use clap::Args;

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
