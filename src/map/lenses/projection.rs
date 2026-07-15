// Responsibility: shared-bounded-fact-projection
use crate::map::expand_with_concrete_limit;
use crate::model::HiddenGroup;
use std::collections::BTreeSet;

pub(crate) struct BoundedProjection<T> {
    group: String,
    shown: Vec<T>,
    observed: usize,
    expand: String,
}

impl<T> BoundedProjection<T> {
    pub(crate) fn selected(group: &str, observed: usize, shown: Vec<T>, expand: &str) -> Self {
        debug_assert!(shown.len() <= observed);
        Self {
            group: group.to_string(),
            shown,
            observed,
            expand: expand.to_string(),
        }
    }

    pub(crate) fn ordered(group: &str, mut values: Vec<T>, quota: usize, expand: &str) -> Self {
        let observed = values.len();
        values.truncate(quota);
        Self::selected(group, observed, values, expand)
    }

    pub(crate) fn by_identity<K: Ord, F: FnMut(&T) -> K>(
        group: &str,
        mut values: Vec<T>,
        quota: usize,
        expand: &str,
        mut identity: F,
    ) -> Self {
        values.sort_by_key(|value| identity(value));
        let mut seen = BTreeSet::new();
        values.retain(|value| seen.insert(identity(value)));
        Self::ordered(group, values, quota, expand)
    }

    #[cfg(test)]
    pub(crate) fn observed(&self) -> usize {
        self.observed
    }

    #[cfg(test)]
    pub(crate) fn shown(&self) -> &[T] {
        &self.shown
    }

    #[cfg(test)]
    pub(crate) fn hidden(&self) -> usize {
        self.observed.saturating_sub(self.shown.len())
    }

    pub(crate) fn into_parts(self) -> (Vec<T>, Option<HiddenGroup>) {
        let hidden = self.observed.saturating_sub(self.shown.len());
        let hidden_group = (hidden > 0).then(|| HiddenGroup {
            reason: self.group,
            count: hidden,
            expand: expand_with_concrete_limit(&self.expand, self.observed),
        });
        (self.shown, hidden_group)
    }

    pub(crate) fn into_shown(self) -> Vec<T> {
        self.shown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_projection_is_deterministic_monotonic_deduplicated_and_exact() {
        let values = vec![5, 2, 2, 4, 1, 3];
        let small = BoundedProjection::by_identity(
            "numbers hidden",
            values.clone(),
            2,
            "codemap ls .",
            |value| *value,
        );
        let large = BoundedProjection::by_identity(
            "numbers hidden",
            values.into_iter().rev().collect(),
            4,
            "codemap ls .",
            |value| *value,
        );
        assert_eq!(small.shown(), &[1, 2]);
        assert_eq!(large.shown(), &[1, 2, 3, 4]);
        assert_eq!(small.observed(), 5);
        assert_eq!(small.shown().len() + small.hidden(), small.observed());
        assert!(large.shown().starts_with(small.shown()));
    }

    #[test]
    fn empty_group_does_not_create_hidden_noise() {
        let projection =
            BoundedProjection::<String>::ordered("empty hidden", Vec::new(), 1, "codemap ls .");
        let (shown, hidden) = projection.into_parts();
        assert!(shown.is_empty());
        assert!(hidden.is_none());
    }

    #[test]
    fn expand_preserves_anchor_selector_and_output_mode() {
        let projection = BoundedProjection::ordered(
            "definitions hidden",
            vec!["a", "b", "c"],
            1,
            "codemap where token --kind function --format json",
        );
        let (_, hidden) = projection.into_parts();
        assert_eq!(
            hidden.expect("hidden group").expand,
            "codemap where token --kind function --format json --limit 3"
        );
    }
}
