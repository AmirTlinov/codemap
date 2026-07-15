// Responsibility: repo-packages-detect
use crate::model::{FileInfo, PackageInfo};
use crate::repo::{
    cargo_package_name, go_module_name, manifest_dir, package_name_from_path,
    pyproject_package_name, swift_package_name,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackageManifestCandidate {
    pub(crate) manifest: String,
    pub(crate) ecosystem: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum PackageDiscoveryGapKind {
    ReadUnavailable,
    ParseUnsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackageDiscoveryGap {
    pub(crate) manifest: String,
    pub(crate) ecosystem: &'static str,
    pub(crate) kind: PackageDiscoveryGapKind,
    /// Stable bounded vocabulary, never a parser error containing source text.
    pub(crate) construct: &'static str,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct PackageDiscoveryAudit {
    pub(crate) packages: Vec<PackageInfo>,
    pub(crate) candidates: Vec<PackageManifestCandidate>,
    pub(crate) visited_manifests: Vec<String>,
    pub(crate) unsupported: Vec<PackageDiscoveryGap>,
}

pub(crate) fn detect_packages(root: &Path, files: &BTreeMap<String, FileInfo>) -> Vec<PackageInfo> {
    audit_package_discovery(root, files).packages
}

pub(crate) fn audit_package_discovery(
    root: &Path,
    files: &BTreeMap<String, FileInfo>,
) -> PackageDiscoveryAudit {
    let mut audit = PackageDiscoveryAudit::default();
    for (rel, file) in files {
        let Some(kind) = PackageManifestKind::from_rel(rel) else {
            continue;
        };
        audit.candidates.push(PackageManifestCandidate {
            manifest: rel.clone(),
            ecosystem: kind.ecosystem(),
        });
        if file.content_hash.is_none() {
            audit.unsupported.push(kind.gap(
                rel,
                PackageDiscoveryGapKind::ReadUnavailable,
                "indexed manifest body is unavailable",
            ));
            continue;
        }
        let Some(text) = readable_manifest_text(root, rel) else {
            audit.unsupported.push(kind.gap(
                rel,
                PackageDiscoveryGapKind::ReadUnavailable,
                "manifest could not be read",
            ));
            continue;
        };
        let inspection = kind.inspect(rel, &text);
        if let Some(package) = inspection.package {
            audit.packages.push(package);
        }
        if let Some(construct) = inspection.unsupported_construct {
            audit.unsupported.push(kind.gap(
                rel,
                PackageDiscoveryGapKind::ParseUnsupported,
                construct,
            ));
        } else {
            // A valid workspace-only Cargo.toml is deliberately visited even
            // though it emits no PackageInfo fact.
            audit.visited_manifests.push(rel.clone());
        }
    }
    audit
        .packages
        .sort_by(|a, b| a.path.cmp(&b.path).then_with(|| a.name.cmp(&b.name)));
    audit
}

fn readable_manifest_text(root: &Path, rel: &str) -> Option<String> {
    let path = root.join(rel);
    let metadata = fs::symlink_metadata(&path).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return None;
    }
    crate::repo::scan_file_rejection(&path, rel, metadata.len())
        .is_none()
        .then_some(())?;
    fs::read_to_string(path).ok()
}

/// Audits the already bounded manifest-path inventory used by the cold root
/// map without triggering a second repository scan.
pub(crate) fn audit_package_discovery_paths<'a>(
    root: &Path,
    paths: impl IntoIterator<Item = &'a str>,
) -> PackageDiscoveryAudit {
    let mut audit = PackageDiscoveryAudit::default();
    for rel in paths {
        let Some(kind) = PackageManifestKind::from_rel(rel) else {
            continue;
        };
        audit.candidates.push(PackageManifestCandidate {
            manifest: rel.to_string(),
            ecosystem: kind.ecosystem(),
        });
        let Some(text) = readable_manifest_text(root, rel) else {
            audit.unsupported.push(kind.gap(
                rel,
                PackageDiscoveryGapKind::ReadUnavailable,
                "manifest could not be read",
            ));
            continue;
        };
        let inspection = kind.inspect(rel, &text);
        if let Some(package) = inspection.package {
            audit.packages.push(package);
        }
        if let Some(construct) = inspection.unsupported_construct {
            audit.unsupported.push(kind.gap(
                rel,
                PackageDiscoveryGapKind::ParseUnsupported,
                construct,
            ));
        } else {
            audit.visited_manifests.push(rel.to_string());
        }
    }
    audit
        .packages
        .sort_by(|a, b| a.path.cmp(&b.path).then_with(|| a.name.cmp(&b.name)));
    audit
}

#[derive(Debug, Clone, Copy)]
enum PackageManifestKind {
    JavaScript,
    Rust,
    Go,
    Python,
    Swift,
}

impl PackageManifestKind {
    fn from_rel(rel: &str) -> Option<Self> {
        match Path::new(rel).file_name().and_then(|name| name.to_str()) {
            Some("package.json") => Some(Self::JavaScript),
            Some("Cargo.toml") => Some(Self::Rust),
            Some("go.mod") => Some(Self::Go),
            Some("pyproject.toml") => Some(Self::Python),
            Some("Package.swift") => Some(Self::Swift),
            _ => None,
        }
    }

    fn ecosystem(self) -> &'static str {
        match self {
            Self::JavaScript => "javascript",
            Self::Rust => "rust",
            Self::Go => "go",
            Self::Python => "python",
            Self::Swift => "swift",
        }
    }

    fn inspect(self, rel: &str, text: &str) -> ManifestInspection {
        match self {
            Self::JavaScript => inspect_javascript(rel, text),
            Self::Rust => inspect_cargo(rel, text),
            Self::Go => inspect_go(rel, text),
            Self::Python => inspect_python(rel, text),
            Self::Swift => inspect_swift(rel, text),
        }
    }

    fn gap(
        self,
        rel: &str,
        kind: PackageDiscoveryGapKind,
        construct: &'static str,
    ) -> PackageDiscoveryGap {
        PackageDiscoveryGap {
            manifest: rel.to_string(),
            ecosystem: self.ecosystem(),
            kind,
            construct,
        }
    }
}

struct ManifestInspection {
    package: Option<PackageInfo>,
    unsupported_construct: Option<&'static str>,
}

impl ManifestInspection {
    fn package(rel: &str, name: String, ecosystem: &str) -> Self {
        Self {
            package: Some(PackageInfo {
                name,
                path: manifest_dir(rel),
                manifest: rel.to_string(),
                ecosystem: ecosystem.to_string(),
            }),
            unsupported_construct: None,
        }
    }

    fn unsupported(package: Option<PackageInfo>, construct: &'static str) -> Self {
        Self {
            package,
            unsupported_construct: Some(construct),
        }
    }
}

fn inspect_javascript(rel: &str, text: &str) -> ManifestInspection {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return ManifestInspection::unsupported(None, "invalid package.json JSON");
    };
    let path = manifest_dir(rel);
    let name = value
        .get("name")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| package_name_from_path(&path));
    ManifestInspection::package(rel, name, "javascript")
}

fn inspect_cargo(rel: &str, text: &str) -> ManifestInspection {
    let Ok(value) = toml::from_str::<toml::Value>(text) else {
        return ManifestInspection::unsupported(None, "invalid Cargo.toml TOML");
    };
    if let Some(name) = cargo_package_name(text) {
        return ManifestInspection::package(rel, name, "rust");
    }
    if value.get("package").is_some() {
        return ManifestInspection::unsupported(None, "Cargo.toml has no static package identity");
    }
    if value.get("workspace").is_some() {
        return ManifestInspection {
            package: None,
            unsupported_construct: None,
        };
    }
    ManifestInspection::unsupported(None, "Cargo.toml has no package or workspace declaration")
}

fn inspect_go(rel: &str, text: &str) -> ManifestInspection {
    match go_module_name(text) {
        Some(name) => ManifestInspection::package(rel, name, "go"),
        None => ManifestInspection::unsupported(None, "go.mod has no static module directive"),
    }
}

fn inspect_python(rel: &str, text: &str) -> ManifestInspection {
    let path = manifest_dir(rel);
    let fallback = || PackageInfo {
        name: package_name_from_path(&path),
        path: path.clone(),
        manifest: rel.to_string(),
        ecosystem: "python".to_string(),
    };
    if toml::from_str::<toml::Value>(text).is_err() {
        return ManifestInspection::unsupported(Some(fallback()), "invalid pyproject.toml TOML");
    }
    let name = pyproject_package_name(text).unwrap_or_else(|| package_name_from_path(&path));
    ManifestInspection::package(rel, name, "python")
}

fn inspect_swift(rel: &str, text: &str) -> ManifestInspection {
    if let Some(name) = swift_package_name(text) {
        return ManifestInspection::package(rel, name, "swift");
    }
    let path = manifest_dir(rel);
    ManifestInspection::unsupported(
        Some(PackageInfo {
            name: package_name_from_path(&path),
            path,
            manifest: rel.to_string(),
            ecosystem: "swift".to_string(),
        }),
        "Package.swift name is not statically recognized",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::scan_files;

    fn write(root: &Path, rel: &str, text: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().expect("manifest parent")).expect("create parent");
        fs::write(path, text).expect("write manifest");
    }

    #[test]
    fn audit_uses_canonical_candidates_and_visits_workspace_only_cargo() {
        let root = tempfile::TempDir::new().expect("package audit root");
        write(root.path(), "Cargo.toml", "[workspace]\nmembers = []\n");
        write(root.path(), "apps/web/package.json", r#"{"name":"web"}"#);
        write(
            root.path(),
            "apps/upper/PACKAGE.JSON",
            r#"{"name":"not-canonical"}"#,
        );
        let (files, _) = scan_files(root.path()).expect("scan manifests");

        let audit = audit_package_discovery(root.path(), &files);

        assert_eq!(
            audit
                .candidates
                .iter()
                .map(|candidate| candidate.manifest.as_str())
                .collect::<Vec<_>>(),
            vec!["Cargo.toml", "apps/web/package.json"]
        );
        assert_eq!(
            audit.visited_manifests,
            vec!["Cargo.toml", "apps/web/package.json"]
        );
        assert!(audit.unsupported.is_empty(), "{:#?}", audit.unsupported);
        assert_eq!(audit.packages.len(), 1);
        assert_eq!(audit.packages[0].name, "web");
    }

    #[test]
    fn audit_types_read_and_parse_gaps_without_losing_path_fallback_facts() {
        let root = tempfile::TempDir::new().expect("package audit root");
        write(root.path(), "bad/package.json", "{");
        write(root.path(), "workspace/Cargo.toml", "[package\n");
        write(root.path(), "service/go.mod", "go 1.24\n");
        write(root.path(), "python/pyproject.toml", "[project\n");
        write(
            root.path(),
            "swift/Package.swift",
            "let packageName = dynamic()\n",
        );
        write(
            root.path(),
            "unreadable/package.json",
            r#"{"name":"hidden"}"#,
        );
        let (mut files, _) = scan_files(root.path()).expect("scan manifests");
        files
            .get_mut("unreadable/package.json")
            .expect("unreadable candidate")
            .content_hash = None;

        let audit = audit_package_discovery(root.path(), &files);

        assert!(audit.visited_manifests.is_empty());
        assert_eq!(audit.candidates.len(), 6);
        assert_eq!(audit.unsupported.len(), 6);
        assert_eq!(
            audit
                .unsupported
                .iter()
                .filter(|gap| gap.kind == PackageDiscoveryGapKind::ReadUnavailable)
                .map(|gap| gap.manifest.as_str())
                .collect::<Vec<_>>(),
            vec!["unreadable/package.json"]
        );
        assert_eq!(
            audit
                .packages
                .iter()
                .map(|package| (package.manifest.as_str(), package.name.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("python/pyproject.toml", "python"),
                ("swift/Package.swift", "swift")
            ]
        );
    }
}
