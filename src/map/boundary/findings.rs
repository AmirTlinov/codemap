// Responsibility: map-boundary-findings
use crate::map::{
    glob_match, package_edge_matches_rule, package_edge_touched, package_transitive_paths,
    resolve_domain_pattern, root_domain,
};
use crate::model::{BoundaryFinding, BoundaryReport, Project};
use std::collections::BTreeSet;
use std::path::Path;

pub fn boundary_report(
    project: &Project,
    changed_only: Option<&BTreeSet<String>>,
) -> BoundaryReport {
    BoundaryReport {
        kind: "boundary_report",
        schema_version: "3",
        findings: boundary_findings(project, changed_only),
    }
}

pub fn boundary_findings(
    project: &Project,
    changed_only: Option<&BTreeSet<String>>,
) -> Vec<BoundaryFinding> {
    let mut findings = Vec::new();
    let root_domain = root_domain(project);
    let semantic_anchor_changed = changed_only
        .map(|changed| {
            changed
                .iter()
                .any(|rel| is_semantic_anchor_path(project, rel))
        })
        .unwrap_or(false);
    let edge_scope = if semantic_anchor_changed {
        None
    } else {
        changed_only
    };
    for file in project.files.values() {
        if file.has_role("generated")
            && let Some(changed) = changed_only
            && changed.contains(&file.rel)
        {
            findings.push(BoundaryFinding {
                from: file.rel.clone(),
                to: String::new(),
                status: "forbidden".to_string(),
                reason: "generated file edited directly".to_string(),
                recovery: vec!["Edit the source input or generator, then regenerate.".to_string()],
                provenance: "heuristic".to_string(),
                strength: "medium".to_string(),
            });
        }
        for target in &file.resolved_imports {
            for rule in &project.anchors.boundaries.forbidden {
                if rule.from.is_empty() || rule.to.is_empty() {
                    continue;
                }
                let from = resolve_domain_pattern(&root_domain, &rule.from);
                let to = resolve_domain_pattern(&root_domain, &rule.to);
                if let Some(changed) = edge_scope
                    && !file_boundary_edge_touched(file, target, changed)
                {
                    continue;
                }
                if glob_match(&from, &file.rel) && glob_match(&to, target) {
                    findings.push(BoundaryFinding {
                        from: file.rel.clone(),
                        to: target.clone(),
                        status: rule
                            .status
                            .clone()
                            .unwrap_or_else(|| "forbidden".to_string()),
                        reason: rule.reason.clone(),
                        recovery: rule.recovery.clone(),
                        provenance: "semantic_anchor".to_string(),
                        strength: "hard".to_string(),
                    });
                }
            }
        }
    }
    for edge in &project.package_edges {
        if let Some(changed) = edge_scope
            && !package_edge_touched(edge, changed)
        {
            continue;
        }
        for rule in &project.anchors.boundaries.forbidden {
            if rule.from.is_empty() || rule.to.is_empty() {
                continue;
            }
            let from = resolve_domain_pattern(&root_domain, &rule.from);
            let to = resolve_domain_pattern(&root_domain, &rule.to);
            if package_edge_matches_rule(&from, &edge.from)
                && package_edge_matches_rule(&to, &edge.to)
            {
                let mut reason = rule.reason.clone();
                if !reason.is_empty() {
                    reason.push_str("; ");
                }
                reason.push_str(&format!(
                    "package manifest dependency `{}` from {}",
                    edge.dependency, edge.source
                ));
                findings.push(BoundaryFinding {
                    from: edge.from_manifest.clone(),
                    to: edge.to_manifest.clone().unwrap_or_else(|| edge.to.clone()),
                    status: rule
                        .status
                        .clone()
                        .unwrap_or_else(|| "forbidden".to_string()),
                    reason,
                    recovery: rule.recovery.clone(),
                    provenance: "package_manifest+semantic_anchor".to_string(),
                    strength: "hard".to_string(),
                });
            }
        }
    }
    for path in package_transitive_paths(project, 4) {
        if let Some(changed) = edge_scope
            && !path
                .manifests
                .iter()
                .any(|manifest| changed.contains(manifest))
        {
            continue;
        }
        for rule in &project.anchors.boundaries.forbidden {
            if rule.from.is_empty() || rule.to.is_empty() {
                continue;
            }
            let from = resolve_domain_pattern(&root_domain, &rule.from);
            let to = resolve_domain_pattern(&root_domain, &rule.to);
            if package_edge_matches_rule(&from, &path.from)
                && package_edge_matches_rule(&to, &path.to)
            {
                let mut reason = rule.reason.clone();
                if !reason.is_empty() {
                    reason.push_str("; ");
                }
                reason.push_str(&format!(
                    "transitive package manifest dependency path `{}`",
                    path.dependencies.join(" -> ")
                ));
                findings.push(BoundaryFinding {
                    from: path.from_manifest.clone(),
                    to: path.to_manifest.clone().unwrap_or_else(|| path.to.clone()),
                    status: rule
                        .status
                        .clone()
                        .unwrap_or_else(|| "forbidden".to_string()),
                    reason,
                    recovery: rule.recovery.clone(),
                    provenance: "package_manifest_transitive+semantic_anchor".to_string(),
                    strength: "hard".to_string(),
                });
            }
        }
    }
    findings
}

fn is_semantic_anchor_path(project: &Project, rel: &str) -> bool {
    project
        .files
        .get(rel)
        .map(|file| file.has_role("semantic_anchor"))
        .unwrap_or_else(|| {
            matches!(
                Path::new(rel).file_name().and_then(|name| name.to_str()),
                Some(".codemap.yml" | ".codemap.yaml" | ".codemap.json")
            )
        })
}

fn file_boundary_edge_touched(
    file: &crate::model::FileInfo,
    target: &str,
    changed: &BTreeSet<String>,
) -> bool {
    changed.contains(&file.rel) || changed.contains(target)
}
