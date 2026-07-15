// Responsibility: runtime-group-scan-boundary-provenance
use crate::model::{
    CoverageCertificate, CoverageClosure, CoverageLocation, CoverageReason, CoverageStop,
    IndexedBoundary, ScanInventoryBoundary,
};

use super::RuntimeGroupObservationInput;

pub(super) fn apply_scan_boundary_stops(
    certificate: &mut CoverageCertificate,
    input: &RuntimeGroupObservationInput<'_>,
    group: &str,
) {
    certificate
        .external_stops
        .extend(input.external_boundaries.iter().map(|file| CoverageStop {
            kind: CoverageReason::IncompleteTraversal,
            location: Some(CoverageLocation::path(&file.rel)),
            missing_surface: Some(format!(
                "non-followed external tree may hide additional {group} facts"
            )),
        }));
    certificate.unresolved_stops.extend(
        input
            .incomplete_boundaries
            .iter()
            .filter(|file| file.indexed_boundary == Some(IndexedBoundary::TraversalError))
            .map(|file| CoverageStop {
                kind: CoverageReason::IncompleteTraversal,
                location: Some(CoverageLocation::path(&file.rel)),
                missing_surface: Some(format!(
                    "repository traversal failed before additional {group} facts were indexed"
                )),
            }),
    );
    for boundary in input.scan_boundaries {
        certificate.unresolved_stops.push(match boundary {
            ScanInventoryBoundary::FilesystemTraversalUnavailable => CoverageStop {
                kind: CoverageReason::IncompleteTraversal,
                location: Some(CoverageLocation::path(input.scope)),
                missing_surface: Some(format!(
                    "repository traversal failed without a path; hidden {group} facts are unknown"
                )),
            },
            ScanInventoryBoundary::GitIndexUnavailable => CoverageStop {
                kind: CoverageReason::IncompleteTraversal,
                location: Some(CoverageLocation::path(".git/index")),
                missing_surface: Some(format!(
                    "Git index inventory was unavailable; hidden tracked {group} facts are unknown"
                )),
            },
        });
    }
    if input.scope_has_unindexed_entries {
        certificate.unresolved_stops.push(CoverageStop {
            kind: CoverageReason::IncompleteTraversal,
            location: Some(CoverageLocation::path(input.scope)),
            missing_surface: Some(
                "physical scope entries exist outside indexed runtime candidates".to_string(),
            ),
        });
    }
    if !certificate.external_stops.is_empty() || !certificate.unresolved_stops.is_empty() {
        certificate.closure = CoverageClosure::Open;
    }
}
