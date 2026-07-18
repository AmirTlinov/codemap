// Responsibility: symbol-cone-cross-file-traversal
use crate::map::{
    matching_symbols, shell_quote, sort_edges, structural_edge_with_locations, symbol_anchor_path,
    symbol_outgoing_edges,
};
use crate::model::{EvidenceStrength, Project, StructuralEdge};
use std::collections::{BTreeSet, VecDeque};
use std::path::Path;

pub(crate) fn symbol_cone_outgoing_edges(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
    max_depth: usize,
) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::from([(file_rel.to_string(), symbol_name.to_string(), 0usize)]);
    while let Some((rel, symbol, depth)) = queue.pop_front() {
        if !seen.insert((rel.clone(), symbol.clone())) {
            continue;
        }
        let Some(info) = project.files.get(&rel) else {
            continue;
        };
        let next = symbol_outgoing_edges(project, info, &symbol);
        if depth + 1 < max_depth.max(1) {
            for edge in &next {
                let Some((target_rel, target_symbol)) = edge.to.rsplit_once('#') else {
                    continue;
                };
                if project
                    .files
                    .get(target_rel)
                    .is_some_and(|target| !matching_symbols(target, target_symbol).is_empty())
                {
                    queue.push_back((target_rel.to_string(), target_symbol.to_string(), depth + 1));
                }
            }
        }
        edges.extend(next);
    }
    if max_depth <= 1 {
        edges.extend(local_helper_implementation_edges(
            project,
            file_rel,
            symbol_name,
        ));
    }
    sort_edges(&mut edges);
    edges.dedup_by(|left, right| {
        left.from == right.from
            && left.to == right.to
            && left.edge_type == right.edge_type
            && left.evidence == right.evidence
    });
    edges
}

fn local_helper_implementation_edges(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
) -> Vec<StructuralEdge> {
    let source = symbol_anchor_path(file_rel, symbol_name);
    let mut queue = VecDeque::from([symbol_name.to_string()]);
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    while let Some(symbol) = queue.pop_front() {
        if seen.len() >= 16 || !seen.insert(symbol.clone()) {
            continue;
        }
        let Some(info) = project.files.get(file_rel) else {
            break;
        };
        for edge in symbol_outgoing_edges(project, info, &symbol) {
            let Some((target_file, target_symbol)) = edge.to.rsplit_once('#') else {
                continue;
            };
            if target_file == file_rel {
                if !seen.contains(target_symbol) {
                    queue.push_back(target_symbol.to_string());
                }
                continue;
            }
            if !split_implementation_file(file_rel, target_file) {
                continue;
            }
            out.push(structural_edge_with_locations(
                source.clone(),
                edge.to,
                "symbol_uses",
                format!("{}_via_local_helper", edge.evidence),
                edge.strength.min(EvidenceStrength::Medium),
                edge.locations,
            ));
        }
    }
    sort_edges(&mut out);
    out.truncate(6);
    out
}

fn split_implementation_file(anchor_file: &str, target_file: &str) -> bool {
    let anchor = Path::new(anchor_file);
    if anchor.extension().and_then(|value| value.to_str()) != Some("rs")
        || anchor.file_stem().and_then(|value| value.to_str()) == Some("mod")
    {
        return false;
    }
    let implementation_dir = anchor.with_extension("");
    Path::new(target_file).starts_with(implementation_dir)
}

pub(crate) fn symbol_cone_expands(
    file_rel: &str,
    anchor_path: &str,
    depth: usize,
    contracts: &[StructuralEdge],
) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut expands = vec![format!(
        "codemap cone {} --depth {}",
        shell_quote(anchor_path),
        depth + 1
    )];
    expands.extend(
        contracts
            .iter()
            .map(|edge| edge.to.as_str())
            .filter(|target| *target != file_rel && seen.insert((*target).to_string()))
            .take(2)
            .map(|target| format!("codemap contract {}", shell_quote(target))),
    );
    expands.push(format!("codemap ls {}", shell_quote(file_rel)));
    expands
}
