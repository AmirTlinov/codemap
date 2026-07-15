// Responsibility: runtime-scope-and-detector-gap-certificates
use std::collections::BTreeSet;

use crate::model::{
    CoverageCertificate, CoverageLocation, CoverageReason, CoverageStop, FileInfo, Project,
    UnsupportedObservation,
};

use super::runtime_groups::{
    RuntimeGroupObservationInput, base_certificate, capability, root_scope_trim_stop,
};

const ENV_SUPPORTED_EXTS: &[&str] = &["cjs", "js", "jsx", "mjs", "py", "rs", "ts", "tsx"];
const SCRIPT_MANIFEST_NAMES: &[&str] = &[
    "package.json",
    "Cargo.toml",
    "go.mod",
    "pyproject.toml",
    "requirements.txt",
    "Package.swift",
    "Makefile",
    "makefile",
    "justfile",
    "Justfile",
];

/// The script catalog consults repository-root manifests only and keeps a
/// verification-oriented grammar, so nested manifests are exact exclusions and
/// zero is never promoted to proven-zero.
pub(super) fn scripts_certificate(
    project: &Project,
    input: &RuntimeGroupObservationInput<'_>,
) -> CoverageCertificate {
    let candidates = script_manifest_candidates(project, input.scope);
    let visited: Vec<&str> = candidates
        .iter()
        .copied()
        .filter(|rel| input.scope == "." && !rel.contains('/'))
        .collect();
    let mut certificate = base_certificate(
        project,
        input,
        "scripts",
        candidates.len() as u64,
        visited.len() as u64,
        vec![CoverageReason::UnsupportedConstruct],
    );
    let unvisited: Vec<String> = candidates
        .iter()
        .copied()
        .filter(|rel| !visited.contains(rel))
        .map(str::to_string)
        .collect();
    if !unvisited.is_empty() {
        certificate
            .excluded_files_by_reason
            .insert(CoverageReason::IncompleteTraversal, unvisited);
    }
    certificate.extractor_capabilities = vec![capability(
        "codemap.runtime-scripts",
        "manifest",
        &[
            "make_like_targets",
            "package_json_verification_scripts",
            "root_script_catalog",
        ],
    )];
    certificate
}

fn script_manifest_candidates<'a>(project: &'a Project, scope: &str) -> Vec<&'a str> {
    if let Some(file) = project.files.get(scope) {
        return script_manifest_name(&file.rel)
            .then_some(file.rel.as_str())
            .into_iter()
            .collect();
    }
    crate::map::files_under_directory(project, scope)
        .into_iter()
        .filter(|file| script_manifest_name(&file.rel))
        .map(|file| file.rel.as_str())
        .collect()
}

fn script_manifest_name(rel: &str) -> bool {
    std::path::Path::new(rel)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| SCRIPT_MANIFEST_NAMES.contains(&name))
}

/// Env extraction owns a static reference grammar for JS/TS, Python and Rust;
/// other source languages are exact unsupported exclusions and dynamic env
/// lookups become typed stops instead of silent omissions.
pub(super) fn env_certificate(
    project: &Project,
    input: &RuntimeGroupObservationInput<'_>,
) -> CoverageCertificate {
    let candidates: Vec<&&FileInfo> = input
        .scope_files
        .iter()
        .filter(|file| crate::repo::is_source_ext(&file.ext))
        .collect();
    let mut visited = 0_u64;
    let mut excluded_language = Vec::new();
    let mut excluded_unread = Vec::new();
    let mut unsupported = Vec::new();
    for file in &candidates {
        if !ENV_SUPPORTED_EXTS.contains(&file.ext.as_str()) {
            excluded_language.push(file.rel.clone());
            unsupported.push(UnsupportedObservation {
                file: file.rel.clone(),
                construct: format!(".{} env reference extraction", file.ext),
                location: Some(CoverageLocation::path(&file.rel)),
            });
        } else if file.content_hash.is_none() {
            excluded_unread.push(file.rel.clone());
            unsupported.push(UnsupportedObservation {
                file: file.rel.clone(),
                construct: format!(".{} env reference source could not be read", file.ext),
                location: Some(CoverageLocation::path(&file.rel)),
            });
        } else {
            visited += 1;
        }
    }
    let mut certificate = base_certificate(
        project,
        input,
        "env",
        candidates.len() as u64,
        visited,
        Vec::new(),
    );
    if !excluded_language.is_empty() {
        certificate
            .excluded_files_by_reason
            .insert(CoverageReason::UnsupportedLanguage, excluded_language);
    }
    if !excluded_unread.is_empty() {
        certificate
            .excluded_files_by_reason
            .insert(CoverageReason::UnsupportedConstruct, excluded_unread);
    }
    certificate.unsupported = unsupported;
    certificate.dynamic_stops = env_dynamic_stops(input);
    certificate
        .unresolved_stops
        .extend(root_scope_trim_stop(input));
    certificate.extractor_capabilities = vec![capability(
        "codemap.runtime-env",
        "multi",
        &["env_dynamic_lookup_detection", "static_env_reference"],
    )];
    certificate
}

fn env_dynamic_stops(input: &RuntimeGroupObservationInput<'_>) -> Vec<CoverageStop> {
    let scope_paths: BTreeSet<&str> = input
        .scope_files
        .iter()
        .map(|file| file.rel.as_str())
        .collect();
    let mut seen = BTreeSet::new();
    input
        .scope_unknowns
        .iter()
        .filter(|unknown| unknown.kind == "env_dynamic_lookup")
        .filter(|unknown| {
            unknown
                .path
                .as_deref()
                .is_some_and(|path| scope_paths.contains(path))
        })
        .filter(|unknown| {
            seen.insert((
                unknown.path.clone(),
                unknown.line_start,
                unknown.reason.clone(),
            ))
        })
        .map(|unknown| CoverageStop {
            kind: CoverageReason::DynamicEnvLookup,
            location: unknown.path.as_ref().map(|path| CoverageLocation {
                path: path.clone(),
                line_start: unknown.line_start,
                line_end: unknown.line_start,
            }),
            missing_surface: Some(format!(
                "{}: {}; {}",
                unknown.kind, unknown.effect, unknown.reason
            )),
        })
        .collect()
}

/// The unknown detectors themselves own an unsupported boundary: they can only
/// flag gaps inside the grammars they understand, so the unknown inventory is
/// never a proven-complete gap map.
pub(super) fn unknowns_certificate(
    project: &Project,
    input: &RuntimeGroupObservationInput<'_>,
) -> CoverageCertificate {
    let mut visited = 0_u64;
    let mut excluded_unread = Vec::new();
    for file in input.scope_files {
        if file.content_hash.is_none() {
            excluded_unread.push(file.rel.clone());
        } else {
            visited += 1;
        }
    }
    let mut certificate = base_certificate(
        project,
        input,
        "unknowns",
        input.scope_files.len() as u64,
        visited,
        vec![CoverageReason::UnsupportedConstruct],
    );
    if !excluded_unread.is_empty() {
        certificate
            .excluded_files_by_reason
            .insert(CoverageReason::UnsupportedConstruct, excluded_unread);
    }
    certificate
        .unresolved_stops
        .extend(root_scope_trim_stop(input));
    certificate.extractor_capabilities = vec![capability(
        "codemap.runtime-unknown-detectors",
        "multi",
        &[
            "dynamic_import_detection",
            "env_dynamic_lookup_detection",
            "raw_sql_literal_detection",
            "route_gap_detection",
        ],
    )];
    certificate
}
