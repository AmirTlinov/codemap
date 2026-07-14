// Responsibility: changed-map-delta
use crate::map::count_with_hidden;
use crate::model::{ChangedMapDelta, DiffMapReport};

pub(crate) fn changed_map_delta_from_diff(diff: &DiffMapReport) -> ChangedMapDelta {
    ChangedMapDelta {
        added_edges: count_with_hidden(
            diff.added_edges.len(),
            &diff.hidden,
            "added structural edges hidden by limit",
        ),
        removed_edges: count_with_hidden(
            diff.removed_edges.len(),
            &diff.hidden,
            "removed structural edges hidden by limit",
        ),
        changed_symbols: count_with_hidden(
            diff.changed_symbols.len(),
            &diff.hidden,
            "changed symbol surfaces hidden by limit",
        ),
        added_exports: count_with_hidden(
            diff.added_exports.len(),
            &diff.hidden,
            "added export surfaces hidden by limit",
        ),
        removed_exports: count_with_hidden(
            diff.removed_exports.len(),
            &diff.hidden,
            "removed export surfaces hidden by limit",
        ),
        added_runtime_routes: count_with_hidden(
            diff.added_runtime_routes.len(),
            &diff.hidden,
            "added runtime routes hidden by limit",
        ),
        removed_runtime_routes: count_with_hidden(
            diff.removed_runtime_routes.len(),
            &diff.hidden,
            "removed runtime routes hidden by limit",
        ),
        added_env: count_with_hidden(
            diff.added_env.len(),
            &diff.hidden,
            "added env dependencies hidden by limit",
        ),
        removed_env: count_with_hidden(
            diff.removed_env.len(),
            &diff.hidden,
            "removed env dependencies hidden by limit",
        ),
        added_proof_surfaces: count_with_hidden(
            diff.added_proof_surfaces.len(),
            &diff.hidden,
            "added verification surfaces hidden by limit",
        ),
        removed_proof_surfaces: count_with_hidden(
            diff.removed_proof_surfaces.len(),
            &diff.hidden,
            "removed verification surfaces hidden by limit",
        ),
        new_unknowns: count_with_hidden(
            diff.new_unknowns.len(),
            &diff.hidden,
            "new unknowns hidden by limit",
        ),
    }
}
