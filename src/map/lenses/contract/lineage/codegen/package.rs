// Responsibility: generated-artifact-package-export-and-consumer-lineage
use super::CodegenLineage;
use crate::map::{
    anchor_symbol_reference_names, direct_consumer_edges, package_for_rel, quoted_literal_contents,
    structural_edge_with_locations, unknown,
};
use crate::model::{EvidenceLocation, EvidenceStrength, Project};
use crate::repo;
use std::collections::{BTreeSet, VecDeque};

pub(super) fn add_package_export_and_consumers(
    project: &Project,
    generated: &str,
    anchor: &str,
    out: &mut CodegenLineage,
) {
    let Some(package) = package_for_rel(project, generated) else {
        return;
    };
    let Some(manifest) = project.read_indexed_text(&package.manifest) else {
        return;
    };
    let generated_key = artifact_projection_key(generated, &package.path);
    let export_target = quoted_literal_contents(&manifest)
        .into_iter()
        .filter(|literal| literal.starts_with('.') || literal.contains('/'))
        .find(|literal| artifact_projection_key(literal, ".") == generated_key);
    let Some(export_target) = export_target else {
        out.unknowns.push(unknown(
            "generated_package_export_missing",
            Some(&package.manifest),
            None,
            "generated contract artifact has no matching package export target",
            "consumer lineage stops at the generated artifact instead of assuming package visibility",
            Some(format!("codemap contract {}", package.manifest)),
        ));
        return;
    };
    add_export_edge(generated, anchor, package, &manifest, &export_target, out);
    let relevant_names = add_internal_consumers(project, generated, anchor, package, out);
    add_application_consumers(project, anchor, package, &relevant_names, out);
}

fn add_export_edge(
    generated: &str,
    anchor: &str,
    package: &crate::model::PackageInfo,
    manifest: &str,
    export_target: &str,
    out: &mut CodegenLineage,
) {
    let direct_export = repo::normalize_rel_path(export_target.trim_start_matches("./"))
        == repo::normalize_rel_path(
            generated
                .strip_prefix(&format!("{}/", package.path.trim_end_matches('/')))
                .unwrap_or(generated),
        );
    out.edges.push(structural_edge_with_locations(
        package.manifest.clone(),
        anchor.to_string(),
        "exports",
        if direct_export {
            "exact_generated_package_export"
        } else {
            "generated_build_projection_export"
        },
        if direct_export {
            EvidenceStrength::Hard
        } else {
            EvidenceStrength::Medium
        },
        vec![EvidenceLocation::line(
            &package.manifest,
            line_containing(manifest, export_target),
            "package_export",
        )],
    ));
}

fn add_internal_consumers(
    project: &Project,
    generated: &str,
    anchor: &str,
    package: &crate::model::PackageInfo,
    out: &mut CodegenLineage,
) -> BTreeSet<String> {
    let mut names = project
        .files
        .get(generated)
        .map(anchor_symbol_reference_names)
        .unwrap_or_default();
    let mut queue = VecDeque::from([(generated.to_string(), 0usize)]);
    let mut visited = BTreeSet::from([generated.to_string()]);
    while let Some((owner, depth)) = queue.pop_front() {
        if depth >= 4 {
            continue;
        }
        for edge in direct_consumer_edges(project, &owner) {
            let Some(file) = project.files.get(&edge.from) else {
                continue;
            };
            if file.has_role("test")
                || file.has_role("test_support")
                || package_for_rel(project, &file.rel).map(|value| value.path.as_str())
                    != Some(package.path.as_str())
                || !visited.insert(file.rel.clone())
            {
                continue;
            }
            names.extend(anchor_symbol_reference_names(file));
            out.edges.push(structural_edge_with_locations(
                file.rel.clone(),
                anchor.to_string(),
                "consumes",
                if depth == 0 {
                    "direct_generated_artifact_import"
                } else {
                    "mediated_generated_artifact_consumer"
                },
                edge.strength,
                edge.locations,
            ));
            queue.push_back((file.rel.clone(), depth + 1));
        }
    }
    names
}

fn add_application_consumers(
    project: &Project,
    anchor: &str,
    package: &crate::model::PackageInfo,
    relevant_names: &BTreeSet<String>,
    out: &mut CodegenLineage,
) {
    for file in project.files.values().filter(|file| {
        !file.has_role("test")
            && !file.has_role("test_support")
            && repo::is_source_ext(&file.ext)
            && package_for_rel(project, &file.rel).map(|value| value.path.as_str())
                != Some(package.path.as_str())
            && file.imports.iter().any(|spec| {
                spec == &package.name || spec.starts_with(&format!("{}/", package.name))
            })
            && (relevant_names.is_empty()
                || relevant_names
                    .iter()
                    .any(|name| file.references.contains(name)))
    }) {
        out.edges.push(structural_edge_with_locations(
            file.rel.clone(),
            anchor.to_string(),
            "consumes",
            "mediated_generated_package_consumer",
            EvidenceStrength::High,
            vec![EvidenceLocation::path(&file.rel, "package_import")],
        ));
    }
}

fn artifact_projection_key(path: &str, package_path: &str) -> String {
    let normalized = repo::normalize_rel_path(path.trim_start_matches("./"));
    let relative = normalized
        .strip_prefix(&format!("{}/", package_path.trim_end_matches('/')))
        .unwrap_or(&normalized);
    let relative = ["src/", "dist/", "lib/"]
        .into_iter()
        .find_map(|prefix| relative.strip_prefix(prefix))
        .unwrap_or(relative);
    for suffix in [".d.ts", ".d.js", ".ts", ".js", ".mjs", ".cjs"] {
        if let Some(base) = relative.strip_suffix(suffix) {
            return base.to_string();
        }
    }
    relative.to_string()
}

fn line_containing(text: &str, needle: &str) -> usize {
    text.lines()
        .position(|line| line.contains(needle))
        .map(|index| index + 1)
        .unwrap_or(1)
}
