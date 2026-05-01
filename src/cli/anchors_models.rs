#[derive(serde::Serialize)]
struct AnchorValidation {
    kind: &'static str,
    schema_version: &'static str,
    ok: bool,
    config: Option<String>,
    summary: AnchorValidationSummary,
    problems: Vec<String>,
    warnings: Vec<String>,
    details: Vec<AnchorValidationDetail>,
}

#[derive(serde::Serialize)]
struct AnchorValidationSummary {
    domains: usize,
    concepts: usize,
    forbidden_boundaries: usize,
    verification_defaults: usize,
}

#[derive(serde::Serialize)]
struct AnchorValidationDetail {
    kind: String,
    id: String,
    status: String,
    message: String,
    next: Vec<String>,
}

