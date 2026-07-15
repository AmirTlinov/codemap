// Responsibility: map-edges
use crate::evidence::import_statement_locations;
use crate::model::{EvidenceLocation, EvidenceStrength, Project, StructuralEdge};

pub(crate) fn structural_edge_with_locations(
    from: impl Into<String>,
    to: impl Into<String>,
    edge_type: impl Into<String>,
    evidence: impl Into<String>,
    strength: EvidenceStrength,
    locations: Vec<EvidenceLocation>,
) -> StructuralEdge {
    StructuralEdge {
        from: from.into(),
        to: to.into(),
        edge_type: edge_type.into(),
        evidence: evidence.into(),
        strength,
        locations,
    }
}

pub(crate) fn edge_with_path_location(
    from: impl Into<String>,
    to: impl Into<String>,
    edge_type: impl Into<String>,
    evidence: impl Into<String>,
    strength: EvidenceStrength,
    path: impl Into<String>,
    kind: impl Into<String>,
) -> StructuralEdge {
    structural_edge_with_locations(
        from,
        to,
        edge_type,
        evidence,
        strength,
        vec![EvidenceLocation::path(path, kind)],
    )
}

pub(crate) fn edge_with_aggregate_location(
    from: impl Into<String>,
    to: impl Into<String>,
    edge_type: impl Into<String>,
    evidence: impl Into<String>,
    strength: EvidenceStrength,
    kind: impl Into<String>,
) -> StructuralEdge {
    structural_edge_with_locations(
        from,
        to,
        edge_type,
        evidence,
        strength,
        vec![EvidenceLocation::aggregate(kind)],
    )
}

pub(crate) fn import_edge(
    project: &Project,
    from: impl Into<String>,
    to: impl Into<String>,
    edge_type: impl Into<String>,
    evidence: impl Into<String>,
    strength: EvidenceStrength,
) -> StructuralEdge {
    let from = from.into();
    let to = to.into();
    let locations = import_statement_locations(project, &from, &to);
    structural_edge_with_locations(from, to, edge_type, evidence, strength, locations)
}

pub(crate) fn symbol_definition_location(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
    kind: &str,
) -> Vec<EvidenceLocation> {
    project
        .files
        .get(file_rel)
        .and_then(|file| {
            file.symbols
                .iter()
                .find(|symbol| symbol.name == symbol_name)
                .map(|symbol| {
                    vec![EvidenceLocation {
                        path: file_rel.to_string(),
                        line_start: Some(symbol.line_start),
                        line_end: Some(symbol.line_end),
                        kind: kind.to_string(),
                    }]
                })
        })
        .unwrap_or_else(|| vec![EvidenceLocation::path(file_rel, kind)])
}

pub(crate) fn first_identifier_reference_location(
    project: &Project,
    file_rel: &str,
    name: &str,
    kind: &str,
) -> Vec<EvidenceLocation> {
    let Some(text) = project.read_indexed_text(file_rel) else {
        return vec![EvidenceLocation::path(file_rel, kind)];
    };
    for (index, line) in text.lines().enumerate() {
        if line.contains(name) {
            return vec![EvidenceLocation::line(file_rel, index + 1, kind)];
        }
    }
    vec![EvidenceLocation::path(file_rel, kind)]
}
