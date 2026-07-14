// Responsibility: runtime-lens-proof-map-limits
pub(crate) fn proof_map_discovery_limit(
    scope: Option<&str>,
    changed: &[String],
    limit: usize,
    raw_sensors: bool,
) -> usize {
    if raw_sensors || scope.is_some() || changed.is_empty() {
        usize::MAX
    } else {
        limit.saturating_mul(2).max(6)
    }
}
