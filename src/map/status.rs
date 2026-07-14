// Responsibility: map-status
use crate::map::{
    boundary_findings, ci_owner_proof_surfaces, env_ci_reference_proof_surfaces,
    env_consumer_proof_surfaces, env_declared_keys, is_support_artifact_path,
    line_may_contain_static_env_reference, manifest_ci_reference_proof_surfaces,
    manifest_script_proof_surfaces, prisma_env_names, schema_ci_reference_proof_surfaces,
    schema_owner_path, schema_script_proof_surfaces, shell_quote, static_env_names,
};
use crate::model::{FileInfo, Project};
use crate::{cache, repo};
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct StatusReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub root: String,
    pub cwd: String,
    pub vcs: Option<String>,
    pub config: Option<String>,
    pub config_errors: Vec<String>,
    pub nearest_agents: Option<String>,
    pub cache_dir: String,
    pub cache_state: String,
    pub cache_strategy: String,
    pub cache_artifacts: Vec<crate::model::CacheArtifactStatus>,
    pub scanner: crate::model::ScanStats,
    pub timings: crate::model::ProjectTimings,
    pub zero_footprint_default: bool,
    pub package_manager: String,
    pub languages: Vec<String>,
    pub files_scanned: usize,
    pub files_reused: usize,
    pub domains: Vec<DomainStatus>,
    pub scripts: Vec<String>,
    pub fingerprint: String,
    pub boundary_findings: usize,
    pub map_quality: Vec<MapQualityWarning>,
    pub unclassified_source_files: Vec<String>,
    pub unclassified_count: usize,
}

#[derive(Debug, Serialize)]
pub struct DomainStatus {
    pub id: String,
    pub path: String,
    pub config: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MapQualityWarning {
    pub kind: String,
    pub count: usize,
    pub examples: Vec<String>,
    pub reason: String,
    pub effect: String,
    pub expand: Option<String>,
}

pub fn status_report(project: &Project) -> StatusReport {
    let unclassified: Vec<String> = project
        .files
        .values()
        .filter(|file| repo::is_source_ext(&file.ext) && file.roles.is_empty())
        .map(|file| file.rel.clone())
        .collect();
    StatusReport {
        kind: "status_report",
        schema_version: "5",
        root: project.root.to_string_lossy().to_string(),
        cwd: project.cwd.to_string_lossy().to_string(),
        vcs: project.vcs.clone(),
        config: project.config_path.clone(),
        config_errors: project
            .config_errors
            .iter()
            .map(|error| format!("{}: {}", error.path, error.error))
            .collect(),
        nearest_agents: project.nearest_agents.clone(),
        cache_dir: project.cache_dir.to_string_lossy().to_string(),
        cache_state: project.cache_state.clone(),
        cache_strategy: project.cache_strategy.clone(),
        cache_artifacts: project.cache_artifacts.clone(),
        scanner: project.scan_stats.clone(),
        timings: project.timings.clone(),
        zero_footprint_default: true,
        package_manager: project.package_manager.clone(),
        languages: project.languages.iter().cloned().collect(),
        files_scanned: project.files.len(),
        files_reused: project.files_reused,
        domains: project
            .domains
            .iter()
            .map(|d| DomainStatus {
                id: d.id.clone(),
                path: d.path.clone(),
                config: d.config_path.clone(),
            })
            .collect(),
        scripts: project.scripts.iter().map(|s| s.command.clone()).collect(),
        fingerprint: cache::fingerprint(project, None),
        boundary_findings: boundary_findings(project, None).len(),
        map_quality: map_quality_warnings(project),
        unclassified_count: unclassified.len(),
        unclassified_source_files: unclassified.into_iter().take(30).collect(),
    }
}

fn map_quality_warnings(project: &Project) -> Vec<MapQualityWarning> {
    let mut warnings = Vec::new();
    push_missing_role_warning(
        project,
        &mut warnings,
        "manifest_role_missing",
        "manifest",
        "file name is a package/workspace manifest but scanner did not assign manifest role",
        "manifest cones and package verification surfaces may be incomplete",
        |file| {
            let name = status_file_name(&file.rel);
            repo::is_package_manifest_name(&name)
        },
    );
    push_missing_role_warning(
        project,
        &mut warnings,
        "env_role_missing",
        "env_config",
        "file name is .env* but scanner did not assign env_config role",
        "runtime config knobs may be absent from cone/changed unknowns",
        |file| {
            let name = status_file_name(&file.rel);
            repo::is_env_surface_name(&name)
        },
    );
    push_missing_role_warning(
        project,
        &mut warnings,
        "schema_role_missing",
        "schema_contract",
        "file path/extension looks like a schema or contract but scanner did not assign schema role",
        "schema cones and contract/proof checks may be incomplete",
        |file| {
            let name = status_file_name(&file.rel);
            !file.has_role("manifest")
                && !file.has_role("test")
                && !file.has_role("e2e_test")
                && !repo::is_package_manifest_name(&name)
                && repo::is_schema_contract_surface(
                    &file.rel.to_ascii_lowercase(),
                    &name,
                    &file.ext,
                )
                && !file.has_role("migration")
        },
    );
    push_owner_surface_proof_warning(
        project,
        &mut warnings,
        "manifest_without_deterministic_proof",
        "manifest",
        "no manifest script, package-local build/test command, or package-specific CI reference was found",
        "proof may only show broader fallback commands for these manifests",
        |_| true,
    );
    push_owner_surface_proof_warning(
        project,
        &mut warnings,
        "schema_without_deterministic_proof",
        "schema_contract",
        "no schema generate/migrate/seed script or CI reference was found",
        "schema owner proof may be fallback-only even when the schema is indexed",
        schema_proof_warning_candidate,
    );
    push_env_quality_warning(project, &mut warnings);
    push_stale_lens_artifact_warning(project, &mut warnings);
    warnings
}

fn push_missing_role_warning(
    project: &Project,
    warnings: &mut Vec<MapQualityWarning>,
    kind: &str,
    expected_role: &str,
    reason: &str,
    effect: &str,
    candidate: impl Fn(&FileInfo) -> bool,
) {
    let examples = project
        .files
        .values()
        .filter(|file| {
            !is_support_artifact_path(&file.rel) && candidate(file) && !file.has_role(expected_role)
        })
        .map(|file| file.rel.clone())
        .collect::<Vec<_>>();
    push_map_quality_warning(warnings, kind, examples, reason, effect, None);
}

fn push_owner_surface_proof_warning(
    project: &Project,
    warnings: &mut Vec<MapQualityWarning>,
    kind: &str,
    role: &str,
    reason: &str,
    effect: &str,
    candidate: impl Fn(&FileInfo) -> bool,
) {
    let examples = project
        .files
        .values()
        .filter(|file| file.has_role(role))
        .filter(|file| !is_support_artifact_path(&file.rel))
        .filter(|file| candidate(file))
        .filter(|file| !owner_surface_has_deterministic_proof(project, file))
        .map(|file| file.rel.clone())
        .collect::<Vec<_>>();
    let expand = examples
        .first()
        .map(|example| format!("codemap proof {}", shell_quote(example)));
    push_map_quality_warning(warnings, kind, examples, reason, effect, expand);
}

fn owner_surface_has_deterministic_proof(project: &Project, file: &FileInfo) -> bool {
    if file.has_role("manifest") {
        if !manifest_script_proof_surfaces(project, file).is_empty() {
            return true;
        }
        if !manifest_ci_reference_proof_surfaces(project, file).is_empty() {
            return true;
        }
    }
    if file.has_role("schema_contract") || schema_owner_path(&file.rel) {
        if !schema_script_proof_surfaces(project, file).is_empty() {
            return true;
        }
        if !schema_ci_reference_proof_surfaces(project, file).is_empty() {
            return true;
        }
    }
    if file.has_role("env_config") {
        if !env_consumer_proof_surfaces(project, file).is_empty() {
            return true;
        }
        if !env_ci_reference_proof_surfaces(project, file).is_empty() {
            return true;
        }
    }
    if file.has_role("build_ci") {
        return !ci_owner_proof_surfaces(project, file).is_empty();
    }
    false
}

fn schema_proof_warning_candidate(file: &FileInfo) -> bool {
    schema_owner_path(&file.rel)
}

fn push_env_quality_warning(project: &Project, warnings: &mut Vec<MapQualityWarning>) {
    let env_reader_keys = status_env_reader_keys(project);
    let examples = project
        .files
        .values()
        .filter(|file| file.has_role("env_config"))
        .filter(|file| !is_support_artifact_path(&file.rel))
        .filter_map(|file| {
            let keys = env_declared_keys(project, &file.rel);
            if keys.is_empty() {
                return Some(format!("{} (no declared keys)", file.rel));
            }
            let missing_consumers = keys
                .iter()
                .filter(|(key, _)| !env_reader_keys.contains(key.as_str()))
                .count();
            (missing_consumers > 0).then(|| {
                format!(
                    "{} ({missing_consumers} keys without static readers)",
                    file.rel
                )
            })
        })
        .collect::<Vec<_>>();
    let expand = examples.first().map(|example| {
        let path = example.split(' ').next().unwrap_or(example);
        format!("codemap cone {}", shell_quote(path))
    });
    push_map_quality_warning(
        warnings,
        "env_config_without_consumers",
        examples,
        "env file has no declared keys or has keys without deterministic static readers",
        "runtime config cone has missing-consumer unknowns for at least one key",
        expand,
    );
}

fn status_env_reader_keys(project: &Project) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for file in project.files.values() {
        if file.has_role("generated") || file.has_role("fixture") || file.has_role("archive") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(project.root.join(&file.rel)) else {
            continue;
        };
        for line in text.lines() {
            if !line_may_contain_static_env_reference(line) {
                continue;
            }
            keys.extend(static_env_names(line));
            keys.extend(prisma_env_names(line));
        }
    }
    keys
}

fn push_stale_lens_artifact_warning(project: &Project, warnings: &mut Vec<MapQualityWarning>) {
    let fingerprint = cache::fingerprint(project, None);
    let examples = cache::stale_lens_artifact_examples(
        &project.cache_dir,
        repo::VERSION,
        &project.root,
        &fingerprint,
    );
    push_map_quality_warning(
        warnings,
        "stale_lens_artifact",
        examples,
        "cached agent-facing lens artifact does not match current artifact format, binary version, root, or metadata",
        "that lens artifact is ignored and the next matching lens command rebuilds it instead of serving stale map output",
        None,
    );
}

fn push_map_quality_warning(
    warnings: &mut Vec<MapQualityWarning>,
    kind: &str,
    examples: Vec<String>,
    reason: &str,
    effect: &str,
    expand: Option<String>,
) {
    if examples.is_empty() {
        return;
    }
    warnings.push(MapQualityWarning {
        kind: kind.to_string(),
        count: examples.len(),
        examples: examples.into_iter().take(8).collect(),
        reason: reason.to_string(),
        effect: effect.to_string(),
        expand,
    });
}

pub(crate) fn status_file_name(rel: &str) -> String {
    Path::new(rel)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(rel)
        .to_ascii_lowercase()
}
