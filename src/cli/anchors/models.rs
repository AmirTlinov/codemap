// Responsibility: cli-anchors-models

#[derive(serde::Serialize)]
pub(crate) struct AnchorValidation {
    pub(crate) kind: &'static str,
    pub(crate) schema_version: &'static str,
    pub(crate) ok: bool,
    pub(crate) config: Option<String>,
    pub(crate) summary: AnchorValidationSummary,
    pub(crate) problems: Vec<String>,
    pub(crate) warnings: Vec<String>,
    pub(crate) details: Vec<AnchorValidationDetail>,
}

#[derive(serde::Serialize)]
pub(crate) struct AnchorValidationSummary {
    pub(crate) domains: usize,
    pub(crate) concepts: usize,
    pub(crate) role_patterns: usize,
    pub(crate) forbidden_boundaries: usize,
    pub(crate) verification_defaults: usize,
    pub(crate) proof_changed_commands: usize,
}

#[derive(serde::Serialize)]
pub(crate) struct AnchorValidationDetail {
    pub(crate) kind: String,
    pub(crate) id: String,
    pub(crate) status: String,
    pub(crate) message: String,
    pub(crate) next: Vec<String>,
}
