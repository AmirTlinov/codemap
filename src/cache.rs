use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::evidence::{import_statement_locations, package_dependency_locations};
use crate::model::{CacheArtifactStatus, EvidenceStrength, GraphEdge, Project};

pub(crate) mod cached_project;
pub(crate) mod fingerprints;

pub use cached_project::read_cached_project;
pub use fingerprints::{cached_git_head_matches, file_delta, file_delta_for_known_changes};

const CACHE_ARTIFACTS: &[&str] = &[
    "status.json",
    "inventory.json",
    "graph.json",
    "fingerprints.json",
];

pub fn cache_base_dir() -> PathBuf {
    if let Ok(dir) = env::var("CODEMAP_CACHE_DIR") {
        return PathBuf::from(dir);
    }
    dirs::cache_dir()
        .unwrap_or_else(env::temp_dir)
        .join("codemap")
}

pub fn project_cache_dir(root: &Path, remote: Option<&str>, version: &str) -> PathBuf {
    cache_base_dir().join(repo_key(root, remote, version))
}

pub fn cache_enabled() -> bool {
    env::var_os("CODEMAP_NO_CACHE").is_none()
}

pub fn expected_artifacts() -> &'static [&'static str] {
    CACHE_ARTIFACTS
}

pub fn repo_key(root: &Path, remote: Option<&str>, version: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(root.to_string_lossy().as_bytes());
    hasher.update([0]);
    if let Some(remote) = remote {
        hasher.update(remote.as_bytes());
    }
    hasher.update([0]);
    hasher.update(version.as_bytes());
    let hash = hasher.finalize();
    hex_prefix(&hash, 20)
}

pub fn fingerprint(project: &Project, domain_path: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    let domain_prefix = domain_path
        .filter(|p| *p != ".")
        .map(|p| format!("{}/", p.trim_end_matches('/')));
    for file in project.files.values() {
        if let Some(prefix) = &domain_prefix
            && !file.rel.starts_with(prefix)
        {
            continue;
        }
        hasher.update(file.rel.as_bytes());
        hasher.update([0]);
        hasher.update(file.size.to_string().as_bytes());
        if let Some(content_hash) = &file.content_hash {
            hasher.update(b"content");
            hasher.update(content_hash.as_bytes());
        } else if let Ok(meta) = std::fs::metadata(file.rel_path(project))
            && let Ok(modified) = meta.modified()
            && let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH)
        {
            hasher.update(b"metadata");
            hasher.update(duration.as_secs().to_string().as_bytes());
            hasher.update(duration.subsec_nanos().to_string().as_bytes());
        }
    }
    if let Some(path) = &project.config_path {
        hasher.update(path.as_bytes());
    }
    let hash = hasher.finalize();
    hex_prefix(&hash, 16)
}

pub fn write_status(project: &Project, version: &str) -> Result<()> {
    if !cache_enabled() {
        return Ok(());
    }
    fs::create_dir_all(&project.cache_dir)?;
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
        artifacts: CACHE_ARTIFACTS,
    };
    let body = serde_json::to_string_pretty(&status)?;
    fs::write(project.cache_dir.join("status.json"), format!("{body}\n"))?;
    cached_project::write_inventory(project, version)?;
    write_graph(project, version)?;
    fingerprints::write_fingerprints(project, version)?;
    Ok(())
}

pub fn artifact_statuses(project: &Project, fingerprint: &str) -> Vec<CacheArtifactStatus> {
    expected_artifacts()
        .iter()
        .map(|name| {
            let path = project.cache_dir.join(name);
            let meta = fs::metadata(&path).ok();
            let fingerprint_match = if meta.is_some() {
                cached_fingerprint(&path).map(|cached| cached == fingerprint)
            } else {
                None
            };
            CacheArtifactStatus {
                name: (*name).to_string(),
                path: path.to_string_lossy().to_string(),
                exists: meta.is_some(),
                bytes: meta.map(|m| m.len()),
                fingerprint_match,
            }
        })
        .collect()
}

pub fn cache_state(artifacts: &[CacheArtifactStatus]) -> String {
    if !cache_enabled() {
        return "disabled".to_string();
    }
    if artifacts.iter().all(|artifact| artifact.exists)
        && artifacts
            .iter()
            .all(|artifact| artifact.fingerprint_match == Some(true))
    {
        return "warm".to_string();
    }
    if artifacts.iter().any(|artifact| artifact.exists) {
        return "stale".to_string();
    }
    "cold".to_string()
}

fn cached_fingerprint(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    value
        .get("fingerprint")
        .and_then(Value::as_str)
        .map(str::to_string)
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
pub(super) struct CachedDomain {
    id: String,
    path: String,
    config: Option<String>,
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
    fs::write(project.cache_dir.join("graph.json"), format!("{body}\n"))?;
    Ok(())
}

#[derive(Serialize)]
struct CachedGraph<'a> {
    version: &'a str,
    root: String,
    fingerprint: String,
    edges: Vec<GraphEdge>,
}

fn hex_prefix(bytes: &[u8], chars: usize) -> String {
    bytes
        .iter()
        .flat_map(|b| [b >> 4, b & 0x0f])
        .take(chars)
        .map(|n| char::from_digit(n as u32, 16).expect("hex digit"))
        .collect()
}

trait RelPath {
    fn rel_path(&self, project: &Project) -> PathBuf;
}

impl RelPath for crate::model::FileInfo {
    fn rel_path(&self, project: &Project) -> PathBuf {
        project.root.join(&self.rel)
    }
}
