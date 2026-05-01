use std::path::Path;

use crate::model::{EvidenceLocation, PackageDependency, Project};

pub(crate) fn import_statement_locations(
    project: &Project,
    from: &str,
    to: &str,
) -> Vec<EvidenceLocation> {
    let Some(info) = project.files.get(from) else {
        return Vec::new();
    };
    let mut names = Vec::new();
    if let Some(bindings) = info.resolved_import_bindings.get(to) {
        for (local, imported) in bindings {
            if !local.starts_with("export:") {
                names.push(local.as_str());
            }
            if imported != "*" {
                names.push(imported.as_str());
            }
        }
    }
    if let Some(stem) = Path::new(to).file_stem().and_then(|name| name.to_str()) {
        names.push(stem);
    }
    let Ok(text) = std::fs::read_to_string(project.root.join(from)) else {
        return vec![EvidenceLocation::path(from, "import_source_file")];
    };
    let mut locations = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if !line_looks_like_import_or_reexport(trimmed) {
            continue;
        }
        if names
            .iter()
            .any(|name| !name.is_empty() && line.contains(name))
        {
            locations.push(EvidenceLocation::line(from, index + 1, "import_statement"));
            if locations.len() >= 3 {
                break;
            }
        }
    }
    if locations.is_empty() {
        vec![EvidenceLocation::path(from, "import_source_file")]
    } else {
        locations
    }
}

pub(crate) fn package_dependency_locations(
    project: &Project,
    edge: &PackageDependency,
) -> Vec<EvidenceLocation> {
    let mut manifests = vec![edge.from_manifest.as_str()];
    if let Some(workspace_manifest) = edge.workspace_manifest.as_deref() {
        manifests.push(workspace_manifest);
    }
    for manifest in manifests {
        let Ok(text) = std::fs::read_to_string(project.root.join(manifest)) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            if line.contains(&edge.dependency) {
                return vec![EvidenceLocation::line(
                    manifest,
                    index + 1,
                    "package_manifest_dependency",
                )];
            }
        }
    }
    vec![EvidenceLocation::path(
        &edge.from_manifest,
        "package_manifest_dependency",
    )]
}

pub(crate) fn line_looks_like_import_or_reexport(trimmed: &str) -> bool {
    trimmed.starts_with("import ")
        || trimmed.starts_with("import(")
        || trimmed.starts_with("require(")
        || trimmed.starts_with("export ")
        || trimmed.starts_with("use ")
        || trimmed.starts_with("mod ")
        || trimmed.starts_with("pub mod ")
        || trimmed.starts_with("#[path")
        || trimmed.starts_with("include!(")
}
