// Responsibility: cache-module-root
pub(crate) mod artifact_status;
pub(crate) mod cached_project;
pub(crate) mod fingerprint_delta;
pub(crate) mod fingerprints;
pub(crate) mod git_probe;
pub(crate) mod identity;
pub(crate) mod lens_artifacts;
pub(crate) mod runtime_root;
pub(crate) mod snapshots;
pub(crate) mod status_artifacts;

pub use artifact_status::{
    artifact_statuses, cache_state, cached_status_fingerprint, stale_lens_artifact_examples,
};
pub use cached_project::read_cached_project;
pub(crate) use fingerprints::format_version as fingerprint_format_version;
pub use fingerprints::{
    SnapshotDelta, cached_git_head, cached_git_head_matches, file_delta,
    file_delta_by_rechecking_cached_files, file_delta_for_head_change,
    file_delta_for_known_changes, snapshot_delta,
};
pub(crate) use identity::hex_prefix;
pub use identity::{
    cache_enabled, expected_artifacts, fingerprint, inventory_fingerprint, project_cache_dir,
    runtime_scope_fingerprint, runtime_scope_has_unindexed_entries,
    runtime_scope_is_logically_empty,
};
pub(crate) use lens_artifacts::format_version as lens_artifact_format_version;
pub use lens_artifacts::{
    ConeLensKey, LsLensKey, PlaceLensKey, SiblingsLensKey, read_changed_report, read_cone_report,
    read_inventory_ls_report, read_ls_report, read_place_report, read_proof_changed_report,
    read_proof_map_report, read_siblings_report, write_changed_report, write_cone_report,
    write_inventory_ls_report, write_ls_report, write_place_report, write_proof_changed_report,
    write_proof_map_report, write_siblings_report,
};
pub use runtime_root::read_runtime_root_report;
pub use snapshots::{SnapshotMetadata, looks_like_snapshot_token, metadata as snapshot_metadata};
pub(crate) use status_artifacts::CachedDomain;
pub use status_artifacts::write_status_with_change_sets;
