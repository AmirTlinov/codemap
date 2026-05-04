#[derive(Debug, Clone, serde::Serialize)]
pub struct TeachReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub config: Option<String>,
    pub role_patterns: Vec<TeachRolePattern>,
    pub proof_changed: Vec<TeachProofCommand>,
    pub codemap_yml: Vec<String>,
    pub expand: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TeachRolePattern {
    pub pattern: String,
    pub role: String,
    pub evidence: String,
    pub matched: usize,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TeachProofCommand {
    pub command: String,
    pub evidence: String,
    pub source: Option<String>,
    pub line_start: Option<usize>,
}
