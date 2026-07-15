// Responsibility: map-proof-entry-impact-report
use crate::map::{
    BoundedProjection, file_summary, impact_cluster, impact_expand_commands, missing_file_summary,
    shell_quote, unknown_unindexed_anchor,
};
use crate::model::{HiddenGroup, ImpactCluster, ImpactReport, Project};
use crate::repo;

pub fn impact_report(
    project: &Project,
    changed: Vec<String>,
    selector: String,
    depth: usize,
    limit: usize,
) -> ImpactReport {
    let limit = limit.max(1);
    let changed = changed
        .into_iter()
        .map(|file| repo::normalize_rel_path(&file))
        .filter(|file| file != ".")
        .collect::<Vec<_>>();
    let selector = normalized_impact_selector(&changed, selector);
    let mut hidden = Vec::new();
    let mut unknowns = Vec::new();
    let mut changed_summaries = Vec::new();
    let mut cluster_reports = Vec::new();
    let changed_count = changed.len();
    let visible_changed = BoundedProjection::ordered(
        "changed anchors hidden by limit",
        changed.clone(),
        limit,
        &impact_hidden_changed_expand(&selector, depth, changed_count),
    )
    .into_shown()
    .into_iter()
    .collect::<std::collections::BTreeSet<_>>();
    for rel in &changed {
        if let Some(file) = project.files.get(rel) {
            if visible_changed.contains(rel) {
                changed_summaries.push(file_summary(project, file, false, 12));
                let (cluster, cluster_hidden) = impact_cluster(project, rel, depth, limit);
                cluster_reports.push((cluster, cluster_hidden));
            }
        } else {
            unknowns.push(unknown_unindexed_anchor(rel));
            if visible_changed.contains(rel) {
                changed_summaries.push(missing_file_summary(project, rel));
                cluster_reports.push((
                    ImpactCluster {
                        id: format!("changed:{rel}"),
                        changed: vec![rel.clone()],
                        direct_consumers: Vec::new(),
                        cross_boundary_consumers: Vec::new(),
                        contract_links: Vec::new(),
                        proof: Vec::new(),
                        reasons: vec!["changed file is not indexed".to_string()],
                    },
                    Vec::new(),
                ));
            }
        }
    }
    if changed_count > limit {
        hidden.push(HiddenGroup {
            reason: "changed anchors hidden by limit".to_string(),
            count: changed_count - limit,
            expand: impact_hidden_changed_expand(&selector, depth, changed_count),
        });
        hidden.push(HiddenGroup {
            reason: "impact clusters hidden by limit".to_string(),
            count: changed_count - limit,
            expand: impact_hidden_changed_expand(&selector, depth, changed_count),
        });
    }
    let mut clusters = Vec::new();
    for (cluster, cluster_hidden) in cluster_reports.into_iter().take(limit) {
        hidden.extend(cluster_hidden);
        clusters.push(cluster);
    }
    ImpactReport {
        kind: "impact_report",
        schema_version: "7",
        selector: selector.clone(),
        changed: changed_summaries,
        clusters,
        hidden,
        unknowns,
        expand: impact_expand_commands(&changed, &selector),
    }
}

fn normalized_impact_selector(changed: &[String], selector: String) -> String {
    if !selector.trim().is_empty() {
        return selector;
    }
    if changed.is_empty() {
        return String::new();
    }
    let files = changed
        .iter()
        .map(|file| shell_quote(file))
        .collect::<Vec<_>>()
        .join(",");
    format!("--files {files}")
}

fn impact_hidden_changed_expand(selector: &str, depth: usize, limit: usize) -> String {
    let selector = selector.trim();
    if selector.is_empty() {
        return format!("codemap impact --changed --depth {depth} --limit {limit}");
    }
    format!("codemap impact {selector} --depth {depth} --limit {limit}")
}
