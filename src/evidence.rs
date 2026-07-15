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
    let Some(text) = project.read_indexed_text(from) else {
        return vec![EvidenceLocation::path(from, "import_source_file")];
    };
    let mut locations = Vec::new();
    let mut css_block_comment = false;
    let mut inside_import_block = false;
    for (index, line) in text.lines().enumerate() {
        let visible;
        let line = if matches!(info.ext.as_str(), "css" | "scss" | "sass" | "less") {
            visible = crate::repo::css_line_without_block_comments(line, &mut css_block_comment);
            visible.as_str()
        } else {
            line
        };
        let trimmed = line.trim_start();
        let import_like = line_looks_like_import_or_reexport(trimmed);
        if !import_like && !inside_import_block {
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
        if import_like && import_statement_may_continue(line) {
            inside_import_block = true;
        } else if inside_import_block && import_statement_complete(line) {
            inside_import_block = false;
        }
    }
    if locations.is_empty() {
        locations.extend(rust_wildcard_crate_import_locations(
            project, from, to, &text,
        ));
    }
    if locations.is_empty() {
        locations.extend(spec_text_import_locations(info, from, to, &text));
    }
    if locations.is_empty() {
        vec![EvidenceLocation::path(from, "import_source_file")]
    } else {
        locations
    }
}

// Cheap last-resort line search: find the raw import spec text (quoted, or on
// a `use`/`from` line) for specs that plausibly resolve to `to`.
fn spec_text_import_locations(
    info: &crate::model::FileInfo,
    from: &str,
    to: &str,
    text: &str,
) -> Vec<EvidenceLocation> {
    let specs = info
        .imports
        .iter()
        .filter(|spec| spec_refers_to_target(spec, to))
        .collect::<Vec<_>>();
    if specs.is_empty() {
        return Vec::new();
    }
    text.lines()
        .enumerate()
        .filter(|(_, line)| {
            specs
                .iter()
                .any(|spec| line_mentions_import_spec(line, spec))
        })
        .map(|(index, _)| EvidenceLocation::line(from, index + 1, "import_statement"))
        .take(3)
        .collect()
}

fn spec_refers_to_target(spec: &str, to: &str) -> bool {
    let last = spec
        .trim_end_matches('/')
        .rsplit(['/', ':'])
        .next()
        .unwrap_or(spec);
    if last.is_empty() || last == "." || last == ".." || last == "*" {
        return false;
    }
    let last_stem = Path::new(last)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(last);
    let target = Path::new(to);
    let target_name = target.file_name().and_then(|name| name.to_str());
    let target_stem = target.file_stem().and_then(|stem| stem.to_str());
    if Some(last) == target_name || Some(last_stem) == target_stem {
        return true;
    }
    matches!(target_stem, Some("index" | "mod" | "__init__"))
        && target
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            == Some(last_stem)
}

fn line_mentions_import_spec(line: &str, spec: &str) -> bool {
    if line.contains(&format!("\"{spec}\"")) || line.contains(&format!("'{spec}'")) {
        return true;
    }
    let trimmed = line.trim_start();
    (trimmed.starts_with("use ") || trimmed.starts_with("from ")) && line.contains(spec)
}

fn rust_wildcard_crate_import_locations(
    project: &Project,
    from: &str,
    to: &str,
    text: &str,
) -> Vec<EvidenceLocation> {
    let Some(info) = project.files.get(from) else {
        return Vec::new();
    };
    if info.ext != "rs" {
        return Vec::new();
    }
    let Some(crate_name) = rust_crate_name_for_target(project, to) else {
        return Vec::new();
    };
    let wildcard = format!("use {crate_name}::*");
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let compact = line.split_whitespace().collect::<Vec<_>>().join(" ");
            compact
                .starts_with(&wildcard)
                .then(|| EvidenceLocation::line(from, index + 1, "import_statement"))
        })
        .take(3)
        .collect()
}

fn rust_crate_name_for_target(project: &Project, target: &str) -> Option<String> {
    project
        .packages
        .iter()
        .filter(|package| package.ecosystem == "rust")
        .filter(|package| {
            package.path == "."
                || target == package.path
                || target.starts_with(&format!("{}/", package.path.trim_end_matches('/')))
        })
        .max_by_key(|package| {
            if package.path == "." {
                0
            } else {
                package.path.len()
            }
        })
        .map(|package| package.name.replace('-', "_"))
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
        let Some(text) = project.read_indexed_text(manifest) else {
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
        || trimmed.starts_with("from ")
        || trimmed.starts_with("export ")
        || trimmed.starts_with("use ")
        || trimmed.starts_with("mod ")
        || trimmed.starts_with("pub mod ")
        || trimmed.starts_with("#[path")
        || trimmed.starts_with("include!(")
        || trimmed.starts_with("@import ")
}

fn import_statement_may_continue(line: &str) -> bool {
    let trimmed = line.trim_end();
    if trimmed.ends_with(';') || trimmed.ends_with(')') {
        return false;
    }
    let compact = trimmed.trim_start();
    (compact.starts_with("import ")
        || compact.starts_with("export ")
        || compact.starts_with("use ")
        || compact.starts_with("from "))
        && (trimmed.contains('{') || trimmed.contains('(') || trimmed.ends_with(','))
}

fn import_statement_complete(line: &str) -> bool {
    let trimmed = line.trim_end();
    trimmed.ends_with(';')
        || trimmed.ends_with(')')
        || (trimmed.contains(" from ") && (trimmed.contains('"') || trimmed.contains('\'')))
}
