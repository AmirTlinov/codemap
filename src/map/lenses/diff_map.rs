pub enum DiffMapMode {
    WorkingTree,
    Staged,
    Since(String),
}

pub fn diff_map_report(
    project: &Project,
    changed: Vec<String>,
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
    let mut new_unknowns = Vec::new();
    let mut hidden = Vec::new();
    let selector = diff_map_selector(&changed, &mode);
    let diff_expand = format!("codemap diff-map {selector} --limit <larger-number>");
    for rel in &changed {
        if let Some(file) = project.files.get(rel) {
            changed_summaries.push(file_summary(project, file, false, 12));
            for symbol in file.symbols.iter().filter(|symbol| symbol.exported) {
                changed_symbols.push(ChangedSymbol {
                    path: rel.clone(),
                    name: symbol.name.clone(),
                    change: "file_changed_symbol_surface".to_string(),
                    line_start: Some(symbol.line_start),
                    line_end: Some(symbol.line_end),
                });
            }
        } else {
            changed_summaries.push(missing_file_summary(project, rel));
            new_unknowns.push(unknown_unindexed_anchor(rel));
        }
        let delta = git_unified_zero_delta(project, rel, &mode);
        for (line, text) in &delta.added {
            if line_looks_like_import_or_reexport(text.trim_start()) {
                added_edges.push(edge_with_path_location(
                    rel.clone(),
                    structural_line_target(text),
                    "added_structural_line",
                    "git_diff_added_import_or_export",
                    EvidenceStrength::Medium,
                    rel.clone(),
                    format!("diff_added_line:{line}"),
                ));
            }
            if text.trim_start().starts_with("export ") {
                added_exports.push(surface_from_path(
                    "added_export",
                    rel,
                    "git_diff_added_export",
                    EvidenceStrength::Medium,
                ));
            }
            if let Some(unknown) = unknown_from_added_line(rel, *line, text) {
                new_unknowns.push(unknown);
            }
        }
        for (line, text) in &delta.removed {
            if line_looks_like_import_or_reexport(text.trim_start()) {
                removed_edges.push(edge_with_path_location(
                    rel.clone(),
                    structural_line_target(text),
                    "removed_structural_line",
                    "git_diff_removed_import_or_export",
                    EvidenceStrength::Medium,
                    rel.clone(),
                    format!("diff_removed_line:{line}"),
                ));
            }
            if text.trim_start().starts_with("export ") {
                removed_exports.push(surface_from_path(
                    "removed_export",
                    rel,
                    "git_diff_removed_export",
                    EvidenceStrength::Medium,
                ));
            }
        }
    }
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
        &mut new_unknowns,
        limit,
        &mut hidden,
        "new unknowns hidden by limit",
        &diff_expand,
    );
    DiffMapReport {
        kind: "diff_map_report",
        schema_version: "1",
        changed: changed_summaries,
        added_edges,
        removed_edges,
        changed_symbols,
        added_exports,
        removed_exports,
        new_unknowns,
        hidden,
        expand: vec![
            format!("codemap impact {selector}"),
            format!("codemap proof-map {selector}"),
        ],
    }
}

fn diff_map_selector(changed: &[String], mode: &DiffMapMode) -> String {
    match mode {
        DiffMapMode::Staged => "--staged".to_string(),
        DiffMapMode::Since(since) => format!("--since {}", shell_quote(since)),
        DiffMapMode::WorkingTree => changed_snapshot_selector(changed),
    }
}

fn changed_snapshot_selector(changed: &[String]) -> String {
    if changed.is_empty() {
        return "--changed".to_string();
    }
    let files = changed
        .iter()
        .map(|file| shell_quote(file))
        .collect::<Vec<_>>()
        .join(",");
    format!("--files {files}")
}
