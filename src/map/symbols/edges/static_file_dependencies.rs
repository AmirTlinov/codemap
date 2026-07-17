// Responsibility: map-symbols-static-file-dependencies
use crate::map::{structural_edge_with_locations, symbol_anchor_path, symbol_body_text};
use crate::model::{EvidenceLocation, EvidenceStrength, FileInfo, Project, StructuralEdge};
use crate::repo;
use std::collections::BTreeSet;

pub(crate) fn rust_static_file_dependency_edges(
    project: &Project,
    info: &FileInfo,
    symbol_name: &str,
) -> Vec<StructuralEdge> {
    if info.ext != "rs" {
        return Vec::new();
    }
    let Some(body) = symbol_body_text(project, info, symbol_name) else {
        return Vec::new();
    };
    let paths = project.files.keys().cloned().collect::<BTreeSet<_>>();
    repo::extract_rust_include_specs(&body)
        .into_iter()
        .filter_map(|spec| {
            let target = repo::resolve_rust(&info.rel, &spec, &paths, &project.packages)?;
            let line = symbol_body_dependency_line(info, symbol_name, &body, &spec)?;
            Some(structural_edge_with_locations(
                symbol_anchor_path(&info.rel, symbol_name),
                target,
                "embeds_file",
                "rust_static_include_in_symbol_body",
                EvidenceStrength::High,
                vec![EvidenceLocation::line(
                    &info.rel,
                    line,
                    "rust_static_include_in_symbol_body",
                )],
            ))
        })
        .collect()
}

fn symbol_body_dependency_line(
    info: &FileInfo,
    symbol_name: &str,
    body: &str,
    spec: &str,
) -> Option<usize> {
    let start = info
        .symbols
        .iter()
        .filter(|symbol| symbol.name == symbol_name)
        .map(|symbol| symbol.line_start)
        .min()?;
    body.lines()
        .position(|line| line.contains(spec))
        .map(|offset| start + offset)
}
