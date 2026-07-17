// Responsibility: symbol-cone-cross-file-traversal
use crate::map::{matching_symbols, sort_edges, symbol_outgoing_edges};
use crate::model::{Project, StructuralEdge};
use std::collections::{BTreeSet, VecDeque};

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
    sort_edges(&mut edges);
    edges.dedup_by(|left, right| {
        left.from == right.from
            && left.to == right.to
            && left.edge_type == right.edge_type
            && left.evidence == right.evidence
    });
    edges
}
