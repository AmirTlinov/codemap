// Responsibility: project-ecosystem-support-projection-from-release-manifest
use crate::model::{Project, ProjectEcosystemSupport, ReleaseEcosystemSupport};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct ReleaseManifest {
    ecosystem_support_version: u32,
    ecosystem_support: Vec<ReleaseEcosystemSupport>,
}

pub(crate) fn release_ecosystem_support_version() -> u32 {
    release_manifest().ecosystem_support_version
}

pub(crate) fn release_ecosystem_support() -> Vec<ReleaseEcosystemSupport> {
    release_manifest().ecosystem_support
}

pub(crate) fn project_ecosystem_support(project: &Project) -> Vec<ProjectEcosystemSupport> {
    let mut observed = BTreeMap::<String, ObservedFamily>::new();
    for file in project.files.values() {
        let ecosystem = ecosystem_for_file(file);
        if let Some(ecosystem) = ecosystem {
            observed
                .entry(ecosystem.to_string())
                .or_default()
                .record(&file.rel, file.has_role("generated"));
        }
        if file.has_role("schema_contract") && ecosystem != Some("schema/protocol") {
            observed
                .entry("schema/protocol".to_string())
                .or_default()
                .record(&file.rel, file.has_role("generated"));
        }
        if file.has_role("generated") && ecosystem != Some("generated clients") {
            observed
                .entry("generated clients".to_string())
                .or_default()
                .record(&file.rel, true);
        }
    }
    for package in &project.packages {
        let ecosystem = match package.ecosystem.as_str() {
            "javascript" => "javascript/typescript",
            "python" => "python",
            "rust" => "rust",
            "go" => "go",
            "swift" => "swift",
            _ => continue,
        };
        observed
            .entry(ecosystem.to_string())
            .or_default()
            .record(&package.manifest, false);
    }
    release_ecosystem_support()
        .into_iter()
        .filter_map(|declaration| {
            let facts = observed.remove(&declaration.ecosystem)?;
            Some(ProjectEcosystemSupport {
                declaration,
                detected_files: facts.files,
                generated_files: facts.generated,
                examples: facts.examples,
            })
        })
        .collect()
}

fn release_manifest() -> ReleaseManifest {
    serde_json::from_str(include_str!("../../schemas/manifest.json"))
        .expect("embedded schema manifest must contain a valid ecosystem support contract")
}

fn ecosystem_for_file(file: &crate::model::FileInfo) -> Option<&'static str> {
    Some(match file.language.as_str() {
        "javascript/typescript" => "javascript/typescript",
        "python" => "python",
        "rust" => "rust",
        "go" => "go",
        "swift" => "swift",
        "shell" => "shell",
        "sql" => "sql",
        "schema" => "schema/protocol",
        "config" if matches!(file.ext.as_str(), "yaml" | "yml") => "yaml/config",
        "unknown" if crate::repo::is_source_ext(&file.ext) => "other source languages",
        _ => return None,
    })
}

#[derive(Default)]
struct ObservedFamily {
    files: usize,
    generated: usize,
    examples: Vec<String>,
}

impl ObservedFamily {
    fn record(&mut self, path: &str, generated: bool) {
        self.files += 1;
        self.generated += usize::from(generated);
        if self.examples.len() < 5 && !self.examples.iter().any(|item| item == path) {
            self.examples.push(path.to_string());
        }
    }
}
