// Responsibility: map-listing-ls-root-horizons
use crate::map::{
    RootInventoryGroupVisibility, RootInventoryObservationInput, files_under_directory,
    record_root_inventory_observations, root_script_manifest_partition,
};
use crate::model::{DirectorySurface, LsReport, ObservationLedger, Project};

/// Group fact counts owned by full-index root `ls .` assembly, captured
/// before the projection truncates its surfaces.
pub(crate) struct RootLsGroupCounts {
    pub surface_total: usize,
    pub packages_observed: usize,
    pub scripts_observed: usize,
    pub tests_observed: usize,
    pub current_level_entries: usize,
    pub classified_entries: usize,
}

/// Converts the full-index root projection into the four certified
/// root-inventory horizons. Shown facts are read from the visible surfaces
/// themselves, so the ledger can never drift from the projection.
pub(crate) fn root_ls_observations(
    project: &Project,
    counts: &RootLsGroupCounts,
    surfaces: &[DirectorySurface],
) -> ObservationLedger {
    let package_audit = crate::repo::audit_package_discovery(&project.root, &project.files);
    let package_manifest_unsupported = package_audit
        .unsupported
        .into_iter()
        .map(package_discovery_gap_observation)
        .collect();
    let test_surface_unsupported = root_test_surface_gaps(project);
    let (script_manifests_visited, script_manifests_excluded) = root_script_manifest_partition(
        files_under_directory(project, ".")
            .into_iter()
            .map(|file| file.rel.as_str()),
    );
    let mut ledger = ObservationLedger::default();
    record_root_inventory_observations(
        RootInventoryObservationInput {
            snapshot: crate::cache::fingerprint(project, None),
            classified_entries: counts.classified_entries as u64,
            current_level_entries: counts.current_level_entries as u64,
            package_manifest_candidates: package_audit
                .candidates
                .into_iter()
                .map(|candidate| candidate.manifest)
                .collect(),
            package_manifests_visited: package_audit.visited_manifests,
            package_manifest_unsupported,
            script_manifests_visited,
            script_manifests_excluded,
            full_index: true,
            complete_current_level_atlas: true,
            directory_surfaces: group_visibility(counts.surface_total, surfaces.len()),
            packages: group_visibility(
                counts.packages_observed,
                shown_surface_facts(surfaces, "packages"),
            ),
            scripts: group_visibility(
                counts.scripts_observed,
                shown_surface_facts(surfaces, "scripts"),
            ),
            tests: group_visibility(
                counts.tests_observed,
                shown_surface_facts(surfaces, "test_surfaces"),
            ),
            test_surface_unsupported,
        },
        &mut ledger,
    );
    ledger
}

pub(crate) fn package_discovery_gap_observation(
    gap: crate::repo::PackageDiscoveryGap,
) -> crate::model::UnsupportedObservation {
    crate::model::UnsupportedObservation {
        file: gap.manifest.clone(),
        construct: format!("{} package discovery: {}", gap.ecosystem, gap.construct),
        location: Some(crate::model::CoverageLocation::path(gap.manifest)),
    }
}

/// The projection communicates group facts through visible surface counts;
/// this mirrors `LsReport::validate_observations` exactly.
pub(crate) fn shown_surface_facts(surfaces: &[DirectorySurface], group: &str) -> usize {
    surfaces
        .iter()
        .filter(|surface| match group {
            "packages" => {
                surface.kind.starts_with("package:") || surface.kind.starts_with("support_package:")
            }
            "scripts" => surface.kind == "script",
            _ => LsReport::TEST_SURFACE_KINDS.contains(&surface.kind.as_str()),
        })
        .map(|surface| surface.examples.len())
        .sum()
}

fn root_test_surface_gaps(project: &Project) -> Vec<crate::model::UnsupportedObservation> {
    project
        .files
        .values()
        .filter(|file| file.rel.matches('/').count() <= 1)
        .filter(|file| crate::repo::is_source_ext(&file.ext))
        .filter(|file| !file.has_role("test") && !file.has_role("test_support"))
        .filter(|file| {
            file.content_hash.is_none()
                || crate::repo::is_test_path(&file.rel.to_ascii_lowercase(), &file.ext)
        })
        .map(|file| crate::model::UnsupportedObservation {
            file: file.rel.clone(),
            construct: if file.content_hash.is_none() {
                "test-role source body unavailable".to_string()
            } else {
                "test-path candidate not classified".to_string()
            },
            location: Some(crate::model::CoverageLocation::path(&file.rel)),
        })
        .collect()
}

pub(crate) fn group_visibility(observed: usize, shown: usize) -> RootInventoryGroupVisibility {
    RootInventoryGroupVisibility {
        observed,
        shown,
        expand: (observed > shown).then(|| "codemap ls . --all".to_string()),
    }
}
