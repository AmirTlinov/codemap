// Responsibility: runtime-lens-path-conventions
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeExternalPathKind {
    Container,
    ConcreteFile,
}

pub(crate) fn runtime_worker_or_job_convention(rel: &str) -> bool {
    let path = std::path::Path::new(rel);
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(file_name)
        .to_ascii_lowercase();
    if exact_runtime_worker_job_token(&stem)
        || split_runtime_tokens(&stem)
            .iter()
            .any(|token| exact_runtime_worker_job_token(token))
    {
        return true;
    }
    path.parent()
        .map(|parent| {
            parent
                .components()
                .filter_map(|component| component.as_os_str().to_str())
                .any(|segment| exact_runtime_worker_job_token(&segment.to_ascii_lowercase()))
        })
        .unwrap_or(false)
}

/// Classifies a non-followed external node from repository-owned path evidence.
///
/// Target metadata is deliberately excluded: changing an external symlink
/// target between a file and directory cannot change repository facts. A
/// filename extension or a canonical extensionless build/CI filename is the
/// positive evidence required to retain an exact path fact; every ambiguous
/// extensionless node remains a container boundary.
pub(crate) fn runtime_external_path_kind(rel: &str) -> RuntimeExternalPathKind {
    let normalized = rel.trim_matches('/').to_ascii_lowercase();
    let path = Path::new(&normalized);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    let has_extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| !extension.is_empty());
    if has_extension || crate::repo::is_known_build_ci_name(name) {
        RuntimeExternalPathKind::ConcreteFile
    } else {
        RuntimeExternalPathKind::Container
    }
}

fn split_runtime_tokens(value: &str) -> Vec<String> {
    value
        .split(|ch: char| !(ch.is_ascii_alphanumeric()))
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

fn exact_runtime_worker_job_token(value: &str) -> bool {
    matches!(
        value,
        "worker" | "workers" | "job" | "jobs" | "cron" | "crons"
    )
}
