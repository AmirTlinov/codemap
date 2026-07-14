// Responsibility: runtime-lens-scope
use crate::map::{
    direct_files_under_directory, files_under_directory, route_reference_edges_with_index,
    runtime_fact_index_for_files,
};
use crate::model::{FileInfo, Project, RuntimeRoute, StructuralEdge, Surface};
use std::collections::BTreeSet;

pub(crate) fn runtime_scope_files<'a>(
    project: &'a Project,
    scope: &str,
    include_hidden: bool,
) -> (Vec<&'a FileInfo>, usize) {
    if let Some(file) = project.files.get(scope) {
        return (vec![file], 0);
    }
    let all = files_under_directory(project, scope);
    if scope != "." || include_hidden {
        return (all, 0);
    }

    let mut seen = BTreeSet::new();
    let mut current_level = Vec::new();
    for file in direct_files_under_directory(project, ".")
        .into_iter()
        .chain(files_under_directory(project, ".github"))
    {
        if seen.insert(file.rel.clone()) {
            current_level.push(file);
        }
    }
    let hidden = all.len().saturating_sub(current_level.len());
    (current_level, hidden)
}

// On the root scope, still extract routes from nested files — the most
// signal-bearing, low-volume runtime category — so an agent gets an overview
// without a second round. Entrypoints stay current-level (package containers)
// because they are high-volume; other nested runtime files stay hidden in
// `hidden_scope_count`.
pub(crate) fn extend_root_nested_routes<'a>(
    project: &'a Project,
    current_level: &[&'a FileInfo],
    routes: &mut Vec<RuntimeRoute>,
    proof: &mut Vec<StructuralEdge>,
) {
    let current: BTreeSet<&str> = current_level.iter().map(|file| file.rel.as_str()).collect();
    let nested: Vec<&FileInfo> = files_under_directory(project, ".")
        .into_iter()
        .filter(|file| !current.contains(file.rel.as_str()))
        .collect();
    let facts = runtime_fact_index_for_files(project, nested.iter().copied());
    for file in &nested {
        let file_routes = facts.routes_for_file(&file.rel);
        for route in &file_routes {
            proof.extend(route_reference_edges_with_index(project, route, &facts));
        }
        routes.extend(file_routes);
    }
}

pub(crate) fn dedupe_runtime_entrypoints(values: Vec<Surface>) -> Vec<Surface> {
    let manifest_targets = values
        .iter()
        .filter(|surface| surface.kind == "cli_entrypoint")
        .filter_map(|surface| surface.path.clone())
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for surface in values {
        if surface.kind == "runtime_entrypoint"
            && surface
                .path
                .as_ref()
                .is_some_and(|path| manifest_targets.contains(path))
        {
            continue;
        }
        let key = (
            surface.kind.clone(),
            surface.path.clone().unwrap_or_default(),
            surface.evidence.clone(),
            surface.examples.join("\n"),
        );
        if seen.insert(key) {
            out.push(surface);
        }
    }
    out
}
