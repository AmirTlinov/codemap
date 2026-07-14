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
enum LsSection {
    Observed,
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

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq)]
enum ProofSection {
    Observed,
    Links,
    Roles,
    Proof,
    #[value(alias = "unknowns")]
    Unknown,
    Hidden,
}

#[derive(Debug, Args)]
struct WhereArgs {
    /// Exact symbol name to locate across the indexed map.
    query: String,
    /// Optional symbol-kind filter (function, class, struct, ...).
    #[arg(long)]
    kind: Option<String>,
    #[arg(long = "all", alias = "include-hidden")]
    include_hidden: bool,
    #[arg(long, default_value_t = 20, hide = true)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = default_output_format(), hide = true)]
    format: OutputFormat,
    #[arg(long, hide = true)]
    json: bool,
}
