use crate::model::{Domain, GraphLens, Project};

use super::impacted_domains;

mod assembly;
pub(crate) use assembly::*;
mod causal;
pub(crate) use causal::*;
mod report_graphs;
pub(crate) use report_graphs::*;

pub fn graph_lens(
    project: &Project,
    path: Option<&str>,
    lens: &str,
    limit: usize,
    changed: Option<&[String]>,
) -> GraphLens {
    let limit = limit.max(1);
    let lens_key = lens.to_ascii_lowercase();
    let explicit_seed = path.and_then(|path| file_seed_for_path(project, path));
    let changed_selected = changed.is_some();
    let graph_changed = changed.or(explicit_seed.as_ref().map(std::slice::from_ref));
    let (nodes, edges, hidden) = match lens_key.as_str() {
        "boundary" | "boundaries" => {
            boundary_graph(project, graph_changed, limit, lens, path, changed_selected)
        }
        "impact" => impact_graph(
            project,
            graph_changed.unwrap_or(&[]),
            limit,
            lens,
            path,
            changed_selected,
        ),
        "proof" => proof_graph(project, path, changed, limit, lens, changed_selected),
        _ => causal_graph(project, path, limit, lens),
    };
    let domain = graph_output_domain(project, path, &nodes);
    GraphLens {
        kind: "graph_lens",
        schema_version: "5",
        domain: (&domain).into(),
        lens: lens.to_string(),
        nodes,
        edges,
        hidden,
    }
}

fn graph_output_domain(project: &Project, path: Option<&str>, nodes: &[String]) -> Domain {
    if let Some(path) = path
        && let Some(domain) = domain_for_path(project, path)
    {
        return domain.clone();
    }
    let domains = impacted_domains(
        project,
        &nodes
            .iter()
            .filter(|node| project.files.contains_key(*node))
            .cloned()
            .collect::<Vec<_>>(),
    );
    match domains.as_slice() {
        [only] => (*only).clone(),
        _ => project
            .domains
            .iter()
            .find(|domain| domain.path == ".")
            .cloned()
            .unwrap_or_else(|| Domain {
                id: "repo".to_string(),
                path: ".".to_string(),
                config_path: None,
            }),
    }
}

fn file_seed_for_path(project: &Project, path: &str) -> Option<String> {
    let rel = if std::path::Path::new(path).is_absolute() {
        let absolute = std::path::Path::new(path).canonicalize().ok()?;
        absolute
            .strip_prefix(&project.root)
            .ok()
            .map(|path| crate::repo::normalize_rel_path(&path.to_string_lossy()))?
    } else {
        crate::repo::normalize_rel_path(path)
    };
    project.files.contains_key(&rel).then_some(rel)
}

fn domain_for_path<'a>(project: &'a Project, path: &str) -> Option<&'a Domain> {
    let rel = crate::repo::normalize_rel_path(path);
    project
        .domains
        .iter()
        .filter(|domain| {
            domain.path == "."
                || rel == domain.path
                || rel.starts_with(&format!("{}/", domain.path))
        })
        .max_by_key(|domain| {
            if domain.path == "." {
                0
            } else {
                domain.path.len()
            }
        })
}
