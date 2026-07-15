// Responsibility: changed-session-snapshot-public-contract
use crate::model::{Project, SessionSnapshot};

pub(crate) fn current_session_snapshot(project: &Project) -> SessionSnapshot {
    let token = crate::cache::fingerprint(project, None);
    crate::cache::snapshot_metadata(&project.cache_dir, &token)
        .map(session_snapshot_from_metadata)
        .unwrap_or_else(|| unavailable_snapshot(token))
}

pub(crate) fn session_snapshot_from_metadata(
    metadata: crate::cache::SnapshotMetadata,
) -> SessionSnapshot {
    SessionSnapshot {
        reuse: format!("codemap changed --since {}", metadata.token),
        token: metadata.token,
        created_unix_seconds: Some(metadata.created_unix_seconds),
        file_count: metadata.file_count,
        content_files: metadata.content_files,
        storage: metadata.storage,
        freshness: "exact".to_string(),
    }
}

fn unavailable_snapshot(token: String) -> SessionSnapshot {
    SessionSnapshot {
        reuse: format!("codemap changed --since {token}"),
        token,
        created_unix_seconds: None,
        file_count: 0,
        content_files: 0,
        storage: "external_cache".to_string(),
        freshness: "unavailable".to_string(),
    }
}
