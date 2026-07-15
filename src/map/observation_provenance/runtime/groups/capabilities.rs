// Responsibility: runtime-group-candidate-and-capability-declarations
use std::path::Path;

use crate::map::{runtime_entrypoint_kind, runtime_manifest_entrypoint_candidate};
use crate::model::{CoverageReason, ExtractorCapability, FileInfo};

pub(super) fn entrypoint_candidate(file: &FileInfo) -> bool {
    source_candidate(file)
        || crate::repo::is_script_ext(&file.ext)
        || runtime_manifest_entrypoint_candidate(file)
}

pub(super) fn entrypoint_capability(
    file: &FileInfo,
) -> Result<ExtractorCapability, (CoverageReason, String)> {
    readable(file)?;
    let name = file_name(file).unwrap_or_default();
    if matches!(name, "package.json" | "Cargo.toml" | "pyproject.toml") {
        return Ok(capability(
            "codemap.runtime-manifest-entrypoints",
            "manifest",
            &["manifest_cli_entrypoint"],
        ));
    }
    if file.ext.eq_ignore_ascii_case("rs") {
        return Ok(capability(
            "codemap.runtime-code-entrypoints",
            "rust",
            &["file_entrypoint_convention", "rust_clap_subcommand"],
        ));
    }
    if runtime_entrypoint_kind(file).is_some() {
        return Ok(capability(
            "codemap.runtime-entrypoint-paths",
            &file.language,
            &["file_entrypoint_convention"],
        ));
    }
    Err((
        if matches!(file.ext.as_str(), "swift" | "kt" | "java" | "rb") {
            CoverageReason::UnsupportedLanguage
        } else {
            CoverageReason::UnsupportedConstruct
        },
        format!(".{} runtime entrypoint extraction", file.ext),
    ))
}

pub(super) fn script_candidate(file: &FileInfo) -> bool {
    matches!(
        file_name(file),
        Some(
            "package.json"
                | "Cargo.toml"
                | "go.mod"
                | "pyproject.toml"
                | "requirements.txt"
                | "Package.swift"
                | "Makefile"
                | "makefile"
                | "GNUmakefile"
                | "Justfile"
                | "justfile"
        )
    )
}

pub(super) fn script_capability(
    scope: &str,
    file: &FileInfo,
) -> Result<ExtractorCapability, (CoverageReason, String)> {
    readable(file)?;
    if scope != "." || file.rel.contains('/') {
        return Err((
            CoverageReason::UnsupportedConstruct,
            "script catalog extraction is root-manifest-only".to_string(),
        ));
    }
    Ok(capability(
        "codemap.root-script-catalog",
        "manifest",
        &["validation_script_inventory"],
    ))
}

pub(super) fn env_capability(
    file: &FileInfo,
) -> Result<ExtractorCapability, (CoverageReason, String)> {
    readable(file)?;
    if !matches!(
        file.ext.as_str(),
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "py" | "rs"
    ) {
        return Err((
            CoverageReason::UnsupportedLanguage,
            format!(".{} static environment reference extraction", file.ext),
        ));
    }
    Ok(capability(
        "codemap.static-runtime-env",
        &file.language,
        &["static_environment_key"],
    ))
}

pub(super) fn relation_capability(
    file: &FileInfo,
) -> Result<ExtractorCapability, (CoverageReason, String)> {
    readable(file)?;
    if !matches!(
        file.ext.as_str(),
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "py" | "rs" | "go"
    ) {
        return Err((
            CoverageReason::UnsupportedLanguage,
            format!(".{} runtime verification relation extraction", file.ext),
        ));
    }
    Ok(capability(
        "codemap.runtime-route-verification",
        &file.language,
        &["static_route_reference", "static_route_visit"],
    ))
}

pub(super) fn unknown_capability(
    file: &FileInfo,
) -> Result<ExtractorCapability, (CoverageReason, String)> {
    readable(file)?;
    if !matches!(
        file.ext.as_str(),
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "py" | "rs" | "go"
    ) {
        return Err((
            CoverageReason::UnsupportedLanguage,
            format!(".{} runtime unknown detection", file.ext),
        ));
    }
    Ok(capability(
        "codemap.runtime-unknowns",
        &file.language,
        &["dynamic_import", "dynamic_env", "dynamic_route"],
    ))
}

pub(super) fn ci_capability(
    file: &FileInfo,
) -> Result<ExtractorCapability, (CoverageReason, String)> {
    if file.content_hash.is_none()
        && matches!(file.ext.as_str(), "yml" | "yaml")
        && !file.has_role("build_ci")
    {
        return Err((
            CoverageReason::UnsupportedConstruct,
            "unread YAML may carry content-derived CI role evidence".to_string(),
        ));
    }
    Ok(capability(
        "codemap.indexed-build-ci-roles",
        "indexed-role",
        &["build_ci_role"],
    ))
}

pub(super) fn source_candidate(file: &FileInfo) -> bool {
    crate::repo::is_source_ext(&file.ext)
}

fn readable(file: &FileInfo) -> Result<(), (CoverageReason, String)> {
    if file.content_hash.is_some() {
        Ok(())
    } else {
        Err((
            CoverageReason::UnsupportedConstruct,
            "runtime source content is unavailable".to_string(),
        ))
    }
}

fn file_name(file: &FileInfo) -> Option<&str> {
    Path::new(&file.rel).file_name()?.to_str()
}

pub(super) fn capability(id: &str, language: &str, constructs: &[&str]) -> ExtractorCapability {
    ExtractorCapability {
        extractor_id: id.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        language: language.to_string(),
        constructs: constructs.iter().map(|value| value.to_string()).collect(),
    }
}
