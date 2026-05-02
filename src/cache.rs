use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::evidence::{import_statement_locations, package_dependency_locations};
use crate::model::{CacheArtifactStatus, EvidenceStrength, GraphEdge, Project};

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
    write_inventory(project, version)?;
    write_graph(project, version)?;
    write_fingerprints(project, version)?;
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
                line_count: file.line_count,
                roles: file.roles.iter().cloned().collect(),
                import_bindings: file
                    .import_bindings
                    .iter()
                    .map(|(spec, bindings)| CachedImportBindings {
                        spec: spec.clone(),
                        bindings: bindings
                            .iter()
                            .map(|(local, imported)| CachedImportBinding {
                                local: local.clone(),
                                imported: imported.clone(),
                            })
                            .collect(),
                    })
                    .collect(),
                unresolved_imports: file.unresolved_imports.iter().cloned().collect(),
                resolved_import_bindings: file
                    .resolved_import_bindings
                    .iter()
                    .map(|(target, bindings)| CachedImportBindings {
                        spec: target.clone(),
                        bindings: bindings
                            .iter()
                            .map(|(local, imported)| CachedImportBinding {
                                local: local.clone(),
                                imported: imported.clone(),
                            })
                            .collect(),
                    })
                    .collect(),
                symbols: file.symbols.clone(),
                jsx_tags: file.jsx_tags.iter().cloned().collect(),
                local_bindings: file.local_bindings.iter().cloned().collect(),
                surface_tokens: file.surface_tokens.iter().cloned().collect(),
                surface_phrases: file.surface_phrases.iter().cloned().collect(),
                visited_route_paths: file.visited_route_paths.iter().cloned().collect(),
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
    line_count: usize,
    roles: Vec<String>,
    import_bindings: Vec<CachedImportBindings>,
    unresolved_imports: Vec<String>,
    resolved_import_bindings: Vec<CachedImportBindings>,
    symbols: Vec<crate::model::SymbolInfo>,
    jsx_tags: Vec<String>,
    local_bindings: Vec<String>,
    surface_tokens: Vec<String>,
    surface_phrases: Vec<String>,
    visited_route_paths: Vec<String>,
}

#[derive(Serialize)]
struct CachedImportBindings {
    spec: String,
    bindings: Vec<CachedImportBinding>,
}

#[derive(Serialize)]
struct CachedImportBinding {
    local: String,
    imported: String,
}

#[derive(Serialize)]
struct CachedGraph<'a> {
    version: &'a str,
    root: String,
    fingerprint: String,
    edges: Vec<GraphEdge>,
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
