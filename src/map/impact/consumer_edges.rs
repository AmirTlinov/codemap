// Responsibility: map-impact-consumer-edges
use crate::map::{
    anchor_symbol_reference_names, first_identifier_reference_location, import_edge, sort_edges,
    structural_edge_with_locations,
};
use crate::model::{EvidenceStrength, FileInfo, Project, StructuralEdge};
use crate::repo;
use std::path::Path;

pub(crate) fn direct_consumer_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let mut edges = project
        .reverse_imports
        .get(rel)
        .into_iter()
        .flat_map(|importers| importers.iter())
        .filter(|importer| {
            project
                .files
                .get(*importer)
                .map(|file| !file.has_role("test"))
                .unwrap_or(true)
        })
        .map(|importer| {
            import_edge(
                project,
                importer.clone(),
                rel.to_string(),
                "direct_consumer",
                "reverse_import",
                EvidenceStrength::High,
            )
        })
        .collect::<Vec<_>>();
    edges.extend(same_package_symbol_reference_consumers(project, rel));
    sort_edges(&mut edges);
    edges
}

pub(crate) fn direct_dependency_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let Some(file) = project.files.get(rel) else {
        return Vec::new();
    };
    let mut edges = file
        .resolved_imports
        .iter()
        .filter(|dependency| project.files.contains_key(*dependency))
        .map(|dependency| {
            import_edge(
                project,
                rel.to_string(),
                dependency.clone(),
                "direct_dependency",
                "resolved_import",
                EvidenceStrength::High,
            )
        })
        .collect::<Vec<_>>();
    sort_edges(&mut edges);
    edges
}

pub(crate) fn same_package_symbol_reference_consumers(
    project: &Project,
    rel: &str,
) -> Vec<StructuralEdge> {
    let Some(anchor) = project.files.get(rel) else {
        return Vec::new();
    };
    let names = anchor_symbol_reference_names(anchor);
    if names.is_empty() {
        return Vec::new();
    }
    project
        .files
        .values()
        .filter(|file| file.rel != rel)
        .filter(|file| !file.has_role("test") && !file.has_role("test_support"))
        .filter(|file| !file.resolved_imports.contains(rel))
        .filter(|file| same_symbol_reference_scope(anchor, file))
        .filter(|file| names.iter().any(|name| file.references.contains(name)))
        .map(|file| {
            let name = names.iter().next().map(String::as_str).unwrap_or(rel);
            structural_edge_with_locations(
                file.rel.clone(),
                rel.to_string(),
                "direct_consumer",
                "same_package_symbol_reference",
                EvidenceStrength::High,
                first_identifier_reference_location(project, &file.rel, name, "symbol_reference"),
            )
        })
        .collect()
}

pub(crate) fn same_symbol_reference_scope(anchor: &FileInfo, file: &FileInfo) -> bool {
    if anchor.ext == "go" && file.ext == "go" {
        return Path::new(&anchor.rel).parent() == Path::new(&file.rel).parent();
    }
    if anchor.ext == "swift" && file.ext == "swift" {
        return same_swift_target_reference_scope(anchor, file);
    }
    false
}

fn same_swift_target_reference_scope(anchor: &FileInfo, file: &FileInfo) -> bool {
    let Some((anchor_root, anchor_target)) = swift_source_scope(&anchor.rel) else {
        return false;
    };
    if file.has_role("test") {
        return swift_test_package_root(&file.rel)
            .map(|test_root| test_root == anchor_root)
            .unwrap_or(false)
            && file.imports.contains(&anchor_target);
    }
    swift_source_scope(&file.rel)
        .map(|scope| scope == (anchor_root, anchor_target))
        .unwrap_or(false)
}

pub(crate) fn swift_source_scope(rel: &str) -> Option<(String, String)> {
    let normalized = repo::normalize_rel_path(rel);
    if let Some(rest) = normalized.strip_prefix("Sources/") {
        return rest
            .split('/')
            .next()
            .map(|target| (".".to_string(), target.to_string()));
    }
    if let Some((root, rest)) = normalized.split_once("/Sources/") {
        return rest
            .split('/')
            .next()
            .map(|target| (root.to_string(), target.to_string()));
    }
    None
}

pub(crate) fn swift_test_package_root(rel: &str) -> Option<String> {
    let normalized = repo::normalize_rel_path(rel);
    if normalized.starts_with("Tests/") {
        return Some(".".to_string());
    }
    normalized
        .split_once("/Tests/")
        .map(|(root, _)| root.to_string())
}
