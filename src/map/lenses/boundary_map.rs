// Responsibility: boundary-map-lens
use crate::map::{
    boundary_findings, domain_by_rel, file_summary, files_under_directory, import_edge,
    is_support_artifact_path, limit_edge_section, package_for_rel, path_under_scope, shell_quote,
    truncate_with_hidden,
};
use crate::model::{
    BoundaryFinding, BoundaryMapReport, DomainRef, EvidenceStrength, HiddenGroup,
    PackageDependency, Project,
};
use crate::repo;
use std::collections::BTreeSet;

pub fn boundary_map_report(
    project: &Project,
    scope: &str,
    changed: Option<&BTreeSet<String>>,
    include_hidden: bool,
    limit: usize,
) -> BoundaryMapReport {
    let limit = limit.max(1);
    let scope = repo::normalize_rel_path(scope);
    let changed_flag = if changed.is_some() { " --changed" } else { "" };
    let include_hidden_expand = format!(
        "codemap boundary-map {}{changed_flag} --all",
        shell_quote(&scope)
    );
    let scope_is_support = is_support_artifact_path(&scope);
    let hide_support = !include_hidden && !scope_is_support;
    let changed_has_support = changed.is_some_and(|changed| {
        changed
            .iter()
            .any(|path| is_support_artifact_path(path.as_str()))
    });
    let scope_files = files_under_directory(project, &scope);
    let mut support_hidden_count = 0;

    let mut actual_cross_edges = Vec::new();
    let mut test_only_crossings = Vec::new();
    for file in &scope_files {
        for target in &file.resolved_imports {
            let paths = [file.rel.as_str(), target.as_str()];
            if !changed_touches_any(changed, &paths) {
                continue;
            }
            let cross = domain_by_rel(project, &file.rel).map(|domain| domain.path.clone())
                != domain_by_rel(project, target).map(|domain| domain.path.clone())
                || package_for_rel(project, &file.rel).map(|package| package.path.clone())
                    != package_for_rel(project, target).map(|package| package.path.clone());
            if cross {
                if support_fact_hidden(hide_support, changed, &paths) {
                    support_hidden_count += 1;
                    continue;
                }
                let edge = import_edge(
                    project,
                    file.rel.clone(),
                    target.clone(),
                    "cross_boundary_import",
                    "resolved_import_cross_boundary",
                    EvidenceStrength::High,
                );
                if file.has_role("test") || file.has_role("test_support") {
                    test_only_crossings.push(edge);
                } else {
                    actual_cross_edges.push(edge);
                }
            }
        }
    }
    let mut public_boundary_files = scope_files
        .iter()
        .filter(|file| file.has_role("public_boundary") && changed_touches(changed, &file.rel))
        .filter_map(|file| {
            let paths = [file.rel.as_str()];
            if support_fact_hidden(hide_support, changed, &paths) {
                support_hidden_count += 1;
                None
            } else {
                Some(file_summary(project, file, false, 12))
            }
        })
        .collect::<Vec<_>>();
    let mut package_edges = project
        .package_edges
        .iter()
        .filter(|edge| path_under_scope(&edge.from_manifest, &scope))
        .filter_map(|edge| {
            let paths = package_edge_paths(edge);
            if !changed_touches_any(changed, &paths) {
                return None;
            }
            if support_fact_hidden(hide_support, changed, &paths) {
                support_hidden_count += 1;
                None
            } else {
                Some(edge.clone())
            }
        })
        .collect::<Vec<_>>();
    let mut explicit_forbidden_findings = boundary_findings(project, changed)
        .into_iter()
        .filter(|finding| finding_touches_scope(finding, &scope))
        .filter_map(|finding| {
            let paths = [finding.from.as_str(), finding.to.as_str()];
            if support_fact_hidden(hide_support, changed, &paths) {
                support_hidden_count += 1;
                None
            } else {
                Some(finding)
            }
        })
        .collect::<Vec<_>>();
    let mut hidden = Vec::new();
    if support_hidden_count > 0 {
        hidden.push(HiddenGroup {
            reason: "support boundary artifacts hidden".to_string(),
            count: support_hidden_count,
            expand: include_hidden_expand.clone(),
        });
    }
    limit_edge_section(
        &mut actual_cross_edges,
        &mut hidden,
        include_hidden,
        limit,
        "actual cross-boundary edges hidden by limit",
        &include_hidden_expand,
    );
    limit_edge_section(
        &mut test_only_crossings,
        &mut hidden,
        include_hidden,
        limit,
        "test-only boundary crossings hidden by limit",
        &include_hidden_expand,
    );
    truncate_with_hidden(
        &mut public_boundary_files,
        limit,
        &mut hidden,
        "public boundary files hidden by limit",
        &include_hidden_expand,
    );
    truncate_with_hidden(
        &mut package_edges,
        limit,
        &mut hidden,
        "package edges hidden by limit",
        &include_hidden_expand,
    );
    truncate_with_hidden(
        &mut explicit_forbidden_findings,
        limit,
        &mut hidden,
        "explicit forbidden findings hidden by limit",
        &include_hidden_expand,
    );
    BoundaryMapReport {
        kind: "boundary_map_report",
        schema_version: "5",
        scope,
        domains: project
            .domains
            .iter()
            .filter(|domain| {
                !hide_support || !is_support_artifact_path(&domain.path) || changed_has_support
            })
            .map(DomainRef::from)
            .collect(),
        actual_cross_edges,
        public_boundary_files,
        test_only_crossings,
        package_edges,
        explicit_forbidden_findings,
        unknowns: Vec::new(),
        hidden,
        expand: vec!["codemap boundaries".to_string()],
    }
}

fn finding_touches_scope(finding: &BoundaryFinding, scope: &str) -> bool {
    path_under_scope(&finding.from, scope) || path_under_scope(&finding.to, scope)
}

fn changed_touches(changed: Option<&BTreeSet<String>>, path: &str) -> bool {
    changed.is_none_or(|changed| changed.contains(path))
}

fn changed_touches_any(changed: Option<&BTreeSet<String>>, paths: &[&str]) -> bool {
    changed.is_none_or(|changed| paths.iter().any(|path| changed.contains(*path)))
}

fn support_fact_hidden(
    hide_support: bool,
    changed: Option<&BTreeSet<String>>,
    paths: &[&str],
) -> bool {
    if !hide_support || !paths.iter().any(|path| is_support_artifact_path(path)) {
        return false;
    }
    changed.is_none_or(|changed| !paths.iter().any(|path| changed.contains(*path)))
}

fn package_edge_paths(edge: &PackageDependency) -> Vec<&str> {
    let mut paths = vec![
        edge.from_manifest.as_str(),
        edge.to_manifest.as_deref().unwrap_or(edge.to.as_str()),
    ];
    if let Some(workspace_manifest) = edge.workspace_manifest.as_deref() {
        paths.push(workspace_manifest);
    }
    paths
}
