// Responsibility: changed-git-status-events
use crate::map::shell_quote;
use crate::model::{EvidenceLocation, GitChange};

pub(crate) fn changed_structural_events(
    git_state: &[GitChange],
    selector: &str,
) -> Vec<crate::model::ChangedStructuralEvent> {
    let mut events = git_state
        .iter()
        .filter_map(|change| changed_structural_event(change, selector))
        .collect::<Vec<_>>();
    sort_changed_structural_events(&mut events);
    events
}

pub(crate) fn sort_changed_structural_events(events: &mut [crate::model::ChangedStructuralEvent]) {
    events.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.old_path.cmp(&b.old_path))
    });
}

fn changed_structural_event(
    change: &GitChange,
    selector: &str,
) -> Option<crate::model::ChangedStructuralEvent> {
    let (kind, effect, location_kind, expand) = match change.status.as_str() {
        "deleted" => (
            "removed_anchor",
            "path was removed from the working tree; inspect removed edges and exports",
            "git_deleted",
            Some(format!("codemap diff-map {selector}")),
        ),
        "renamed" => (
            "renamed_anchor",
            "path moved; old-path consumers may still point at the previous anchor",
            "git_renamed",
            Some(format!("codemap cone {}", shell_quote(&change.path))),
        ),
        "typechanged" => (
            "typechanged_anchor",
            "path type changed; structural facts may need a fresh exact anchor check",
            "git_typechanged",
            Some(format!("codemap ls {}", shell_quote(&change.path))),
        ),
        "conflicted" => (
            "conflicted_anchor",
            "merge conflict prevents a stable structural map for this path",
            "git_conflicted",
            Some(format!("codemap ls {}", shell_quote(&change.path))),
        ),
        _ => return None,
    };
    Some(crate::model::ChangedStructuralEvent {
        kind: kind.to_string(),
        path: change.path.clone(),
        old_path: change.old_path.clone(),
        evidence: "git_status".to_string(),
        effect: effect.to_string(),
        locations: vec![EvidenceLocation::path(&change.path, location_kind)],
        expand,
    })
}
