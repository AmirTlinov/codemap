fn ls_symbol_report(
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
            schema_version: "3",
            path: anchor_path.clone(),
            mode: "missing".to_string(),
            anchor: None,
            directory: Vec::new(),
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
        schema_version: "3",
        path: anchor_path.clone(),
        mode: "file".to_string(),
        anchor: Some(anchor),
        directory: Vec::new(),
        edges,
        hidden,
        next: vec![format!("codemap cone {}", shell_quote(&anchor_path))],
    }
}

fn ls_file_report(
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
        schema_version: "3",
        path: info.rel.clone(),
        mode: "file".to_string(),
        anchor: Some(anchor),
        directory: Vec::new(),
        edges,
        hidden,
        next: vec![format!("codemap cone {}", shell_quote(&info.rel))],
    }
}

fn ls_directory_report(
    project: &Project,
    rel: &str,
    include_hidden: bool,
    limit: usize,
) -> LsReport {
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for domain in &project.domains {
        if path_under_scope(&domain.path, rel) {
            grouped
                .entry("domain".to_string())
                .or_default()
                .push(domain.path.clone());
        }
    }
    for package in &project.packages {
        if path_under_scope(&package.path, rel) || path_under_scope(&package.manifest, rel) {
            let package_is_support = is_support_artifact_path(&package.path)
                || is_support_artifact_path(&package.manifest);
            let scope_is_support = is_support_artifact_path(rel);
            if package_is_support && !scope_is_support && !include_hidden {
                grouped
                    .entry("support_package_hidden".to_string())
                    .or_default()
                    .push(package.manifest.clone());
                continue;
            }
            let kind = if package_is_support && !scope_is_support {
                format!("support_package:{}", package.ecosystem)
            } else {
                format!("package:{}", package.ecosystem)
            };
            grouped
                .entry(kind)
                .or_default()
                .push(package.manifest.clone());
        }
    }
    for script in &project.scripts {
        if rel == "." {
            grouped
                .entry("script".to_string())
                .or_default()
                .push(format!("{}: {}", script.name, script.command));
        }
    }
    let direct_files = direct_files_under_directory(project, rel);
    let scope_is_support = is_support_artifact_path(rel);
    let mut hidden_support_artifact_count = 0;
    for dir in immediate_child_dirs(project, rel) {
        if is_support_artifact_path(&dir) && !scope_is_support && !include_hidden {
            hidden_support_artifact_count += 1;
            continue;
        }
        if let Some(kind) = directory_role_surface(project, &dir) {
            grouped.entry(kind).or_default().push(dir.clone());
        }
        grouped.entry("dir".to_string()).or_default().push(dir);
    }
    for file in &direct_files {
        let kind = file_kind_for_ls(file);
        let noisy = is_generic_noise(file);
        if noisy && !include_hidden {
            grouped
                .entry("generic_hidden".to_string())
                .or_default()
                .push(file.rel.clone());
            continue;
        }
        grouped.entry(kind).or_default().push(file.rel.clone());
    }
    let recursive_files = files_under_directory(project, rel)
        .into_iter()
        .filter(|file| !direct_files.iter().any(|direct| direct.rel == file.rel))
        .collect::<Vec<_>>();
    if include_hidden {
        for file in &recursive_files {
            let kind = format!("recursive:{}", file_kind_for_ls(file));
            grouped.entry(kind).or_default().push(file.rel.clone());
        }
    }
    let hidden_generic_count = grouped
        .remove("generic_hidden")
        .map(|v| v.len())
        .unwrap_or(0);
    let hidden_support_package_count = grouped
        .remove("support_package_hidden")
        .map(|v| v.len())
        .unwrap_or(0);
    let mut surfaces = grouped
        .into_iter()
        .map(|(kind, mut files)| {
            files.sort();
            let count = files.len();
            let examples = files.into_iter().take(5).collect::<Vec<_>>();
            DirectorySurface {
                id: directory_surface_id(rel, &kind, &examples),
                path: directory_surface_path(&examples),
                role: directory_surface_role(&kind),
                evidence: directory_surface_evidence(&kind),
                strength: directory_surface_strength(&kind),
                kind,
                count,
                examples,
                hidden_count: count.saturating_sub(5),
            }
        })
        .collect::<Vec<_>>();
    surfaces.sort_by(|a, b| {
        surface_priority(&a.kind)
            .cmp(&surface_priority(&b.kind))
            .then_with(|| b.count.cmp(&a.count))
            .then_with(|| a.kind.cmp(&b.kind))
    });
    let surface_count = surfaces.len();
    surfaces.truncate(limit);
    let mut hidden = Vec::new();
    let mut edges = directory_edges(project, rel, include_hidden);
    let edge_count = edges.len();
    if !include_hidden {
        edges = balanced_edge_prefix_by_source(&edges, limit);
    }
    if edge_count > edges.len() {
        hidden.push(HiddenGroup {
            reason: "directory edges hidden by limit".to_string(),
            count: edge_count - edges.len(),
            expand: format!("codemap ls {} --all", shell_quote(rel)),
        });
    }
    if surface_count > surfaces.len() {
        hidden.push(HiddenGroup {
            reason: "directory surfaces hidden by limit".to_string(),
            count: surface_count - surfaces.len(),
            expand: format!("codemap ls {} --all", shell_quote(rel)),
        });
    }
    if hidden_generic_count > 0 {
        hidden.push(HiddenGroup {
            reason: "generic source files hidden".to_string(),
            count: hidden_generic_count,
            expand: format!("codemap ls {} --all", shell_quote(rel)),
        });
    }
    if hidden_support_package_count > 0 {
        hidden.push(HiddenGroup {
            reason: "support packages hidden below support scopes".to_string(),
            count: hidden_support_package_count,
            expand: format!("codemap ls {} --all", shell_quote(rel)),
        });
    }
    if hidden_support_artifact_count > 0 {
        hidden.push(HiddenGroup {
            reason: "support artifacts hidden".to_string(),
            count: hidden_support_artifact_count,
            expand: format!("codemap ls {} --all", shell_quote(rel)),
        });
    }
    if !include_hidden && !recursive_files.is_empty() {
        hidden.push(HiddenGroup {
            reason: "recursive files below this level hidden".to_string(),
            count: recursive_files.len(),
            expand: format!("codemap ls {} --all", shell_quote(rel)),
        });
    }
    LsReport {
        kind: "ls_report",
        schema_version: "3",
        path: rel.to_string(),
        mode: "directory".to_string(),
        anchor: None,
        directory: surfaces,
        edges,
        hidden,
        next: directory_next_commands(rel),
    }
}

fn directory_next_commands(rel: &str) -> Vec<String> {
    let graph = if rel == "." {
        "codemap graph --lens causal".to_string()
    } else {
        format!("codemap graph --path {} --lens causal", shell_quote(rel))
    };
    vec![graph, format!("codemap cone {} --depth 1", shell_quote(rel))]
}

fn directory_surface_id(scope: &str, kind: &str, examples: &[String]) -> String {
    let anchor = examples
        .first()
        .map(String::as_str)
        .unwrap_or(scope)
        .trim_end_matches('/');
    format!("surface:{kind}:{anchor}")
}

fn directory_surface_path(examples: &[String]) -> Option<String> {
    (examples.len() == 1).then(|| examples[0].clone())
}

fn directory_surface_role(kind: &str) -> Option<String> {
    if kind == "domain" {
        Some("domain".to_string())
    } else if kind == "script" {
        Some("script".to_string())
    } else if kind.starts_with("package:") || kind.starts_with("support_package:") {
        Some("package".to_string())
    } else if matches!(
        kind,
        "test"
            | "e2e_test"
            | "test_support"
            | "source"
            | "snapshot"
            | "golden"
            | "asset"
            | "config"
            | "docs"
            | "env_config"
            | "runtime_config"
            | "manifest"
            | "lockfile"
            | "schema_contract"
            | "schema"
            | "public_boundary"
            | "public_api"
            | "internal_api"
            | "build_ci"
            | "deploy"
            | "entrypoint"
            | "runtime_surface"
            | "application"
            | "service"
            | "domain"
            | "controller"
            | "module"
            | "repository"
            | "adapter"
            | "parser"
            | "renderer_ui"
            | "persistence"
            | "package_graph"
            | "role_classifier"
            | "script_catalog"
            | "cli_surface"
            | "map_surface"
            | "extractor"
            | "config_loader"
            | "evidence_surface"
            | "repo_discovery"
            | "cache"
            | "semantic_anchor"
            | "agent_bootstrap"
            | "fixture"
            | "example"
            | "generated"
            | "archive"
            | "witness"
            | "receipt"
            | "proof_runner"
            | "owner_doc"
            | "migration"
            | "build_output"
    ) {
        Some(kind.to_string())
    } else {
        None
    }
}

fn directory_surface_evidence(kind: &str) -> String {
    if kind == "domain" {
        "domain_boundary".to_string()
    } else if kind == "script" {
        "package_script".to_string()
    } else if kind.starts_with("package:") || kind.starts_with("support_package:") {
        "package_manifest".to_string()
    } else if kind == "manifest" {
        "manifest_file".to_string()
    } else if kind == "env_config" {
        "env_file".to_string()
    } else if kind == "runtime_config" {
        "runtime_config_file".to_string()
    } else if kind == "lockfile" {
        "lockfile".to_string()
    } else if kind.starts_with("recursive:") {
        "recursive_inventory".to_string()
    } else {
        "file_role_or_extension".to_string()
    }
}

fn directory_surface_strength(kind: &str) -> EvidenceStrength {
    if kind == "script" || kind.starts_with("package:") || kind.starts_with("support_package:") {
        EvidenceStrength::Hard
    } else if kind == "domain"
        || kind == "schema_contract"
        || kind == "public_boundary"
        || kind == "manifest"
        || kind == "env_config"
        || kind == "runtime_config"
        || kind == "lockfile"
        || kind == "build_ci"
    {
        EvidenceStrength::High
    } else {
        EvidenceStrength::Medium
    }
}
