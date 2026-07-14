// Responsibility: changed-surface-hints-section
use crate::model::ChangedReport;
use crate::render::{
    canonical_roles, changed_common_dir_prefix, changed_relative_path, changed_render_limit,
    changed_roles_for_path, changed_selector_suffix, disclaimer, root_aware_expand,
};

pub(crate) fn changed_roles_section(report: &ChangedReport, force: bool, compact: bool) {
    let mut paths_with_summaries = std::collections::BTreeSet::new();
    let mut grouped: std::collections::BTreeMap<String, Vec<(String, String)>> =
        std::collections::BTreeMap::new();
    for file in &report.changed {
        paths_with_summaries.insert(file.path.clone());
        for role in canonical_roles(file) {
            grouped.entry(role).or_default().push((
                file.path.clone(),
                format!("[{}; {}]", file.kind, file.language),
            ));
        }
    }
    for change in &report.git_state {
        if paths_with_summaries.contains(&change.path) {
            continue;
        }
        for role in changed_roles_for_path(&change.path) {
            grouped.entry(role).or_default().push((
                change.path.clone(),
                format!("[git; status={}]", change.status),
            ));
        }
    }
    if grouped.is_empty() {
        if force {
            println!("\n## Surface Hints\n");
            println!("No changed surface hints found.");
        }
        return;
    }
    for entries in grouped.values_mut() {
        entries.sort();
        entries.dedup();
    }
    println!("\n## Surface Hints\n");
    disclaimer(
        "Derived from deterministic path/name/extension/manifest/git patterns. Not change intent, correctness, or verification sufficiency.",
    );
    let paths = grouped
        .values()
        .flat_map(|entries| entries.iter().map(|(path, _)| path.as_str()))
        .collect::<Vec<_>>();
    let prefix = changed_common_dir_prefix(&paths);
    if let Some(prefix) = &prefix {
        println!("prefix: `{prefix}`");
    }
    let mut rendered = std::collections::BTreeSet::new();
    let compact_roles = compact_changed_roles(&grouped, compact);
    for role in CHANGED_ROLE_ORDER {
        if let Some(entries) = grouped.get(*role) {
            if compact && !compact_roles.contains(role) {
                continue;
            }
            changed_role_entries(
                role,
                entries,
                prefix.as_deref(),
                changed_render_limit(report, compact),
                compact,
            );
            rendered.insert((*role).to_string());
        }
    }
    for (role, entries) in grouped {
        if rendered.contains(&role) {
            continue;
        }
        if compact && !compact_roles.contains(role.as_str()) {
            continue;
        }
        changed_role_entries(
            &role,
            &entries,
            prefix.as_deref(),
            changed_render_limit(report, compact),
            compact,
        );
    }
    if compact && compact_roles.hidden > 0 {
        println!(
            "- hidden surface hint groups: `{}`; expand: `{}`",
            compact_roles.hidden,
            root_aware_expand(&format!(
                "codemap changed{} --section roles",
                changed_selector_suffix(&report.selector)
            ))
        );
    }
}

const COMPACT_CHANGED_ROLE_GROUP_LIMIT: usize = 12;

struct CompactChangedRoles {
    visible: std::collections::BTreeSet<String>,
    hidden: usize,
}

impl CompactChangedRoles {
    fn contains(&self, role: &str) -> bool {
        self.visible.contains(role)
    }
}

fn compact_changed_roles(
    grouped: &std::collections::BTreeMap<String, Vec<(String, String)>>,
    compact: bool,
) -> CompactChangedRoles {
    if !compact || grouped.len() <= COMPACT_CHANGED_ROLE_GROUP_LIMIT {
        return CompactChangedRoles {
            visible: grouped.keys().cloned().collect(),
            hidden: 0,
        };
    }
    let mut roles = grouped
        .iter()
        .map(|(role, entries)| {
            let order = CHANGED_ROLE_ORDER
                .iter()
                .position(|ordered| *ordered == role)
                .unwrap_or(usize::MAX);
            (role.clone(), entries.len(), order)
        })
        .collect::<Vec<_>>();
    roles.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| left.2.cmp(&right.2))
            .then_with(|| left.0.cmp(&right.0))
    });
    CompactChangedRoles {
        visible: roles
            .iter()
            .take(COMPACT_CHANGED_ROLE_GROUP_LIMIT)
            .map(|(role, _, _)| role.clone())
            .collect(),
        hidden: grouped.len() - COMPACT_CHANGED_ROLE_GROUP_LIMIT,
    }
}

const CHANGED_ROLE_ORDER: &[&str] = &[
    "source",
    "application",
    "service",
    "domain",
    "controller",
    "module",
    "repository",
    "test",
    "schema",
    "manifest",
    "env",
    "config",
    "lockfile",
    "docs",
    "public_boundary",
    "contract_doc",
    "ci",
    "script",
    "package_graph",
    "role_classifier",
    "script_catalog",
    "cli_surface",
    "map_surface",
    "extractor",
    "config_loader",
    "evidence_surface",
    "fixture",
    "generated",
    "archive",
    "witness",
    "receipt",
    "proof_runner",
    "proof_rail",
    "build_output",
    "unknown",
];

fn changed_role_entries(
    role: &str,
    entries: &[(String, String)],
    prefix: Option<&str>,
    limit: usize,
    compact: bool,
) {
    if compact {
        let sample = entries
            .iter()
            .take(3)
            .map(|(path, _)| format!("`{}`", changed_relative_path(path, prefix)))
            .collect::<Vec<_>>()
            .join(", ");
        let hidden = entries.len().saturating_sub(3);
        if sample.is_empty() {
            println!("- `{role}`: `{}`", entries.len());
        } else if hidden > 0 {
            println!(
                "- `{role}`: `{}`; sample: {sample}; hidden: `{hidden}` surfaces",
                entries.len()
            );
        } else {
            println!("- `{role}`: `{}`; sample: {sample}", entries.len());
        }
        return;
    }
    println!("- `{role}`: `{}`", entries.len());
    for (path, meta) in entries.iter().take(limit) {
        println!("  - `{}` {}", changed_relative_path(path, prefix), meta);
    }
    let hidden = entries.len().saturating_sub(limit);
    if hidden > 0 {
        println!("  - hidden: `{hidden}` surfaces");
    }
}
