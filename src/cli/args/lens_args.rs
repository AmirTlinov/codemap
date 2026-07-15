// Responsibility: cli-lens-args
use crate::cli::{
    ChangedSection, ConeSection, GraphOutputFormat, LsSection, OutputFormat,
    default_graph_output_format, default_output_format, positive_usize,
};
use clap::Args;

#[derive(Debug, Args)]
pub(crate) struct LsArgs {
    /// Exact file, file#symbol, or directory scope; use `.` only for root orientation.
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

impl LsArgs {
    /// Root, exact anchors, and nested-directory machine projections are
    /// complete. Readable nested-directory output keeps its bounded signal.
    pub(crate) fn effective_projection(
        &self,
        path: &str,
        format: OutputFormat,
        exact_file: bool,
    ) -> (bool, usize, bool, bool) {
        let complete_json = format == OutputFormat::Json
            && (path == "." || crate::map::split_symbol_anchor(path).is_some());
        let complete_file_projection =
            self.include_hidden || (format == OutputFormat::Json && exact_file);
        let complete_directory_projection = path != "."
            && (self.include_hidden
                || (format == OutputFormat::Json
                    && !exact_file
                    && crate::map::split_symbol_anchor(path).is_none()));
        let include_hidden = self.include_hidden || complete_json;
        let limit = if include_hidden {
            usize::MAX / 2
        } else {
            self.limit
        };
        (
            include_hidden,
            limit,
            complete_file_projection,
            complete_directory_projection,
        )
    }
}

#[derive(Debug, Args)]
pub(crate) struct ConeArgs {
    /// Exact file, file#symbol, or directory anchor.
    pub(crate) path: String,
    #[arg(long, default_value_t = 1)]
    pub(crate) depth: usize,
    #[arg(long, value_enum)]
    pub(crate) section: Option<ConeSection>,
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
