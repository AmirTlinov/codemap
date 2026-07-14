// Responsibility: map-boundary-risk-plan
use crate::map::{
    find_script, impacted_domains, infer_minimal_commands, package_name_for_file, unique,
};
use crate::model::CountFact;
use crate::model::{FileSummary, Project, Risk, VerificationPlan};

pub(crate) fn missing_file_summary(project: &Project, rel: &str) -> FileSummary {
    let exists = project.root.join(rel).exists();
    FileSummary {
        path: rel.to_string(),
        kind: if exists { "unindexed" } else { "missing" }.to_string(),
        package: package_name_for_file(project, rel),
        language: "unknown".to_string(),
        lines: 0,
        roles: Vec::new(),
        symbols: Vec::new(),
        exports: Vec::new(),
        imports: Vec::new(),
        imported_by: CountFact::unknown("file is not indexed"),
    }
}

pub fn verification_plan(
    project: &Project,
    changed: &[String],
    impacted: &[String],
) -> VerificationPlan {
    let all_files: Vec<String> = changed
        .iter()
        .chain(impacted.iter())
        .cloned()
        .collect::<Vec<_>>();
    let domains = if all_files.is_empty() {
        project.domains.iter().collect::<Vec<_>>()
    } else {
        impacted_domains(project, &all_files)
    };
    let max_risk = all_files
        .iter()
        .map(|f| impact_level_for_file(project, f).0)
        .max()
        .unwrap_or(Risk::Low);

    let mut minimal = if changed.is_empty() {
        project.anchors.verification.default.clone()
    } else {
        project.anchors.proof.changed.clone()
    };
    if minimal.is_empty() {
        minimal = project.anchors.verification.default.clone();
    }
    if minimal.is_empty() {
        minimal = infer_minimal_commands(project, &domains, &all_files, changed);
    }
    let mut supplemental = Vec::new();
    if matches!(max_risk, Risk::MediumHigh | Risk::High | Risk::Critical)
        && let Some(typecheck) = find_script(project, &["typecheck", "tsc", "check"])
    {
        supplemental.push(typecheck);
    }
    if matches!(max_risk, Risk::High | Risk::Critical) {
        supplemental.push("codemap boundaries --changed".to_string());
    }
    let mut full = Vec::new();
    if matches!(max_risk, Risk::Critical)
        && let Some(test) = find_script(project, &["test"])
    {
        full.push(test);
    }
    VerificationPlan {
        minimal: unique(minimal).into_iter().take(3).collect(),
        supplemental: unique(supplemental).into_iter().take(3).collect(),
        full_only_if_triggered: unique(full).into_iter().take(3).collect(),
    }
}

pub(crate) fn impact_level_for_file(project: &Project, rel: &str) -> (Risk, Vec<String>) {
    let Some(file) = project.files.get(rel) else {
        return (Risk::Medium, vec!["file not found in scan".to_string()]);
    };
    let mut risk = Risk::Low;
    let mut reasons = Vec::new();
    let mut bump = |level, reason: &str| {
        risk = risk.max(level);
        reasons.push(reason.to_string());
    };
    if file.has_role("generated") {
        bump(Risk::Critical, "generated file");
    }
    if file.has_role("semantic_anchor") {
        bump(Risk::High, "semantic context anchor");
    }
    if file.has_role("public_boundary") {
        bump(Risk::Critical, "public boundary");
    }
    if file.has_role("schema_contract") {
        bump(Risk::High, "schema/contract/DTO");
    }
    if file.has_role("state_model") || file.has_role("persistence") {
        bump(Risk::High, "state model / persistence");
    }
    if file.has_role("runtime_state") {
        bump(Risk::MediumHigh, "runtime state / session/controller");
    }
    if file.has_role("cli_surface") {
        bump(Risk::High, "CLI command surface");
    }
    if file.has_role("build_ci") {
        bump(Risk::MediumHigh, "build/CI configuration");
    }
    if file.has_role("repo_discovery") {
        bump(Risk::MediumHigh, "repo discovery / inventory");
    }
    if file.has_role("cache") {
        bump(Risk::Medium, "external cache / fingerprint");
    }
    let fan_in = project
        .reverse_imports
        .get(rel)
        .map(|x| x.len())
        .unwrap_or(0);
    let fan_out = file.resolved_imports.len();
    if fan_in >= 8 {
        bump(Risk::Critical, &format!("high fan-in ({fan_in} importers)"));
    } else if fan_in >= 3 {
        bump(Risk::High, &format!("multiple importers ({fan_in})"));
    }
    if fan_out >= 12 {
        bump(Risk::Medium, &format!("high fan-out ({fan_out} imports)"));
    }
    if file.has_role("test") {
        bump(Risk::Low, "test file");
    }
    (risk, unique(reasons))
}
