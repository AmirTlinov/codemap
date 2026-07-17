// Responsibility: map-listing-ls-anchor-reports
use crate::map::{
    FileLsObservationInput, SymbolLsObservationInput, cone_proof_edges_with_direct_consumers,
    file_ls_observations, file_summary, import_edge, missing_symbol_ls_observations,
    proof_evidence_precedence, shell_quote, sort_edges, symbol_anchor_path, symbol_file_summary,
    symbol_ls_observations, symbol_proof_edges, symbol_reference_edges,
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
    let mut consumers = symbol_reference_edges(project, &info.rel, symbol_name, false);
    let mut verification = symbol_proof_edges(project, &info.rel, symbol_name);
    let consumers_observed = consumers.len();
    let verification_observed = verification.len();
    sort_edges(&mut consumers);
    sort_edges(&mut verification);
    let expand_all = format!("codemap ls {} --all", shell_quote(&anchor_path));
    let quota = if include_hidden { usize::MAX } else { limit };
    consumers = bounded_ls_group("consumers", consumers, quota, &expand_all);
    verification = bounded_ls_group("verification", verification, quota, &expand_all);
    let mut edges = consumers;
    edges.extend(verification);
    sort_edges(&mut edges);
    edges.dedup_by(|a, b| {
        a.from == b.from && a.to == b.to && a.edge_type == b.edge_type && a.evidence == b.evidence
    });
    let consumers_shown = edges
        .iter()
        .filter(|edge| edge.edge_type == "symbol_reference")
        .count();
    let verification_shown = edges
        .iter()
        .filter(|edge| edge.edge_type == "tests")
        .count();
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
    complete_file_projection: bool,
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
        verification =
            cone_proof_edges_with_direct_consumers(project, std::slice::from_ref(&info.rel));
    }
    let imports_observed = imports.len();
    let consumers_observed = consumers.len();
    let verification_observed = verification.len();
    let expand_all = format!("codemap ls {} --all", shell_quote(&info.rel));
    let quota = if include_hidden || complete_file_projection {
        usize::MAX
    } else {
        limit
    };
    sort_edges(&mut imports);
    sort_edges(&mut consumers);
    verification.sort_by(verification_edge_order);
    imports = bounded_ls_group("imports", imports, quota, &expand_all);
    consumers = bounded_ls_group("consumers", consumers, quota, &expand_all);
    verification = bounded_ls_group("verification", verification, quota, &expand_all);
    let mut edges = imports;
    edges.extend(consumers);
    edges.extend(verification);
    edges.sort_by(|a, b| {
        a.edge_type
            .cmp(&b.edge_type)
            .then_with(|| {
                if a.edge_type == "tests" {
                    verification_edge_order(a, b)
                } else {
                    a.from.cmp(&b.from)
                }
            })
            .then_with(|| a.to.cmp(&b.to))
    });
    edges.dedup_by(|a, b| a.from == b.from && a.to == b.to && a.edge_type == b.edge_type);
    let hidden = Vec::new();
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
    let anchor = file_summary(
        project,
        info,
        include_hidden || complete_file_projection,
        limit,
    );
    let symbols_observed = info.symbols.len();
    let symbols_shown = anchor.symbols.len();
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
            symbols_observed,
            symbols_shown,
            symbols_expand: (symbols_shown < symbols_observed).then(|| expand_all.clone()),
        },
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

fn verification_edge_order(
    a: &crate::model::StructuralEdge,
    b: &crate::model::StructuralEdge,
) -> std::cmp::Ordering {
    b.strength
        .cmp(&a.strength)
        .then_with(|| {
            (a.evidence == "test_role_surface_match")
                .cmp(&(b.evidence == "test_role_surface_match"))
        })
        .then_with(|| {
            proof_evidence_precedence(&b.evidence).cmp(&proof_evidence_precedence(&a.evidence))
        })
        .then_with(|| a.from.cmp(&b.from))
}

fn bounded_ls_group(
    group: &str,
    edges: Vec<crate::model::StructuralEdge>,
    limit: usize,
    expand: &str,
) -> Vec<crate::model::StructuralEdge> {
    crate::map::BoundedProjection::ordered(group, edges, limit, expand).into_shown()
}
