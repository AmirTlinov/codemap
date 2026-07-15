// Responsibility: cache-status-artifacts
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::runtime_root::write_runtime_root;
use super::{cache_enabled, cached_project, expected_artifacts, fingerprint, fingerprints};
use crate::evidence::{import_statement_locations, package_dependency_locations};
use crate::model::{EvidenceStrength, GraphEdge, Project};

pub fn write_status_with_change_sets(
    project: &Project,
    version: &str,
    git_status_change_sets: Option<(&BTreeSet<String>, &BTreeSet<String>)>,
) -> Result<()> {
    if !cache_enabled() {
        return Ok(());
    }
    std::fs::create_dir_all(&project.cache_dir)?;
    let status = CacheStatus {
        version,
        root: project.root.to_string_lossy().to_string(),
        fingerprint: fingerprint(project, None),
        files: project.files.len(),
        domains: project
            .domains
            .iter()
            .map(|d| CachedDomain {
                id: d.id.clone(),
                path: d.path.clone(),
                config: d.config_path.clone(),
            })
            .collect(),
        artifacts: expected_artifacts(),
    };
    cached_project::write_inventory(project, version)?;
    write_graph(project, version)?;
    super::reverse_imports::write(project, version)?;
    write_runtime_root(project, version)?;
    fingerprints::write_fingerprints(project, version, git_status_change_sets)?;
    // status.json is the transaction marker consumed by lens fast paths. Publish it
    // last so an interrupted refresh can only look cold/stale, never falsely warm.
    let body = serde_json::to_string_pretty(&status)?;
    super::io::write_cache_path(
        &project.cache_dir,
        &project.cache_dir.join("status.json"),
        format!("{body}\n"),
    )?;
    Ok(())
}

#[derive(Serialize)]
struct CacheStatus<'a> {
    version: &'a str,
    root: String,
    fingerprint: String,
    files: usize,
    domains: Vec<CachedDomain>,
    artifacts: &'static [&'static str],
}

#[derive(Deserialize, Serialize)]
pub(crate) struct CachedDomain {
    pub(crate) id: String,
    pub(crate) path: String,
    pub(crate) config: Option<String>,
}

fn write_graph(project: &Project, version: &str) -> Result<()> {
    let mut edges = Vec::new();
    for file in project.files.values() {
        for target in &file.resolved_imports {
            edges.push(GraphEdge {
                from: file.rel.clone(),
                to: target.clone(),
                edge_type: "imports".to_string(),
                evidence: "resolved_import".to_string(),
                strength: EvidenceStrength::High,
                locations: import_statement_locations(project, &file.rel, target),
            });
        }
    }
    for edge in &project.package_edges {
        edges.push(GraphEdge {
            from: edge.from_manifest.clone(),
            to: edge.to_manifest.clone().unwrap_or_else(|| edge.to.clone()),
            edge_type: "package_depends".to_string(),
            evidence: edge.source.clone(),
            strength: EvidenceStrength::Hard,
            locations: package_dependency_locations(project, edge),
        });
    }
    let graph = CachedGraph {
        version,
        root: project.root.to_string_lossy().to_string(),
        fingerprint: fingerprint(project, None),
        edges,
    };
    let body = serde_json::to_string_pretty(&graph)?;
    super::io::write_cache_path(
        &project.cache_dir,
        &project.cache_dir.join("graph.json"),
        format!("{body}\n"),
    )?;
    Ok(())
}

#[derive(Serialize)]
struct CachedGraph<'a> {
    version: &'a str,
    root: String,
    fingerprint: String,
    edges: Vec<GraphEdge>,
}
