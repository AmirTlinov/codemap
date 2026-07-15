// Responsibility: cache-incremental-reverse-import-index
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::model::FileInfo;

const FORMAT_VERSION: u32 = 1;
const ARTIFACT: &str = "reverse-imports.json";

pub struct ReverseImportUpdate {
    pub index: BTreeMap<String, BTreeSet<String>>,
    pub strategy: &'static str,
    pub affected_targets: usize,
}

pub fn full(files: &BTreeMap<String, FileInfo>) -> ReverseImportUpdate {
    let index = crate::repo::build_reverse_imports(files);
    ReverseImportUpdate {
        affected_targets: index.len(),
        index,
        strategy: "full",
    }
}

pub fn incremental(
    cache_dir: &Path,
    version: &str,
    root: &Path,
    fingerprint: &str,
    old_files: &BTreeMap<String, FileInfo>,
    new_files: &BTreeMap<String, FileInfo>,
) -> ReverseImportUpdate {
    let Some(mut index) = read(cache_dir, version, root, fingerprint) else {
        return full(new_files);
    };
    let mut sources = old_files
        .keys()
        .chain(new_files.keys())
        .collect::<BTreeSet<_>>();
    let mut affected_targets = BTreeSet::new();
    for source in std::mem::take(&mut sources) {
        let old = old_files
            .get(source)
            .map(|file| crate::repo::reverse_import_targets_for_file(old_files, file));
        let new = new_files
            .get(source)
            .map(|file| crate::repo::reverse_import_targets_for_file(new_files, file));
        if old == new {
            continue;
        }
        if let Some(targets) = &old {
            for target in targets {
                affected_targets.insert(target.clone());
                if let Some(importers) = index.get_mut(target) {
                    importers.remove(source);
                    if importers.is_empty() {
                        index.remove(target);
                    }
                }
            }
        }
        if let Some(targets) = &new {
            for target in targets {
                affected_targets.insert(target.clone());
                index
                    .entry(target.clone())
                    .or_default()
                    .insert(source.clone());
            }
        }
    }
    ReverseImportUpdate {
        index,
        strategy: if affected_targets.is_empty() {
            "cached"
        } else {
            "affected"
        },
        affected_targets: affected_targets.len(),
    }
}

pub fn write(project: &crate::model::Project, version: &str) -> anyhow::Result<()> {
    let index_sha256 = index_sha256(&project.reverse_imports)?;
    let cached = CachedReverseImports {
        format_version: FORMAT_VERSION,
        version: version.to_string(),
        root: project.root.to_string_lossy().to_string(),
        fingerprint: super::fingerprint(project, None),
        index_sha256,
        imports: project.reverse_imports.clone(),
    };
    let body = serde_json::to_string_pretty(&cached)?;
    super::io::write_cache_path(
        &project.cache_dir,
        &project.cache_dir.join(ARTIFACT),
        format!("{body}\n"),
    )
}

fn read(
    cache_dir: &Path,
    version: &str,
    root: &Path,
    fingerprint: &str,
) -> Option<BTreeMap<String, BTreeSet<String>>> {
    let path = cache_dir.join(ARTIFACT);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => {
            super::io::record_event(cache_dir, "read", ARTIFACT, "failed", &error.to_string());
            let _ = super::io::quarantine_artifact(cache_dir, &path, "reverse index read failure");
            return None;
        }
    };
    let cached: CachedReverseImports = match serde_json::from_str(&text) {
        Ok(cached) => cached,
        Err(error) => {
            let _ = super::io::quarantine_artifact(
                cache_dir,
                &path,
                &format!("reverse index parse failure: {error}"),
            );
            return None;
        }
    };
    let identity_matches = cached.format_version == FORMAT_VERSION
        && cached.version == version
        && cached.root == root.to_string_lossy()
        && cached.fingerprint == fingerprint;
    let integrity_matches =
        index_sha256(&cached.imports).is_ok_and(|actual| actual == cached.index_sha256);
    if !identity_matches || !integrity_matches {
        let reason = if identity_matches {
            "reverse index integrity mismatch"
        } else {
            "reverse index identity mismatch"
        };
        let _ = super::io::quarantine_artifact(cache_dir, &path, reason);
        return None;
    }
    Some(cached.imports)
}

fn index_sha256(index: &BTreeMap<String, BTreeSet<String>>) -> anyhow::Result<String> {
    let body = serde_json::to_vec(index)?;
    let mut hasher = Sha256::new();
    hasher.update(body);
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

#[derive(Deserialize, Serialize)]
struct CachedReverseImports {
    format_version: u32,
    version: String,
    root: String,
    fingerprint: String,
    index_sha256: String,
    imports: BTreeMap<String, BTreeSet<String>>,
}
