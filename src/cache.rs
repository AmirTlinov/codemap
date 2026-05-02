use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::Result;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::evidence::{import_statement_locations, package_dependency_locations};
use crate::model::{
    CacheArtifactStatus, EnvSurface, EvidenceStrength, GraphEdge, HiddenGroup, Project,
    RuntimeReport, RuntimeRoute, StructuralEdge, Surface, Unknown,
};

pub(crate) mod cached_project;
pub(crate) mod fingerprint_delta;
pub(crate) mod fingerprints;
pub(crate) mod lens_artifacts;

pub use cached_project::read_cached_project;
pub use fingerprints::{
    cached_git_head, cached_git_head_matches, file_delta, file_delta_by_rechecking_cached_files,
    file_delta_for_head_change, file_delta_for_known_changes,
};
pub use lens_artifacts::{
    ConeLensKey, LsLensKey, PlaceLensKey, SiblingsLensKey, read_changed_report, read_cone_report,
    read_ls_report, read_place_report, read_proof_changed_report, read_proof_map_report,
    read_siblings_report, write_changed_report, write_cone_report, write_ls_report,
    write_place_report, write_proof_changed_report, write_proof_map_report, write_siblings_report,
};

const CACHE_ARTIFACTS: &[&str] = &[
    "status.json",
    "inventory.json",
    "graph.json",
    "fingerprints.json",
    "runtime-root.json",
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

pub fn stale_lens_artifact_examples(
    cache_dir: &Path,
    version: &str,
    root: &Path,
    fingerprint: &str,
) -> Vec<String> {
    lens_artifacts::artifact_names()
        .iter()
        .filter_map(|name| {
            let path = cache_dir.join(name);
            if !path.exists() {
                return None;
            }
            stale_lens_artifact_reason(&path, version, root, fingerprint)
                .map(|reason| format!("{name} ({reason})"))
        })
        .collect()
}

fn stale_lens_artifact_reason(
    path: &Path,
    version: &str,
    root: &Path,
    fingerprint: &str,
) -> Option<String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => return Some("unreadable json".to_string()),
    };
    let value: Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(_) => return Some("invalid json".to_string()),
    };
    match value.get("format_version").and_then(Value::as_u64) {
        Some(found) if found == lens_artifacts::format_version() => {}
        Some(found) => {
            return Some(format!(
                "format {found} != {}",
                lens_artifacts::format_version()
            ));
        }
        None => return Some("format missing".to_string()),
    }
    match value.get("version").and_then(Value::as_str) {
        Some(found) if found == version => {}
        Some(found) => return Some(format!("version {found} != {version}")),
        None => return Some("version missing".to_string()),
    }
    let expected_root = root.to_string_lossy();
    match value.get("root").and_then(Value::as_str) {
        Some(found) if found == expected_root.as_ref() => {}
        Some(_) => return Some("root mismatch".to_string()),
        None => return Some("root missing".to_string()),
    }
    match value.get("fingerprint").and_then(Value::as_str) {
        Some(found) if found == fingerprint => None,
        Some(_) => Some("fingerprint mismatch".to_string()),
        None => Some("fingerprint missing".to_string()),
    }
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

pub fn write_status_with_change_sets(
    project: &Project,
    version: &str,
    git_status_change_sets: Option<(&BTreeSet<String>, &BTreeSet<String>)>,
) -> Result<()> {
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
    write_runtime_root(project, version)?;
    fingerprints::write_fingerprints(project, version, git_status_change_sets)?;
    Ok(())
}

pub fn read_runtime_root_report(
    cache_dir: &Path,
    version: &str,
    root: &Path,
) -> Option<RuntimeReport> {
    let text = fs::read_to_string(cache_dir.join("runtime-root.json")).ok()?;
    let cached: CachedRuntimeRoot = serde_json::from_str(&text).ok()?;
    if cached.version != version {
        return None;
    }
    if cached.root != root.to_string_lossy() {
        return None;
    }
    if cached.fingerprint != cached_status_fingerprint(cache_dir)? {
        return None;
    }
    Some(cached.report.into_report())
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
    if let Some(fingerprint) = cached_fingerprint_from_header(path) {
        return Some(fingerprint);
    }
    let text = fs::read_to_string(path).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    value
        .get("fingerprint")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn cached_fingerprint_from_header(path: &Path) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    for line in BufReader::new(file).lines().take(64) {
        let line = line.ok()?;
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("\"fingerprint\"") else {
            continue;
        };
        let (_, value) = rest.split_once(':')?;
        let value = value.trim().trim_end_matches(',');
        return serde_json::from_str(value).ok();
    }
    None
}

pub(super) fn cached_status_fingerprint(cache_dir: &Path) -> Option<String> {
    cached_fingerprint(&cache_dir.join("status.json"))
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

fn write_runtime_root(project: &Project, version: &str) -> Result<()> {
    let report = crate::map::runtime_report(project, ".", false, 20);
    let cached = CachedRuntimeRoot {
        version: version.to_string(),
        root: project.root.to_string_lossy().to_string(),
        fingerprint: fingerprint(project, None),
        report: CachedRuntimeReport::from_report(report),
    };
    let body = serde_json::to_string_pretty(&cached)?;
    fs::write(
        project.cache_dir.join("runtime-root.json"),
        format!("{body}\n"),
    )?;
    Ok(())
}

#[derive(Deserialize, Serialize)]
struct CachedRuntimeRoot {
    version: String,
    root: String,
    fingerprint: String,
    report: CachedRuntimeReport,
}

#[derive(Deserialize, Serialize)]
struct CachedRuntimeReport {
    kind: String,
    schema_version: String,
    scope: String,
    entrypoints: Vec<Surface>,
    routes: Vec<RuntimeRoute>,
    scripts: Vec<Surface>,
    env: Vec<EnvSurface>,
    workers: Vec<Surface>,
    ci: Vec<Surface>,
    proof: Vec<StructuralEdge>,
    unknowns: Vec<Unknown>,
    hidden: Vec<HiddenGroup>,
    expand: Vec<String>,
}

impl CachedRuntimeReport {
    fn from_report(report: RuntimeReport) -> Self {
        Self {
            kind: report.kind.to_string(),
            schema_version: report.schema_version.to_string(),
            scope: report.scope,
            entrypoints: report.entrypoints,
            routes: report.routes,
            scripts: report.scripts,
            env: report.env,
            workers: report.workers,
            ci: report.ci,
            proof: report.proof,
            unknowns: report.unknowns,
            hidden: report.hidden,
            expand: report.expand,
        }
    }

    fn into_report(self) -> RuntimeReport {
        RuntimeReport {
            kind: "runtime_report",
            schema_version: "1",
            scope: self.scope,
            entrypoints: self.entrypoints,
            routes: self.routes,
            scripts: self.scripts,
            env: self.env,
            workers: self.workers,
            ci: self.ci,
            proof: self.proof,
            unknowns: self.unknowns,
            hidden: self.hidden,
            expand: self.expand,
        }
    }
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
