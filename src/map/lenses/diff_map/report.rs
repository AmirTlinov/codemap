// Responsibility: diff-map-report
use crate::evidence::line_looks_like_import_or_reexport;
use crate::map::{
    DiffMapMode, added_runtime_routes_from_diff_line, changed_symbols_from_delta,
    dedupe_env_surfaces, dedupe_proof_surfaces, dedupe_runtime_routes, diff_base_file_texts,
    diff_current_file_texts, diff_path_needs_runtime_scan, edge_with_path_location,
    env_surfaces_from_diff_line, file_summary, git_unified_zero_deltas, missing_file_summary,
    proof_surfaces_from_diff_line, removed_runtime_routes_from_diff_line, runtime_code_line_lookup,
    runtime_route_from_path_convention, structural_line_target, surface_from_path,
    truncate_with_hidden, unknown_from_added_line, unknown_unindexed_anchor,
    unsupported_framework_route_context,
};
use crate::model::{DiffMapReport, EvidenceStrength, Project};
use crate::repo;

pub fn diff_map_report(
    project: &Project,
    changed: Vec<String>,
    selector: String,
    limit: usize,
    mode: DiffMapMode,
) -> DiffMapReport {
    let limit = limit.max(1);
    let changed = changed
        .into_iter()
        .map(|file| repo::normalize_rel_path(&file))
        .filter(|file| file != ".")
        .collect::<Vec<_>>();
    let mut changed_summaries = Vec::new();
    let mut added_edges = Vec::new();
    let mut removed_edges = Vec::new();
    let mut changed_symbols = Vec::new();
    let mut added_exports = Vec::new();
    let mut removed_exports = Vec::new();
    let mut added_runtime_routes = Vec::new();
    let mut removed_runtime_routes = Vec::new();
    let mut added_env = Vec::new();
    let mut removed_env = Vec::new();
    let mut added_proof_surfaces = Vec::new();
    let mut removed_proof_surfaces = Vec::new();
    let mut new_unknowns = Vec::new();
    let mut hidden = Vec::new();
    let selector = if selector.trim().is_empty() {
        "--changed".to_string()
    } else {
        selector
    };
    let diff_expand = format!("codemap diff-map {selector} --limit <larger-number>");
    let deltas = git_unified_zero_deltas(project, &changed, &mode);
    let text_scan_paths = changed
        .iter()
        .filter(|rel| diff_path_needs_runtime_scan(rel))
        .cloned()
        .collect::<Vec<_>>();
    let current_texts = diff_current_file_texts(project, &text_scan_paths, &mode);
    let base_texts = diff_base_file_texts(project, &text_scan_paths, &mode);
    for rel in &changed {
        let current_text = current_texts.get(rel);
        let base_text = base_texts.get(rel);
        let delta = deltas.get(rel).cloned().unwrap_or_default();
        let added_code = current_text
            .map(String::as_str)
            .map(runtime_code_line_lookup)
            .unwrap_or_default();
        let removed_code = base_text
            .map(String::as_str)
            .map(runtime_code_line_lookup)
            .unwrap_or_default();
        let added_framework_context = current_text
            .map(String::as_str)
            .map(unsupported_framework_route_context)
            .unwrap_or_default();
        if let Some(file) = project.files.get(rel) {
            changed_summaries.push(file_summary(project, file, false, 12));
            changed_symbols.extend(changed_symbols_from_delta(
                rel,
                file,
                &delta,
                &added_code,
                &removed_code,
            ));
        } else {
            changed_summaries.push(missing_file_summary(project, rel));
            new_unknowns.push(unknown_unindexed_anchor(rel));
        }
        if current_text.is_some()
            && base_text.is_none()
            && let Some(route) = runtime_route_from_path_convention(rel)
        {
            added_runtime_routes.push(route);
        }
        if current_text.is_none()
            && base_text.is_some()
            && let Some(route) = runtime_route_from_path_convention(rel)
        {
            removed_runtime_routes.push(route);
        }
        for (line, text) in &delta.added {
            let code = added_code.get(line).map(String::as_str).unwrap_or("");
            if line_looks_like_import_or_reexport(code.trim_start())
                && let Some(target) = structural_line_target(text)
            {
                added_edges.push(edge_with_path_location(
                    rel.clone(),
                    target,
                    "added_structural_line",
                    "git_diff_added_import_or_export",
                    EvidenceStrength::Medium,
                    rel.clone(),
                    format!("diff_added_line:{line}"),
                ));
            }
            if code.trim_start().starts_with("export ") {
                added_exports.push(surface_from_path(
                    "added_export",
                    rel,
                    "git_diff_added_export",
                    EvidenceStrength::Medium,
                ));
            }
            if let Some(unknown) =
                unknown_from_added_line(rel, *line, code, &added_framework_context)
            {
                new_unknowns.push(unknown);
            }
            added_runtime_routes.extend(added_runtime_routes_from_diff_line(
                project, rel, *line, code,
            ));
            added_env.extend(env_surfaces_from_diff_line(
                project,
                rel,
                *line,
                code,
                "added_env_reference",
            ));
            added_proof_surfaces.extend(proof_surfaces_from_diff_line(
                rel,
                *line,
                code,
                "added_e2e_route_visit",
            ));
        }
        for (line, text) in &delta.removed {
            let code = removed_code.get(line).map(String::as_str).unwrap_or("");
            if line_looks_like_import_or_reexport(code.trim_start())
                && let Some(target) = structural_line_target(text)
            {
                removed_edges.push(edge_with_path_location(
                    rel.clone(),
                    target,
                    "removed_structural_line",
                    "git_diff_removed_import_or_export",
                    EvidenceStrength::Medium,
                    rel.clone(),
                    format!("diff_removed_line:{line}"),
                ));
            }
            if code.trim_start().starts_with("export ") {
                removed_exports.push(surface_from_path(
                    "removed_export",
                    rel,
                    "git_diff_removed_export",
                    EvidenceStrength::Medium,
                ));
            }
            removed_runtime_routes.extend(removed_runtime_routes_from_diff_line(
                project, rel, *line, code, &mode,
            ));
            removed_env.extend(env_surfaces_from_diff_line(
                project,
                rel,
                *line,
                code,
                "removed_env_reference",
            ));
            removed_proof_surfaces.extend(proof_surfaces_from_diff_line(
                rel,
                *line,
                code,
                "removed_e2e_route_visit",
            ));
        }
    }
    dedupe_runtime_routes(&mut added_runtime_routes);
    dedupe_runtime_routes(&mut removed_runtime_routes);
    dedupe_env_surfaces(&mut added_env);
    dedupe_env_surfaces(&mut removed_env);
    dedupe_proof_surfaces(&mut added_proof_surfaces);
    dedupe_proof_surfaces(&mut removed_proof_surfaces);
    truncate_with_hidden(
        &mut changed_summaries,
        limit,
        &mut hidden,
        "changed file summaries hidden by limit",
        &diff_expand,
    );
    truncate_with_hidden(
        &mut added_edges,
        limit,
        &mut hidden,
        "added structural edges hidden by limit",
        &diff_expand,
    );
    truncate_with_hidden(
        &mut removed_edges,
        limit,
        &mut hidden,
        "removed structural edges hidden by limit",
        &diff_expand,
    );
    truncate_with_hidden(
        &mut changed_symbols,
        limit,
        &mut hidden,
        "changed symbol surfaces hidden by limit",
        &diff_expand,
    );
    truncate_with_hidden(
        &mut added_exports,
        limit,
        &mut hidden,
        "added export surfaces hidden by limit",
        &diff_expand,
    );
    truncate_with_hidden(
        &mut removed_exports,
        limit,
        &mut hidden,
        "removed export surfaces hidden by limit",
        &diff_expand,
    );
    truncate_with_hidden(
        &mut added_runtime_routes,
        limit,
        &mut hidden,
        "added runtime routes hidden by limit",
        &diff_expand,
    );
    truncate_with_hidden(
        &mut removed_runtime_routes,
        limit,
        &mut hidden,
        "removed runtime routes hidden by limit",
        &diff_expand,
    );
    truncate_with_hidden(
        &mut added_env,
        limit,
        &mut hidden,
        "added env dependencies hidden by limit",
        &diff_expand,
    );
    truncate_with_hidden(
        &mut removed_env,
        limit,
        &mut hidden,
        "removed env dependencies hidden by limit",
        &diff_expand,
    );
    truncate_with_hidden(
        &mut added_proof_surfaces,
        limit,
        &mut hidden,
        "added verification surfaces hidden by limit",
        &diff_expand,
    );
    truncate_with_hidden(
        &mut removed_proof_surfaces,
        limit,
        &mut hidden,
        "removed verification surfaces hidden by limit",
        &diff_expand,
    );
    truncate_with_hidden(
        &mut new_unknowns,
        limit,
        &mut hidden,
        "new unknowns hidden by limit",
        &diff_expand,
    );
    DiffMapReport {
        kind: "diff_map_report",
        schema_version: "3",
        selector: selector.clone(),
        changed: changed_summaries,
        added_edges,
        removed_edges,
        changed_symbols,
        added_exports,
        removed_exports,
        added_runtime_routes,
        removed_runtime_routes,
        added_env,
        removed_env,
        added_proof_surfaces,
        removed_proof_surfaces,
        new_unknowns,
        hidden,
        expand: vec![
            format!("codemap impact {selector}"),
            format!("codemap proof-map {selector}"),
        ],
    }
}
