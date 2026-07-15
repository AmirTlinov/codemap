// Responsibility: runtime-path-deployment-env-index
use crate::map::runtime_code_lines;
use crate::model::{EvidenceLocation, Project};
use std::collections::BTreeMap;

pub(crate) type DeploymentEnvIndex = BTreeMap<String, Vec<EvidenceLocation>>;

pub(crate) fn deployment_env_index(project: &Project) -> DeploymentEnvIndex {
    let mut out = BTreeMap::<String, Vec<EvidenceLocation>>::new();
    for file in project.files.values().filter(|file| deployment_file(file)) {
        let Some(text) = project.read_indexed_text(&file.rel) else {
            continue;
        };
        for (line_number, line) in runtime_code_lines(&text) {
            for name in deployment_env_names(&line) {
                out.entry(name).or_default().push(EvidenceLocation::line(
                    &file.rel,
                    line_number,
                    "deployment_env_declaration",
                ));
            }
        }
    }
    for locations in out.values_mut() {
        locations.sort_by(|a, b| {
            a.path
                .cmp(&b.path)
                .then_with(|| a.line_start.cmp(&b.line_start))
        });
        locations.dedup_by(|a, b| a.path == b.path && a.line_start == b.line_start);
    }
    out
}

fn deployment_file(file: &crate::model::FileInfo) -> bool {
    file.has_role("build_ci")
        || file.has_role("deploy")
        || file.rel.contains("/deploy/")
        || file.rel.contains("/k8s/")
        || file.rel.contains("/helm/")
        || file.rel.ends_with("Dockerfile")
}

fn deployment_env_names(line: &str) -> Vec<String> {
    let trimmed = line.trim().trim_start_matches('-').trim_start();
    let mut candidates = Vec::new();
    if let Some(value) = trimmed.strip_prefix("name:") {
        candidates.push(value);
    }
    if let Some(value) = trimmed.strip_prefix("ENV ") {
        candidates.push(value.split(['=', ' ']).next().unwrap_or_default());
    }
    if let Some(value) = trimmed.strip_prefix("--from-literal=") {
        candidates.push(value.split('=').next().unwrap_or_default());
    }
    if let Some((name, _)) = trimmed.split_once('=') {
        candidates.push(name);
    }
    if let Some((name, _)) = trimmed.split_once(':') {
        candidates.push(name);
    }
    candidates
        .into_iter()
        .map(|value| value.trim().trim_matches(['"', '\'', '`']))
        .filter(|value| env_name(value))
        .map(str::to_string)
        .collect()
}

fn env_name(value: &str) -> bool {
    value.len() >= 3
        && value
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        && value
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
}
