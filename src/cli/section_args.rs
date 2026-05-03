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
