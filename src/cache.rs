use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::model::Project;

pub fn cache_base_dir() -> PathBuf {
    if let Ok(dir) = env::var("CTX_CACHE_DIR") {
        return PathBuf::from(dir);
    }
    dirs::cache_dir()
        .unwrap_or_else(env::temp_dir)
        .join("agent-context")
}

pub fn project_cache_dir(root: &Path, remote: Option<&str>, version: &str) -> PathBuf {
    cache_base_dir().join(repo_key(root, remote, version))
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
        if let Ok(meta) = std::fs::metadata(file.rel_path(project))
            && let Ok(modified) = meta.modified()
            && let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH)
        {
            hasher.update(duration.as_secs().to_string().as_bytes());
        }
    }
    if let Some(path) = &project.config_path {
        hasher.update(path.as_bytes());
    }
    let hash = hasher.finalize();
    hex_prefix(&hash, 16)
}

pub fn write_status(project: &Project, version: &str) -> Result<()> {
    if env::var_os("CTX_NO_CACHE").is_some() {
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
        artifacts: vec![
            "status.json",
            "inventory.json",
            "graph.json",
            "fingerprints.json",
        ],
    };
    let body = serde_json::to_string_pretty(&status)?;
    fs::write(project.cache_dir.join("status.json"), format!("{body}\n"))?;
    write_inventory(project, version)?;
    write_graph(project, version)?;
    write_fingerprints(project, version)?;
    Ok(())
}

#[derive(Serialize)]
struct CacheStatus<'a> {
    version: &'a str,
    root: String,
    fingerprint: String,
    files: usize,
    domains: Vec<CachedDomain>,
    artifacts: Vec<&'static str>,
}

#[derive(Serialize)]
struct CachedDomain {
    id: String,
    path: String,
    config: Option<String>,
}

fn write_inventory(project: &Project, version: &str) -> Result<()> {
    let inventory = CachedInventory {
        version,
        root: project.root.to_string_lossy().to_string(),
        fingerprint: fingerprint(project, None),
        files: project
            .files
            .values()
            .map(|file| CachedFile {
                path: file.rel.clone(),
                language: file.language.clone(),
                ext: file.ext.clone(),
                size: file.size,
                roles: file.roles.iter().cloned().collect(),
            })
            .collect(),
        packages: project.packages.clone(),
        domains: project
            .domains
            .iter()
            .map(|domain| CachedDomain {
                id: domain.id.clone(),
                path: domain.path.clone(),
                config: domain.config_path.clone(),
            })
            .collect(),
        scripts: project.scripts.clone(),
        languages: project.languages.iter().cloned().collect(),
    };
    let body = serde_json::to_string_pretty(&inventory)?;
    fs::write(
        project.cache_dir.join("inventory.json"),
        format!("{body}\n"),
    )?;
    Ok(())
}

fn write_graph(project: &Project, version: &str) -> Result<()> {
    let mut edges = Vec::new();
    for file in project.files.values() {
        for target in &file.resolved_imports {
            edges.push(CachedGraphEdge {
                from: file.rel.clone(),
                to: target.clone(),
                kind: "imports".to_string(),
                provenance: "source_import".to_string(),
            });
        }
    }
    for edge in &project.package_edges {
        edges.push(CachedGraphEdge {
            from: edge.from_manifest.clone(),
            to: edge.to_manifest.clone().unwrap_or_else(|| edge.to.clone()),
            kind: "package_depends".to_string(),
            provenance: edge.source.clone(),
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

fn write_fingerprints(project: &Project, version: &str) -> Result<()> {
    let fingerprints = CachedFingerprints {
        version,
        root: project.root.to_string_lossy().to_string(),
        fingerprint: fingerprint(project, None),
        files: project
            .files
            .values()
            .map(|file| CachedFileFingerprint {
                path: file.rel.clone(),
                size: file.size,
                modified_secs: file_modified_secs(project, file),
            })
            .collect(),
    };
    let body = serde_json::to_string_pretty(&fingerprints)?;
    fs::write(
        project.cache_dir.join("fingerprints.json"),
        format!("{body}\n"),
    )?;
    Ok(())
}

#[derive(Serialize)]
struct CachedInventory<'a> {
    version: &'a str,
    root: String,
    fingerprint: String,
    files: Vec<CachedFile>,
    packages: Vec<crate::model::PackageInfo>,
    domains: Vec<CachedDomain>,
    scripts: Vec<crate::model::ScriptInfo>,
    languages: Vec<String>,
}

#[derive(Serialize)]
struct CachedFile {
    path: String,
    language: String,
    ext: String,
    size: u64,
    roles: Vec<String>,
}

#[derive(Serialize)]
struct CachedGraph<'a> {
    version: &'a str,
    root: String,
    fingerprint: String,
    edges: Vec<CachedGraphEdge>,
}

#[derive(Serialize)]
struct CachedGraphEdge {
    from: String,
    to: String,
    kind: String,
    provenance: String,
}

#[derive(Serialize)]
struct CachedFingerprints<'a> {
    version: &'a str,
    root: String,
    fingerprint: String,
    files: Vec<CachedFileFingerprint>,
}

#[derive(Serialize)]
struct CachedFileFingerprint {
    path: String,
    size: u64,
    modified_secs: Option<u64>,
}

fn file_modified_secs(project: &Project, file: &crate::model::FileInfo) -> Option<u64> {
    let meta = fs::metadata(file.rel_path(project)).ok()?;
    let modified = meta.modified().ok()?;
    let duration = modified.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(duration.as_secs())
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
