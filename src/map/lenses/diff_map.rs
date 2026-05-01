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
    let mut added_runtime_routes = Vec::new();
    let mut removed_runtime_routes = Vec::new();
    let mut added_env = Vec::new();
    let mut removed_env = Vec::new();
    let mut added_proof_surfaces = Vec::new();
    let mut removed_proof_surfaces = Vec::new();
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
        let added_code = diff_current_runtime_code(project, rel, &mode);
        let removed_code = diff_base_runtime_code(project, rel, &mode);
        let added_framework_context = diff_current_file_text(project, rel, &mode)
            .as_deref()
            .map(unsupported_framework_route_context)
            .unwrap_or_default();
        if diff_file_is_added(project, rel, &mode)
            && let Some(route) = runtime_route_from_path_convention(rel)
        {
            added_runtime_routes.push(route);
        }
        if diff_file_is_removed(project, rel, &mode)
            && let Some(route) = runtime_route_from_path_convention(rel)
        {
            removed_runtime_routes.push(route);
        }
        for (line, text) in &delta.added {
            let code = added_code.get(line).map(String::as_str).unwrap_or("");
            if line_looks_like_import_or_reexport(code.trim_start()) {
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
            added_runtime_routes.extend(runtime_routes_from_diff_line(rel, *line, code));
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
            if line_looks_like_import_or_reexport(code.trim_start()) {
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
            if code.trim_start().starts_with("export ") {
                removed_exports.push(surface_from_path(
                    "removed_export",
                    rel,
                    "git_diff_removed_export",
                    EvidenceStrength::Medium,
                ));
            }
            removed_runtime_routes.extend(runtime_routes_from_diff_line(rel, *line, code));
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
        "added proof surfaces hidden by limit",
        &diff_expand,
    );
    truncate_with_hidden(
        &mut removed_proof_surfaces,
        limit,
        &mut hidden,
        "removed proof surfaces hidden by limit",
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
        schema_version: "2",
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

fn diff_file_is_added(project: &Project, rel: &str, mode: &DiffMapMode) -> bool {
    let current_exists = diff_current_file_text(project, rel, mode).is_some();
    let base_exists = diff_base_file_text(project, rel, mode).is_some();
    current_exists && !base_exists
}

fn diff_file_is_removed(project: &Project, rel: &str, mode: &DiffMapMode) -> bool {
    let current_exists = diff_current_file_text(project, rel, mode).is_some();
    let base_exists = diff_base_file_text(project, rel, mode).is_some();
    !current_exists && base_exists
}

fn diff_current_file_text(project: &Project, rel: &str, mode: &DiffMapMode) -> Option<String> {
    match mode {
        DiffMapMode::Staged => git_show_file(project, ":", rel),
        DiffMapMode::WorkingTree | DiffMapMode::Since(_) => {
            std::fs::read_to_string(project.root.join(rel)).ok()
        }
    }
}

fn diff_base_file_text(project: &Project, rel: &str, mode: &DiffMapMode) -> Option<String> {
    let revision = match mode {
        DiffMapMode::WorkingTree | DiffMapMode::Staged => "HEAD",
        DiffMapMode::Since(base) => base.as_str(),
    };
    git_show_file(project, revision, rel)
}

fn runtime_route_from_path_convention(rel: &str) -> Option<RuntimeRoute> {
    let route = if let Some(rest) = next_app_route_rest(rel) {
        next_app_route(rest)
    } else if let Some(rest) = next_pages_route_rest(rel) {
        next_pages_route(rest)
    } else {
        None
    }?;
    Some(RuntimeRoute {
        method: if rel.ends_with("/route.ts") || rel.ends_with("/route.js") {
            Some("ANY".to_string())
        } else {
            Some("GET".to_string())
        },
        path: route,
        file: rel.to_string(),
        evidence: "file_route_convention".to_string(),
        strength: EvidenceStrength::High,
        locations: vec![EvidenceLocation::path(rel, "route_file")],
    })
}

fn runtime_routes_from_diff_line(rel: &str, line_number: usize, code: &str) -> Vec<RuntimeRoute> {
    let ext = std::path::Path::new(rel)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();
    if matches!(ext, "js" | "jsx" | "ts" | "tsx") {
        javascript_route_registrations(rel, code, line_number)
    } else if ext == "py" {
        python_route_decorators(rel, code, line_number)
    } else if ext == "go" {
        go_route_registrations(rel, code, line_number)
    } else {
        Vec::new()
    }
}

fn env_surfaces_from_diff_line(
    project: &Project,
    rel: &str,
    line_number: usize,
    code: &str,
    evidence: &str,
) -> Vec<EnvSurface> {
    static_env_names(code)
        .into_iter()
        .map(|name| EnvSurface {
            name,
            used_by: rel.to_string(),
            declaration: env_declaration(project, rel),
            evidence: evidence.to_string(),
            strength: EvidenceStrength::High,
            locations: vec![EvidenceLocation::line(rel, line_number, "env_reference")],
        })
        .collect()
}

fn proof_surfaces_from_diff_line(
    rel: &str,
    line_number: usize,
    code: &str,
    evidence: &str,
) -> Vec<ProofSurface> {
    if !diff_path_can_carry_proof(rel) {
        return Vec::new();
    }
    e2e_route_visits_from_line(code)
        .into_iter()
        .map(|route| ProofSurface {
            command: None,
            path: Some(rel.to_string()),
            evidence: evidence.to_string(),
            strength: EvidenceStrength::High,
            reason: format!("e2e visits runtime route {route}"),
            locations: vec![EvidenceLocation::line(rel, line_number, "e2e_route")],
        })
        .collect()
}

fn diff_path_can_carry_proof(rel: &str) -> bool {
    let lower = rel.to_ascii_lowercase();
    matches!(
        std::path::Path::new(rel)
            .extension()
            .and_then(|ext| ext.to_str()),
        Some("ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs")
    ) && (lower.contains("/test")
        || lower.contains("/spec")
        || lower.contains("/e2e")
        || lower.contains(".test.")
        || lower.contains(".spec.")
        || lower.contains(".e2e."))
}

fn e2e_route_visits_from_line(line: &str) -> Vec<String> {
    let code = code_shape_without_literal_content(line);
    let mut out = Vec::new();
    for start in find_all(&code, ".goto(") {
        let receiver = code[..start].trim_end();
        if !receiver.ends_with("page") {
            continue;
        }
        let arg_start = start + ".goto(".len();
        if let Some(path) = quoted_literal_at(&line[arg_start..])
            && path.starts_with('/')
        {
            out.push(path);
        }
    }
    out.sort();
    out.dedup();
    out
}

fn dedupe_runtime_routes(routes: &mut Vec<RuntimeRoute>) {
    let mut seen = BTreeSet::new();
    routes.retain(|route| {
        seen.insert((
            route.method.clone(),
            route.path.clone(),
            route.file.clone(),
            route.evidence.clone(),
        ))
    });
}

fn dedupe_env_surfaces(env: &mut Vec<EnvSurface>) {
    let mut seen = BTreeSet::new();
    env.retain(|surface| {
        seen.insert((
            surface.name.clone(),
            surface.used_by.clone(),
            surface.evidence.clone(),
        ))
    });
}

fn dedupe_proof_surfaces(proofs: &mut Vec<ProofSurface>) {
    let mut seen = BTreeSet::new();
    proofs.retain(|proof| {
        seen.insert((
            proof.path.clone(),
            proof.evidence.clone(),
            proof.reason.clone(),
        ))
    });
}
