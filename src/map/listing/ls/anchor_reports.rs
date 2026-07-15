// Responsibility: map-listing-ls-anchor-reports
use crate::evidence::import_statement_locations;
use crate::map::{
    FileLsObservationInput, SymbolLsObservationInput, file_ls_observations, file_summary,
    import_edge, missing_symbol_ls_observations, push_symbol_hidden_groups, shell_quote,
    sort_edges, strict_test_edges_for_file, structural_edge_with_locations, symbol_anchor_path,
    symbol_file_summary, symbol_ls_observations, symbol_proof_edges, symbol_reference_edges,
};
use crate::model::{BoundaryFacts, EvidenceStrength, FileInfo, LsReport, Project};

pub(crate) fn ls_symbol_report(
    project: &Project,
    info: &FileInfo,
    symbol_name: &str,
    include_hidden: bool,
    limit: usize,
) -> LsReport {
    let anchor_path = symbol_anchor_path(&info.rel, symbol_name);
    let Some(anchor) = symbol_file_summary(project, info, symbol_name) else {
        return ls_missing_symbol_report(project, &info.rel, symbol_name);
    };
    let consumers = symbol_reference_edges(project, &info.rel, symbol_name, false);
    let verification = symbol_proof_edges(project, &info.rel, symbol_name);
    let consumers_observed = consumers.len();
    let verification_observed = verification.len();
    let mut edges = consumers;
    edges.extend(verification);
    sort_edges(&mut edges);
    edges.dedup_by(|a, b| {
        a.from == b.from && a.to == b.to && a.edge_type == b.edge_type && a.evidence == b.evidence
    });
    if !include_hidden {
        edges.truncate(limit);
    }
    let consumers_shown = edges
        .iter()
        .filter(|edge| edge.edge_type == "symbol_reference")
        .count();
    let verification_shown = edges
        .iter()
        .filter(|edge| edge.edge_type == "tests")
        .count();
    let expand_all = format!("codemap ls {} --all", shell_quote(&anchor_path));
    let observations = symbol_ls_observations(
        project,
        SymbolLsObservationInput {
            file_rel: &info.rel,
            symbol_name,
            consumers_observed,
            consumers_shown,
            consumers_expand: (consumers_shown < consumers_observed).then(|| expand_all.clone()),
            verification_observed,
            verification_shown,
            verification_expand: (verification_shown < verification_observed).then_some(expand_all),
        },
    );
    LsReport {
        kind: "ls_report",
        schema_version: crate::model::LsReport::SCHEMA_VERSION,
        path: anchor_path.clone(),
        mode: "file".to_string(),
        anchor: Some(anchor),
        directory: Vec::new(),
        boundary_facts: BoundaryFacts::default(),
        edges,
        observations,
        hidden: Vec::new(),
        next: vec![format!("codemap cone {}", shell_quote(&anchor_path))],
    }
}

pub(crate) fn ls_missing_symbol_report(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
) -> LsReport {
    let anchor_path = symbol_anchor_path(file_rel, symbol_name);
    LsReport {
        kind: "ls_report",
        schema_version: crate::model::LsReport::SCHEMA_VERSION,
        path: anchor_path.clone(),
        mode: "missing".to_string(),
        anchor: None,
        directory: Vec::new(),
        boundary_facts: BoundaryFacts::default(),
        edges: Vec::new(),
        observations: missing_symbol_ls_observations(project, &anchor_path),
        hidden: Vec::new(),
        next: vec![format!("codemap ls {}", shell_quote(file_rel))],
    }
}

pub(crate) fn ls_file_report(
    project: &Project,
    info: &FileInfo,
    include_hidden: bool,
    limit: usize,
    complete_relationships: bool,
) -> LsReport {
    let mut imports = Vec::new();
    let mut consumers = Vec::new();
    let mut verification = Vec::new();
    if info.content_hash.is_some() {
        for target in &info.resolved_imports {
            imports.push(import_edge(
                project,
                info.rel.clone(),
                target.clone(),
                "imports",
                "resolved_import",
                EvidenceStrength::High,
            ));
        }
        if let Some(importers) = project.reverse_imports.get(&info.rel) {
            for importer in importers.iter().filter(|importer| {
                project
                    .files
                    .get(*importer)
                    .is_none_or(|file| !file.has_role("test") && !file.has_role("test_support"))
            }) {
                consumers.push(import_edge(
                    project,
                    importer.clone(),
                    info.rel.clone(),
                    "imported_by",
                    "reverse_import",
                    EvidenceStrength::High,
                ));
            }
        }
        for (test, evidence, strength) in strict_test_edges_for_file(project, &info.rel, usize::MAX)
        {
            let locations = import_statement_locations(project, &test, &info.rel);
            verification.push(structural_edge_with_locations(
                test,
                info.rel.clone(),
                "tests",
                evidence,
                strength,
                locations,
            ));
        }
    }
    let imports_observed = imports.len();
    let consumers_observed = consumers.len();
    let verification_observed = verification.len();
    let mut edges = imports;
    edges.extend(consumers);
    edges.extend(verification);
    edges.sort_by(|a, b| {
        a.edge_type
            .cmp(&b.edge_type)
            .then_with(|| a.from.cmp(&b.from))
            .then_with(|| a.to.cmp(&b.to))
    });
    edges.dedup_by(|a, b| a.from == b.from && a.to == b.to && a.edge_type == b.edge_type);
    let mut hidden = Vec::new();
    if !include_hidden && !complete_relationships {
        edges.truncate(limit);
    }
    let imports_shown = edges
        .iter()
        .filter(|edge| edge.edge_type == "imports")
        .count();
    let consumers_shown = edges
        .iter()
        .filter(|edge| edge.edge_type == "imported_by")
        .count();
    let verification_shown = edges
        .iter()
        .filter(|edge| edge.edge_type == "tests")
        .count();
    let expand_all = format!("codemap ls {} --all", shell_quote(&info.rel));
    let observations = file_ls_observations(
        project,
        FileLsObservationInput {
            info,
            imports_observed,
            imports_shown,
            imports_expand: (imports_shown < imports_observed).then(|| expand_all.clone()),
            consumers_observed,
            consumers_shown,
            consumers_expand: (consumers_shown < consumers_observed).then(|| expand_all.clone()),
            verification_observed,
            verification_shown,
            verification_expand: (verification_shown < verification_observed)
                .then(|| expand_all.clone()),
        },
    );
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
        observations,
        hidden,
        next: vec![format!("codemap cone {}", shell_quote(&info.rel))],
    }
}
