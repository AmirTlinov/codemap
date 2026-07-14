// Responsibility: repo-packages-domains
use crate::model::{CodemapConfig, Domain, FileInfo};
use crate::repo::{
    DOMAIN_HINT_DIRS, normalize_rel_path, should_ignore_rel, workspace_domain_paths,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub(crate) fn discover_domains(
    root: &Path,
    files: &BTreeMap<String, FileInfo>,
    anchors: &CodemapConfig,
    config_path: Option<&str>,
) -> Vec<Domain> {
    let mut domains = BTreeMap::<String, Domain>::new();
    if let Some(domain) = &anchors.domain {
        let id = domain.id.clone().unwrap_or_else(|| "repo".to_string());
        let path = normalize_rel_path(domain.path.as_deref().unwrap_or("."));
        domains.insert(
            id.clone(),
            Domain {
                id,
                path,
                config_path: config_path.map(str::to_string),
            },
        );
    }
    for (id, domain) in &anchors.domains {
        let path = normalize_rel_path(domain.path.as_deref().unwrap_or(id));
        domains.insert(
            id.clone(),
            Domain {
                id: id.clone(),
                path,
                config_path: config_path.map(str::to_string),
            },
        );
    }

    for rel in workspace_domain_paths(root, files) {
        let id = Path::new(&rel)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&rel)
            .to_string();
        domains.entry(id.clone()).or_insert(Domain {
            id,
            path: rel,
            config_path: None,
        });
    }

    for hint in DOMAIN_HINT_DIRS {
        let base = root.join(hint);
        if !base.is_dir() {
            continue;
        }
        let Ok(children) = fs::read_dir(base) else {
            continue;
        };
        for child in children.flatten() {
            let path = child.path();
            if !path.is_dir() {
                continue;
            }
            let rel =
                normalize_rel_path(&path.strip_prefix(root).unwrap_or(&path).to_string_lossy());
            if should_ignore_rel(&rel) {
                continue;
            }
            let has_files = files
                .keys()
                .any(|file| file.starts_with(&format!("{rel}/")));
            let has_markers = [
                "src",
                "tests",
                "test",
                "package.json",
                "Cargo.toml",
                "go.mod",
                ".codemap.yml",
            ]
            .iter()
            .any(|marker| path.join(marker).exists());
            if has_files || has_markers {
                let id = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&rel)
                    .to_string();
                domains.entry(id.clone()).or_insert(Domain {
                    id,
                    path: rel,
                    config_path: None,
                });
            }
        }
    }

    if domains.is_empty() {
        let id = anchors
            .domain
            .as_ref()
            .and_then(|d| d.id.clone())
            .unwrap_or_else(|| "repo".to_string());
        domains.insert(
            id.clone(),
            Domain {
                id,
                path: ".".to_string(),
                config_path: config_path.map(str::to_string),
            },
        );
    }

    domains.into_values().collect()
}
