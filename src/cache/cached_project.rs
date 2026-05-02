use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::model::{FileInfo, ImportBindingsBySpec, PackageInfo, Project, ScanStats, ScriptInfo};

pub struct CachedProjectData {
    pub files: BTreeMap<String, FileInfo>,
    pub scan_stats: ScanStats,
}

pub fn read_cached_project(
    cache_dir: &Path,
    version: &str,
    cached_fingerprint: &str,
) -> Option<CachedProjectData> {
    let text = fs::read_to_string(cache_dir.join("inventory.json")).ok()?;
    let inventory: CachedInventory = serde_json::from_str(&text).ok()?;
    if inventory.version != version || inventory.fingerprint != cached_fingerprint {
        return None;
    }
    let files = inventory
        .files
        .into_iter()
        .map(|file| {
            let rel = file.path.clone();
            (rel, file.into_file_info())
        })
        .collect();
    Some(CachedProjectData {
        files,
        scan_stats: inventory.scan_stats,
    })
}

pub(super) fn write_inventory(project: &Project, version: &str) -> Result<()> {
    let inventory = CachedInventory {
        version: version.to_string(),
        root: project.root.to_string_lossy().to_string(),
        fingerprint: super::fingerprint(project, None),
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
                imports: file.imports.iter().cloned().collect(),
                import_bindings: cache_bindings(&file.import_bindings),
                resolved_imports: file.resolved_imports.iter().cloned().collect(),
                unresolved_imports: file.unresolved_imports.iter().cloned().collect(),
                resolved_import_bindings: cache_bindings(&file.resolved_import_bindings),
                exports: file.exports.iter().cloned().collect(),
                symbols: file.symbols.clone(),
                tokens: file.tokens.iter().cloned().collect(),
                references: file.references.iter().cloned().collect(),
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
            .map(|domain| super::CachedDomain {
                id: domain.id.clone(),
                path: domain.path.clone(),
                config: domain.config_path.clone(),
            })
            .collect(),
        scripts: project.scripts.clone(),
        languages: project.languages.iter().cloned().collect(),
        scan_stats: project.scan_stats.clone(),
    };
    let body = serde_json::to_string_pretty(&inventory)?;
    fs::write(
        project.cache_dir.join("inventory.json"),
        format!("{body}\n"),
    )?;
    Ok(())
}

fn cache_bindings(values: &ImportBindingsBySpec) -> Vec<CachedImportBindings> {
    values
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
        .collect()
}

#[derive(Deserialize, Serialize)]
struct CachedInventory {
    version: String,
    root: String,
    fingerprint: String,
    files: Vec<CachedFile>,
    packages: Vec<PackageInfo>,
    domains: Vec<super::CachedDomain>,
    scripts: Vec<ScriptInfo>,
    languages: Vec<String>,
    scan_stats: ScanStats,
}

#[derive(Deserialize, Serialize)]
struct CachedFile {
    path: String,
    language: String,
    ext: String,
    size: u64,
    line_count: usize,
    roles: Vec<String>,
    imports: Vec<String>,
    import_bindings: Vec<CachedImportBindings>,
    resolved_imports: Vec<String>,
    unresolved_imports: Vec<String>,
    resolved_import_bindings: Vec<CachedImportBindings>,
    exports: Vec<String>,
    symbols: Vec<crate::model::SymbolInfo>,
    tokens: Vec<String>,
    references: Vec<String>,
    jsx_tags: Vec<String>,
    local_bindings: Vec<String>,
    surface_tokens: Vec<String>,
    surface_phrases: Vec<String>,
    visited_route_paths: Vec<String>,
}

impl CachedFile {
    fn into_file_info(self) -> FileInfo {
        FileInfo {
            rel: self.path,
            ext: self.ext,
            size: self.size,
            line_count: self.line_count,
            language: self.language,
            roles: self.roles.into_iter().collect(),
            imports: self.imports.into_iter().collect(),
            import_bindings: cached_bindings(self.import_bindings),
            resolved_imports: self.resolved_imports.into_iter().collect(),
            unresolved_imports: self.unresolved_imports.into_iter().collect(),
            resolved_import_bindings: cached_bindings(self.resolved_import_bindings),
            exports: self.exports.into_iter().collect(),
            symbols: self.symbols,
            tokens: self.tokens.into_iter().collect(),
            references: self.references.into_iter().collect(),
            jsx_tags: self.jsx_tags.into_iter().collect(),
            local_bindings: self.local_bindings.into_iter().collect(),
            surface_tokens: self.surface_tokens.into_iter().collect(),
            surface_phrases: self.surface_phrases.into_iter().collect(),
            visited_route_paths: self.visited_route_paths.into_iter().collect(),
        }
    }
}

fn cached_bindings(values: Vec<CachedImportBindings>) -> ImportBindingsBySpec {
    values
        .into_iter()
        .map(|value| {
            (
                value.spec,
                value
                    .bindings
                    .into_iter()
                    .map(|binding| (binding.local, binding.imported))
                    .collect(),
            )
        })
        .collect()
}

#[derive(Deserialize, Serialize)]
struct CachedImportBindings {
    spec: String,
    bindings: Vec<CachedImportBinding>,
}

#[derive(Deserialize, Serialize)]
struct CachedImportBinding {
    local: String,
    imported: String,
}
