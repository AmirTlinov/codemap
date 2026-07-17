// Responsibility: map-symbols-symbol-uses
use crate::map::{
    BarrelResolutionCache, barrel_reexports_symbol_from_file, file_has_local_value_shadow,
    first_identifier_reference_location, imported_binding_target_symbol_name, matching_symbols,
    module_binding_matches_target, rust_qualified_symbol_body_references, sort_edges,
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
            if symbol_body_references_imported_local(&body, local, &info.ext)
                && let Some(target_symbol) =
                    imported_binding_target_symbol_name(project, target_rel, imported)
            {
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
            if module_binding_matches_target(target_rel, imported)
                && !file_has_local_value_shadow(info, local)
            {
                edges.extend(rust_module_member_edges(
                    project,
                    info,
                    symbol_name,
                    target_rel,
                    local,
                ));
            }
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

fn rust_module_member_edges(
    project: &Project,
    info: &FileInfo,
    symbol_name: &str,
    module_rel: &str,
    local: &str,
) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    let mut barrel_cache = BarrelResolutionCache::default();
    for (member, line) in rust_qualified_symbol_body_references(project, info, symbol_name, local) {
        let Some((owner_rel, reexported)) =
            rust_module_member_owner(project, module_rel, &member, &mut barrel_cache)
        else {
            continue;
        };
        edges.push(structural_edge_with_locations(
            symbol_anchor_path(&info.rel, symbol_name),
            symbol_anchor_path(&owner_rel, &member),
            "symbol_uses",
            if reexported {
                "reexported_module_symbol_in_symbol_body"
            } else {
                "module_symbol_in_symbol_body"
            },
            EvidenceStrength::High,
            vec![crate::model::EvidenceLocation::line(
                &info.rel,
                line,
                "symbol_body_reference",
            )],
        ));
    }
    edges
}

fn rust_module_member_owner(
    project: &Project,
    module_rel: &str,
    member: &str,
    barrel_cache: &mut BarrelResolutionCache,
) -> Option<(String, bool)> {
    let module = project.files.get(module_rel)?;
    if file_has_qualified_member_value(module, member) {
        return Some((module_rel.to_string(), false));
    }
    let mut owners = project
        .files
        .values()
        .filter(|file| file_has_qualified_member_value(file, member))
        .filter(|file| {
            barrel_reexports_symbol_from_file(
                project,
                module,
                &file.rel,
                member,
                member,
                barrel_cache,
            )
        })
        .map(|file| file.rel.clone())
        .collect::<Vec<_>>();
    owners.sort();
    owners.dedup();
    (owners.len() == 1).then(|| (owners.remove(0), true))
}

fn file_has_qualified_member_value(file: &FileInfo, member: &str) -> bool {
    matching_symbols(file, member)
        .into_iter()
        .any(|symbol| symbol.kind != "module")
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
    let Some(text) = project.read_indexed_text(&info.rel) else {
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
