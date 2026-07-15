// Responsibility: map-listing-ls-anchor-reports
use crate::evidence::import_statement_locations;
use crate::map::{
    file_summary, import_edge, push_symbol_hidden_groups, shell_quote, sort_edges,
    strict_test_edges_for_file, structural_edge_with_locations, symbol_anchor_path,
    symbol_file_summary, symbol_proof_edges, symbol_reference_edges,
};
use crate::model::{BoundaryFacts, EvidenceStrength, FileInfo, HiddenGroup, LsReport, Project};

pub(crate) fn ls_symbol_report(
    project: &Project,
    info: &FileInfo,
    symbol_name: &str,
    include_hidden: bool,
    limit: usize,
) -> LsReport {
    let anchor_path = symbol_anchor_path(&info.rel, symbol_name);
    let Some(anchor) = symbol_file_summary(project, info, symbol_name) else {
        return LsReport {
            kind: "ls_report",
            schema_version: crate::model::LsReport::SCHEMA_VERSION,
            path: anchor_path.clone(),
            mode: "missing".to_string(),
            anchor: None,
            directory: Vec::new(),
            boundary_facts: BoundaryFacts::default(),
            edges: Vec::new(),
            hidden: Vec::new(),
            next: vec![format!("codemap ls {}", shell_quote(&info.rel))],
        };
    };
    let mut edges = symbol_reference_edges(project, &info.rel, symbol_name, false);
    edges.extend(symbol_proof_edges(project, &info.rel, symbol_name));
    sort_edges(&mut edges);
    edges.dedup_by(|a, b| {
        a.from == b.from && a.to == b.to && a.edge_type == b.edge_type && a.evidence == b.evidence
    });
    let edge_count = edges.len();
    let mut hidden = Vec::new();
    if !include_hidden {
        edges.truncate(limit);
        if edge_count > edges.len() {
            hidden.push(HiddenGroup {
                reason: "symbol edges hidden by limit".to_string(),
                count: edge_count - edges.len(),
                expand: format!("codemap ls {} --all", shell_quote(&anchor_path)),
            });
        }
    }
    LsReport {
        kind: "ls_report",
        schema_version: crate::model::LsReport::SCHEMA_VERSION,
        path: anchor_path.clone(),
        mode: "file".to_string(),
        anchor: Some(anchor),
        directory: Vec::new(),
        boundary_facts: BoundaryFacts::default(),
        edges,
        hidden,
        next: vec![format!("codemap cone {}", shell_quote(&anchor_path))],
    }
}

pub(crate) fn ls_file_report(
    project: &Project,
    info: &FileInfo,
    include_hidden: bool,
    limit: usize,
) -> LsReport {
    let mut edges = Vec::new();
    for target in &info.resolved_imports {
        edges.push(import_edge(
            project,
            info.rel.clone(),
            target.clone(),
            "imports",
            "resolved_import",
            EvidenceStrength::High,
        ));
    }
    if let Some(importers) = project.reverse_imports.get(&info.rel) {
        for importer in importers {
            edges.push(import_edge(
                project,
                importer.clone(),
                info.rel.clone(),
                "imported_by",
                "reverse_import",
                EvidenceStrength::High,
            ));
        }
    }
    for (test, evidence, strength) in strict_test_edges_for_file(project, &info.rel, usize::MAX) {
        let locations = import_statement_locations(project, &test, &info.rel);
        edges.push(structural_edge_with_locations(
            test,
            info.rel.clone(),
            "tests",
            evidence,
            strength,
            locations,
        ));
    }
    edges.sort_by(|a, b| {
        a.edge_type
            .cmp(&b.edge_type)
            .then_with(|| a.from.cmp(&b.from))
            .then_with(|| a.to.cmp(&b.to))
    });
    edges.dedup_by(|a, b| a.from == b.from && a.to == b.to && a.edge_type == b.edge_type);
    let edge_count = edges.len();
    let mut hidden = Vec::new();
    if !include_hidden {
        edges.truncate(limit);
        if edge_count > edges.len() {
            hidden.push(HiddenGroup {
                reason: "edges hidden by limit".to_string(),
                count: edge_count - edges.len(),
                expand: format!("codemap cone {} --depth 1", shell_quote(&info.rel)),
            });
        }
    }
    let anchor = file_summary(project, info, include_hidden, limit);
    push_symbol_hidden_groups(
        &mut hidden,
        info,
        include_hidden,
        limit,
        &format!("codemap ls {} --all", shell_quote(&info.rel)),
    );
    LsReport {
        kind: "ls_report",
        schema_version: crate::model::LsReport::SCHEMA_VERSION,
        path: info.rel.clone(),
        mode: "file".to_string(),
        anchor: Some(anchor),
        directory: Vec::new(),
        boundary_facts: BoundaryFacts::default(),
        edges,
        hidden,
        next: vec![format!("codemap cone {}", shell_quote(&info.rel))],
    }
}
