// Responsibility: map-symbols-symbol-uses
use crate::map::{
    file_has_local_value_shadow, first_identifier_reference_location,
    imported_binding_target_symbol_name, matching_symbols, sort_edges,
    structural_edge_with_locations, symbol_anchor_path, symbol_body_declares_js_local_binding,
    symbol_body_references_imported_local, symbol_body_text,
};
use crate::model::{EvidenceStrength, FileInfo, Project, StructuralEdge};
use crate::repo;

pub(crate) fn symbol_outgoing_edges(
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
            edges.push(structural_edge_with_locations(
                symbol_anchor_path(&info.rel, symbol_name),
                symbol_anchor_path(target_rel, &target_symbol),
                "symbol_uses",
                "imported_symbol_in_symbol_body",
                EvidenceStrength::High,
                first_identifier_reference_location(
                    project,
                    &info.rel,
                    local,
                    "symbol_body_reference",
                ),
            ));
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
        edges.push(structural_edge_with_locations(
            symbol_anchor_path(&info.rel, symbol_name),
            symbol_anchor_path(&info.rel, &target.name),
            "symbol_uses",
            "local_symbol_in_symbol_body",
            EvidenceStrength::High,
            first_identifier_reference_location(
                project,
                &info.rel,
                &target.name,
                "symbol_body_reference",
            ),
        ));
    }
    edges
}

pub(crate) fn symbol_local_incoming_edges(
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
        edges.push(structural_edge_with_locations(
            symbol_anchor_path(&info.rel, &source.name),
            symbol_anchor_path(&info.rel, symbol_name),
            "symbol_uses",
            "local_symbol_in_symbol_body",
            EvidenceStrength::High,
            first_identifier_reference_location(
                project,
                &info.rel,
                symbol_name,
                "symbol_body_reference",
            ),
        ));
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

pub(crate) fn symbol_is_top_level(
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
