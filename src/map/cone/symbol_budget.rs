// Responsibility: readable attention budget for direct symbol calls
use crate::model::StructuralEdge;
use std::path::Path;

const OUTGOING_ATTENTION_CHARS: usize = 900;
const SHORT_ENDPOINT_CHARS: usize = 48;

pub(crate) fn symbol_outgoing_limit(
    edges: &[StructuralEdge],
    anchor_file: &str,
    requested: usize,
) -> usize {
    let requested = requested.min(edges.len());
    if edges
        .iter()
        .take(requested)
        .all(|edge| endpoint_display_len(&edge.to, anchor_file) <= SHORT_ENDPOINT_CHARS)
    {
        return requested;
    }
    let mut chars = 0;
    let mut shown = 0;
    for edge in edges.iter().take(requested) {
        let cost = 24
            + endpoint_display_len(&edge.to, anchor_file)
            + edge.edge_type.chars().count()
            + edge.evidence.chars().count();
        if shown > 0 && chars + cost > OUTGOING_ATTENTION_CHARS {
            break;
        }
        chars += cost;
        shown += 1;
    }
    shown
}

fn endpoint_display_len(endpoint: &str, anchor_file: &str) -> usize {
    let (file, symbol) = endpoint
        .split_once('#')
        .map_or((endpoint, None), |(file, symbol)| (file, Some(symbol)));
    let symbol_len = symbol.map_or(0, |symbol| symbol.chars().count() + 1);
    if file == anchor_file {
        return 5 + symbol_len;
    }
    let parent = Path::new(anchor_file)
        .parent()
        .and_then(Path::to_str)
        .filter(|parent| !parent.is_empty());
    if let Some(relative) = parent.and_then(|parent| file.strip_prefix(&format!("{parent}/"))) {
        return 2 + relative.chars().count() + symbol_len;
    }
    endpoint.chars().count().min(88)
}
