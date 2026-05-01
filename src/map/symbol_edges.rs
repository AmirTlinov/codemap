#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportedSymbolReferenceKind {
    Direct,
    Reexported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BarrelPublicNameResolution {
    Explicit {
        target_rel: String,
        imported_name: String,
    },
    Star {
        target_rel: String,
    },
}

fn symbol_reference_edges(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
    include_tests: bool,
) -> Vec<StructuralEdge> {
    let Some(anchor) = project.files.get(file_rel) else {
        return Vec::new();
    };
    if matching_symbols(anchor, symbol_name).is_empty() {
        return Vec::new();
    }
    let anchor_path = symbol_anchor_path(file_rel, symbol_name);
    let mut edges = Vec::new();
    for file in project.files.values() {
        if file.rel == file_rel {
            continue;
        }
        if !include_tests && (file.has_role("test") || file.has_role("test_support")) {
            continue;
        }
        if let Some(kind) =
            file_imported_symbol_reference_kind(project, file, file_rel, symbol_name)
        {
            edges.push(StructuralEdge {
                from: file.rel.clone(),
                to: anchor_path.clone(),
                edge_type: "symbol_reference".to_string(),
                evidence: match kind {
                    ImportedSymbolReferenceKind::Direct => "imported_symbol_reference",
                    ImportedSymbolReferenceKind::Reexported => "reexported_symbol_reference",
                }
                .to_string(),
                strength: EvidenceStrength::High,
            });
            continue;
        }
        if same_scope_file_references_symbol(anchor, file, symbol_name) {
            edges.push(StructuralEdge {
                from: file.rel.clone(),
                to: anchor_path.clone(),
                edge_type: "symbol_reference".to_string(),
                evidence: "same_scope_symbol_reference".to_string(),
                strength: EvidenceStrength::High,
            });
        }
    }
    sort_edges(&mut edges);
    edges
}

fn symbol_proof_edges(project: &Project, file_rel: &str, symbol_name: &str) -> Vec<StructuralEdge> {
    let Some(anchor) = project.files.get(file_rel) else {
        return Vec::new();
    };
    if matching_symbols(anchor, symbol_name).is_empty() {
        return Vec::new();
    }
    let anchor_path = symbol_anchor_path(file_rel, symbol_name);
    let mut edges = Vec::new();
    for file in project.files.values() {
        if !file.has_role("test") || file.has_role("test_support") {
            continue;
        }
        let evidence = if let Some(kind) =
            file_imported_symbol_reference_kind(project, file, file_rel, symbol_name)
        {
            Some(match kind {
                ImportedSymbolReferenceKind::Direct => "test_imported_symbol_reference",
                ImportedSymbolReferenceKind::Reexported => "test_reexported_symbol_reference",
            })
        } else if same_scope_file_references_symbol(anchor, file, symbol_name) {
            Some("test_symbol_reference")
        } else {
            None
        };
        if let Some(evidence) = evidence {
            edges.push(StructuralEdge {
                from: file.rel.clone(),
                to: anchor_path.clone(),
                edge_type: "tests".to_string(),
                evidence: evidence.to_string(),
                strength: EvidenceStrength::High,
            });
        }
    }
    sort_edges(&mut edges);
    edges
}

fn symbol_proof_edges_with_owning_file(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
    limit: usize,
) -> Vec<StructuralEdge> {
    let exact = symbol_proof_edges(project, file_rel, symbol_name);
    if !exact.is_empty() {
        return exact;
    }
    let via_local_consumers =
        symbol_proof_edges_via_local_consumers(project, file_rel, symbol_name, limit);
    if !via_local_consumers.is_empty() {
        return via_local_consumers;
    }
    symbol_owning_file_proof_edges(project, file_rel, symbol_name, limit)
}

fn symbol_proof_edges_via_local_consumers(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
    limit: usize,
) -> Vec<StructuralEdge> {
    let Some(info) = project.files.get(file_rel) else {
        return Vec::new();
    };
    let target_anchor = symbol_anchor_path(file_rel, symbol_name);
    let mut edges = Vec::new();
    for consumer in symbol_local_incoming_edges(project, info, symbol_name)
        .into_iter()
        .take(limit)
    {
        let Some((consumer_file, consumer_symbol)) = split_symbol_anchor(&consumer.from) else {
            continue;
        };
        if consumer_file != file_rel {
            continue;
        }
        for proof in symbol_proof_edges(project, file_rel, &consumer_symbol)
            .into_iter()
            .take(limit)
        {
            edges.push(StructuralEdge {
                from: proof.from,
                to: target_anchor.clone(),
                edge_type: "tests".to_string(),
                evidence: format!("{}_via_local_symbol_consumer", proof.evidence),
                strength: proof.strength.min(EvidenceStrength::Medium),
            });
        }
    }
    sort_edges(&mut edges);
    edges.dedup_by(|a, b| {
        a.from == b.from && a.to == b.to && a.edge_type == b.edge_type && a.evidence == b.evidence
    });
    edges
}

fn symbol_owning_file_proof_edges(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
    limit: usize,
) -> Vec<StructuralEdge> {
    let Some(anchor) = project.files.get(file_rel) else {
        return Vec::new();
    };
    if !matching_symbols(anchor, symbol_name)
        .into_iter()
        .any(|symbol| symbol.exported)
    {
        return Vec::new();
    }
    let anchor_path = symbol_anchor_path(file_rel, symbol_name);
    strict_test_edges_for_file(project, file_rel, limit)
        .into_iter()
        .filter(|(test, evidence, _)| {
            symbol_owning_file_proof_can_use(project, file_rel, symbol_name, test, evidence)
        })
        .map(|(test, evidence, strength)| StructuralEdge {
            from: test,
            to: anchor_path.clone(),
            edge_type: "tests".to_string(),
            evidence: format!("{evidence}_owning_file"),
            strength: strength.min(EvidenceStrength::Medium),
        })
        .collect()
}

fn symbol_owning_file_proof_can_use(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
    test_rel: &str,
    evidence: &str,
) -> bool {
    if !matches!(
        evidence,
        "e2e_path_surface" | "e2e_surface_phrase" | "test_surface_phrase"
    ) {
        return false;
    }
    let symbol_terms = semantic_name_terms(symbol_name);
    if symbol_terms.is_empty() {
        return false;
    }
    let Some(anchor) = project.files.get(file_rel) else {
        return false;
    };
    let path_terms = semantic_path_terms(file_rel);
    let mut sibling_terms = BTreeSet::new();
    for symbol in &anchor.symbols {
        if symbol.name != symbol_name && symbol.exported {
            sibling_terms.extend(semantic_name_terms(&symbol.name));
        }
    }
    for export in &anchor.exports {
        if export != symbol_name {
            sibling_terms.extend(semantic_name_terms(export));
        }
    }
    let distinctive_symbol_terms = symbol_terms
        .difference(&path_terms)
        .filter(|term| !sibling_terms.contains(*term))
        .cloned()
        .collect::<BTreeSet<_>>();
    if distinctive_symbol_terms.is_empty() {
        return false;
    }
    let Some(test) = project.files.get(test_rel) else {
        return false;
    };
    let mut test_terms = test_surface_terms(test);
    for phrase in &test.surface_phrases {
        test_terms.extend(surface_phrase_terms(phrase));
    }
    !distinctive_symbol_terms.is_disjoint(&test_terms)
}

fn symbol_outgoing_edges(
    project: &Project,
    info: &FileInfo,
    symbol_name: &str,
) -> Vec<StructuralEdge> {
    if !repo::is_source_ext(&info.ext) {
        return Vec::new();
    }
    let Some(body) = symbol_body_text(project, info, symbol_name) else {
        return Vec::new();
    };
    let mut edges = Vec::new();
    for (target_rel, bindings) in &info.resolved_import_bindings {
        for (local, imported) in bindings {
            if file_has_local_value_shadow(info, local) {
                continue;
            }
            if !symbol_body_references_imported_local(&body, local, &info.ext) {
                continue;
            }
            let Some(target_symbol) =
                imported_binding_target_symbol_name(project, target_rel, imported)
            else {
                continue;
            };
            edges.push(StructuralEdge {
                from: symbol_anchor_path(&info.rel, symbol_name),
                to: symbol_anchor_path(target_rel, &target_symbol),
                edge_type: "symbol_uses".to_string(),
                evidence: "imported_symbol_in_symbol_body".to_string(),
                strength: EvidenceStrength::High,
            });
        }
    }
    edges.extend(symbol_local_outgoing_edges(
        project,
        info,
        symbol_name,
        &body,
    ));
    sort_edges(&mut edges);
    edges
}

fn symbol_local_outgoing_edges(
    project: &Project,
    info: &FileInfo,
    symbol_name: &str,
    body: &str,
) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    for target in &info.symbols {
        if target.name == symbol_name {
            continue;
        }
        if !symbol_is_local_symbol_use_target(project, info, target) {
            continue;
        }
        if symbol_body_declares_js_local_binding(body, &target.name, &info.ext) {
            continue;
        }
        if !symbol_body_references_imported_local(body, &target.name, &info.ext) {
            continue;
        }
        edges.push(StructuralEdge {
            from: symbol_anchor_path(&info.rel, symbol_name),
            to: symbol_anchor_path(&info.rel, &target.name),
            edge_type: "symbol_uses".to_string(),
            evidence: "local_symbol_in_symbol_body".to_string(),
            strength: EvidenceStrength::High,
        });
    }
    edges
}

fn symbol_local_incoming_edges(
    project: &Project,
    info: &FileInfo,
    symbol_name: &str,
) -> Vec<StructuralEdge> {
    if !matching_symbols(info, symbol_name)
        .into_iter()
        .any(|symbol| symbol_is_local_symbol_use_target(project, info, &symbol))
    {
        return Vec::new();
    }
    let mut edges = Vec::new();
    for source in &info.symbols {
        if source.name == symbol_name {
            continue;
        }
        if !symbol_is_top_level(project, info, source) {
            continue;
        }
        let Some(body) = symbol_body_text(project, info, &source.name) else {
            continue;
        };
        if symbol_body_declares_js_local_binding(&body, symbol_name, &info.ext) {
            continue;
        }
        if !symbol_body_references_imported_local(&body, symbol_name, &info.ext) {
            continue;
        }
        edges.push(StructuralEdge {
            from: symbol_anchor_path(&info.rel, &source.name),
            to: symbol_anchor_path(&info.rel, symbol_name),
            edge_type: "symbol_uses".to_string(),
            evidence: "local_symbol_in_symbol_body".to_string(),
            strength: EvidenceStrength::High,
        });
    }
    sort_edges(&mut edges);
    edges.dedup_by(|a, b| {
        a.from == b.from && a.to == b.to && a.edge_type == b.edge_type && a.evidence == b.evidence
    });
    edges
}

fn symbol_is_local_symbol_use_target(
    project: &Project,
    info: &FileInfo,
    symbol: &crate::model::SymbolInfo,
) -> bool {
    symbol_is_top_level(project, info, symbol) && symbol.kind != "method"
}

fn symbol_is_top_level(
    project: &Project,
    info: &FileInfo,
    symbol: &crate::model::SymbolInfo,
) -> bool {
    let Ok(text) = std::fs::read_to_string(project.root.join(&info.rel)) else {
        return true;
    };
    text.lines()
        .nth(symbol.line_start.saturating_sub(1))
        .map(|line| {
            line.chars()
                .take_while(|ch| ch.is_ascii_whitespace())
                .count()
                == 0
        })
        .unwrap_or(true)
}
