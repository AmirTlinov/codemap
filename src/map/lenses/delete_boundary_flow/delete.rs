// Responsibility: delete-lens-report
use crate::map::{
    cone_proof_edges, direct_consumer_edges, file_summary, limit_edge_section,
    missing_file_summary, package_export_edges, runtime_reference_edges, shell_quote,
    split_symbol_anchor, symbol_file_summary, symbol_proof_edges, symbol_reference_edges,
    truncate_with_hidden, unknown_missing_symbol_anchor, unknown_unindexed_anchor,
    unknowns_for_file,
};
use crate::model::{DeleteReport, Project};
use crate::repo;

pub fn delete_report(
    project: &Project,
    anchor_path: &str,
    include_hidden: bool,
    limit: usize,
) -> DeleteReport {
    let limit = limit.max(1);
    let rel = repo::normalize_rel_path(anchor_path);
    let (file_rel, symbol) =
        split_symbol_anchor(&rel).unwrap_or_else(|| (rel.clone(), String::new()));
    if !project.files.contains_key(&file_rel) {
        return DeleteReport {
            kind: "delete_report",
            schema_version: "3",
            anchor: missing_file_summary(project, &rel),
            direct_users: Vec::new(),
            symbol_users: Vec::new(),
            reexports: Vec::new(),
            package_exports: Vec::new(),
            tests: Vec::new(),
            runtime_refs: Vec::new(),
            unknowns: vec![unknown_unindexed_anchor(&file_rel)],
            checklist: Vec::new(),
            hidden: Vec::new(),
            expand: vec![format!("codemap ls {}", shell_quote(&file_rel))],
        };
    }
    if !symbol.is_empty()
        && project
            .files
            .get(&file_rel)
            .and_then(|file| symbol_file_summary(project, file, &symbol))
            .is_none()
    {
        return DeleteReport {
            kind: "delete_report",
            schema_version: "3",
            anchor: missing_file_summary(project, &rel),
            direct_users: Vec::new(),
            symbol_users: Vec::new(),
            reexports: Vec::new(),
            package_exports: Vec::new(),
            tests: Vec::new(),
            runtime_refs: Vec::new(),
            unknowns: vec![unknown_missing_symbol_anchor(&file_rel, &symbol)],
            checklist: Vec::new(),
            hidden: Vec::new(),
            expand: vec![format!("codemap ls {}", shell_quote(&file_rel))],
        };
    }
    let anchor = project
        .files
        .get(&file_rel)
        .map(|file| {
            if symbol.is_empty() {
                file_summary(project, file, include_hidden, 20)
            } else {
                symbol_file_summary(project, file, &symbol)
                    .unwrap_or_else(|| file_summary(project, file, include_hidden, 20))
            }
        })
        .unwrap_or_else(|| missing_file_summary(project, &file_rel));
    let mut direct_users = direct_consumer_edges(project, &file_rel);
    let mut symbol_users = if symbol.is_empty() {
        Vec::new()
    } else {
        symbol_reference_edges(project, &file_rel, &symbol, true)
    };
    let mut reexports = symbol_users
        .iter()
        .filter(|edge| edge.evidence.contains("reexport"))
        .cloned()
        .collect::<Vec<_>>();
    let mut package_exports = package_export_edges(project, &file_rel);
    let mut tests = if symbol.is_empty() {
        cone_proof_edges(project, std::slice::from_ref(&file_rel))
    } else {
        symbol_proof_edges(project, &file_rel, &symbol)
    };
    let mut runtime_refs = runtime_reference_edges(project, &file_rel);
    let mut hidden = Vec::new();
    for (edges, reason) in [
        (&mut direct_users, "direct users hidden by limit"),
        (&mut symbol_users, "symbol users hidden by limit"),
        (&mut reexports, "reexports hidden by limit"),
        (&mut package_exports, "package exports hidden by limit"),
        (&mut tests, "tests hidden by limit"),
        (&mut runtime_refs, "runtime refs hidden by limit"),
    ] {
        limit_edge_section(
            edges,
            &mut hidden,
            include_hidden,
            limit,
            reason,
            &format!("codemap delete {} --all", shell_quote(&rel)),
        );
    }
    let mut checklist = Vec::new();
    if !direct_users.is_empty() {
        checklist.push("update direct users shown above".to_string());
    }
    if !symbol_users.is_empty() {
        checklist.push("update symbol references shown above".to_string());
    }
    if !reexports.is_empty() {
        checklist.push("remove or update barrel reexports shown above".to_string());
    }
    if !package_exports.is_empty() {
        checklist.push("remove or update package public exports shown above".to_string());
    }
    if !tests.is_empty() {
        checklist.push("update direct linked verification surfaces shown above".to_string());
    }
    if !runtime_refs.is_empty() {
        checklist.push("inspect runtime references shown above".to_string());
    }
    let mut unknowns = project
        .files
        .get(&file_rel)
        .map(|file| unknowns_for_file(project, file))
        .unwrap_or_default();
    truncate_with_hidden(
        &mut unknowns,
        limit,
        &mut hidden,
        "delete unknowns hidden by limit",
        &format!("codemap delete {} --all", shell_quote(&rel)),
    );
    DeleteReport {
        kind: "delete_report",
        schema_version: "3",
        anchor,
        direct_users,
        symbol_users,
        reexports,
        package_exports,
        tests,
        runtime_refs,
        unknowns,
        checklist,
        hidden,
        expand: vec![format!("codemap cone {}", shell_quote(&file_rel))],
    }
}
