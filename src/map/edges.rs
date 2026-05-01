fn structural_edge_with_locations(
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

fn edge_with_path_location(
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

fn edge_with_aggregate_location(
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

fn import_edge(
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

fn import_statement_locations(project: &Project, from: &str, to: &str) -> Vec<EvidenceLocation> {
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
        if names.iter().any(|name| !name.is_empty() && line.contains(name)) {
            locations.push(EvidenceLocation::line(
                from,
                index + 1,
                "import_statement",
            ));
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

fn line_looks_like_import_or_reexport(trimmed: &str) -> bool {
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

fn symbol_definition_location(
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

fn first_identifier_reference_location(
    project: &Project,
    file_rel: &str,
    name: &str,
    kind: &str,
) -> Vec<EvidenceLocation> {
    let Ok(text) = std::fs::read_to_string(project.root.join(file_rel)) else {
        return vec![EvidenceLocation::path(file_rel, kind)];
    };
    for (index, line) in text.lines().enumerate() {
        if line.contains(name) {
            return vec![EvidenceLocation::line(file_rel, index + 1, kind)];
        }
    }
    vec![EvidenceLocation::path(file_rel, kind)]
}
