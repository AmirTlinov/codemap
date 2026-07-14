// Responsibility: map-boundary-domains
use crate::model::{Domain, Project};
use crate::repo;
use std::path::Path;

pub(crate) fn root_domain(project: &Project) -> Domain {
    project
        .domains
        .iter()
        .find(|domain| domain.path == ".")
        .cloned()
        .unwrap_or_else(|| {
            project.domains.first().cloned().unwrap_or_else(|| Domain {
                id: "repo".to_string(),
                path: ".".to_string(),
                config_path: None,
            })
        })
}

fn path_is_in_domain(rel: &str, domain: &Domain) -> bool {
    let prefix = domain.path.trim_end_matches('/');
    prefix == "." || rel == prefix || rel.starts_with(&format!("{prefix}/"))
}

pub(crate) fn domain_for_path<'a>(project: &'a Project, path: &str) -> &'a Domain {
    let rel = if Path::new(path).is_absolute() {
        let absolute = Path::new(path)
            .canonicalize()
            .unwrap_or_else(|_| Path::new(path).to_path_buf());
        absolute
            .strip_prefix(&project.root)
            .map(|p| repo::normalize_rel_path(&p.to_string_lossy()))
            .unwrap_or_else(|_| repo::normalize_rel_path(path))
    } else {
        repo::normalize_rel_path(path)
    };
    let mut best = project
        .domains
        .iter()
        .find(|domain| domain.path == ".")
        .unwrap_or(&project.domains[0]);
    let mut best_len = 0usize;
    for domain in &project.domains {
        if path_is_in_domain(&rel, domain) {
            let prefix = domain.path.trim_end_matches('/');
            let len = if prefix == "." { 0 } else { prefix.len() };
            if len >= best_len {
                best = domain;
                best_len = len;
            }
        }
    }
    best
}

pub(crate) fn domain_by_rel<'a>(project: &'a Project, rel: &str) -> Option<&'a Domain> {
    Some(domain_for_path(project, rel))
}

pub(crate) fn scoped_domain_path_for_rel(
    project: &Project,
    rel: &str,
    scope: Option<&Domain>,
) -> Option<String> {
    if let Some(domain) = scope
        && (domain.path == "."
            || rel == domain.path
            || rel.starts_with(&format!("{}/", domain.path.trim_end_matches('/'))))
    {
        return Some(domain.path.clone());
    }
    domain_by_rel(project, rel).map(|domain| domain.path.clone())
}
