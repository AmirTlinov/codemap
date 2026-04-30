use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

use serde::Serialize;

use crate::cache;
use crate::model::{
    BoundaryFinding, BoundaryReport, ConeReport, DirectorySurface, Domain, EvidenceStrength,
    FileInfo, FileSummary, HiddenGroup, ImpactCluster, ImpactReport, LsReport, Project,
    ProofReport, ProofSurface, Risk, StructuralEdge, VerificationPlan,
};
use crate::repo;

mod graph_lens;
pub use graph_lens::graph_lens;

#[derive(Debug, Serialize)]
pub struct StatusReport {
    pub kind: &'static str,
    pub schema_version: &'static str,
    pub root: String,
    pub cwd: String,
    pub vcs: Option<String>,
    pub config: Option<String>,
    pub config_errors: Vec<String>,
    pub nearest_agents: Option<String>,
    pub cache_dir: String,
    pub cache_state: String,
    pub cache_artifacts: Vec<crate::model::CacheArtifactStatus>,
    pub zero_footprint_default: bool,
    pub package_manager: String,
    pub languages: Vec<String>,
    pub files_scanned: usize,
    pub domains: Vec<DomainStatus>,
    pub scripts: Vec<String>,
    pub fingerprint: String,
    pub boundary_findings: usize,
    pub unclassified_source_files: Vec<String>,
    pub unclassified_count: usize,
}

#[derive(Debug, Serialize)]
pub struct DomainStatus {
    pub id: String,
    pub path: String,
    pub config: Option<String>,
}

pub fn status_report(project: &Project) -> StatusReport {
    let unclassified: Vec<String> = project
        .files
        .values()
        .filter(|file| repo::is_source_ext(&file.ext) && file.roles.is_empty())
        .map(|file| file.rel.clone())
        .collect();
    StatusReport {
        kind: "status_report",
        schema_version: "2",
        root: project.root.to_string_lossy().to_string(),
        cwd: project.cwd.to_string_lossy().to_string(),
        vcs: project.vcs.clone(),
        config: project.config_path.clone(),
        config_errors: project
            .config_errors
            .iter()
            .map(|error| format!("{}: {}", error.path, error.error))
            .collect(),
        nearest_agents: project.nearest_agents.clone(),
        cache_dir: project.cache_dir.to_string_lossy().to_string(),
        cache_state: project.cache_state.clone(),
        cache_artifacts: project.cache_artifacts.clone(),
        zero_footprint_default: true,
        package_manager: project.package_manager.clone(),
        languages: project.languages.iter().cloned().collect(),
        files_scanned: project.files.len(),
        domains: project
            .domains
            .iter()
            .map(|d| DomainStatus {
                id: d.id.clone(),
                path: d.path.clone(),
                config: d.config_path.clone(),
            })
            .collect(),
        scripts: project.scripts.iter().map(|s| s.command.clone()).collect(),
        fingerprint: cache::fingerprint(project, None),
        boundary_findings: boundary_findings(project, None).len(),
        unclassified_count: unclassified.len(),
        unclassified_source_files: unclassified.into_iter().take(30).collect(),
    }
}

pub fn ls_report(project: &Project, path: &str, include_hidden: bool, limit: usize) -> LsReport {
    let rel = repo::normalize_rel_path(path);
    if let Some(info) = project.files.get(&rel) {
        return ls_file_report(project, info, include_hidden, limit.max(1));
    }
    if directory_has_files(project, &rel) {
        return ls_directory_report(project, &rel, include_hidden, limit.max(1));
    }
    LsReport {
        kind: "ls_report",
        schema_version: "2",
        path: rel.clone(),
        mode: "missing".to_string(),
        anchor: None,
        directory: Vec::new(),
        edges: Vec::new(),
        hidden: Vec::new(),
        next: vec![format!(
            "codemap ls {}",
            shell_quote(&parent_anchor_for_missing(&rel))
        )],
    }
}

pub fn cone_report(
    project: &Project,
    path: &str,
    depth: usize,
    include_hidden: bool,
    limit: usize,
) -> ConeReport {
    let rel = repo::normalize_rel_path(path);
    let depth = depth.min(4);
    let limit = limit.max(1);
    let (anchor, seed_files, mut unknowns, mut hidden) =
        cone_anchor(project, &rel, include_hidden, limit);
    let depths = cone_depths(project, &seed_files, depth);
    let mut outgoing = cone_outgoing_edges(project, &depths, depth);
    let mut incoming = cone_incoming_edges(project, &seed_files);
    let mut proof = cone_proof_edges_with_direct_consumers(project, &seed_files);
    let mut contracts = cone_contract_edges(project, &outgoing);
    let mut boundary = cone_boundary_edges(project, &rel, &depths);
    sort_edges(&mut outgoing);
    sort_edges(&mut incoming);
    sort_edges(&mut proof);
    sort_edges(&mut contracts);
    sort_edges(&mut boundary);
    limit_edge_section(
        &mut outgoing,
        &mut hidden,
        include_hidden,
        limit,
        "outgoing edges hidden by limit",
        &format!(
            "codemap cone {} --depth {depth} --include-hidden",
            shell_quote(&rel)
        ),
    );
    limit_edge_section(
        &mut incoming,
        &mut hidden,
        include_hidden,
        limit,
        "incoming edges hidden by limit",
        &format!(
            "codemap cone {} --depth {depth} --include-hidden",
            shell_quote(&rel)
        ),
    );
    limit_edge_section(
        &mut proof,
        &mut hidden,
        include_hidden,
        limit,
        "proof edges hidden by limit",
        &format!(
            "codemap cone {} --depth {depth} --include-hidden",
            shell_quote(&rel)
        ),
    );
    limit_edge_section(
        &mut contracts,
        &mut hidden,
        include_hidden,
        limit,
        "contract edges hidden by limit",
        &format!(
            "codemap cone {} --depth {depth} --include-hidden",
            shell_quote(&rel)
        ),
    );
    limit_edge_section(
        &mut boundary,
        &mut hidden,
        include_hidden,
        limit,
        "boundary edges hidden by limit",
        &format!(
            "codemap cone {} --depth {depth} --include-hidden",
            shell_quote(&rel)
        ),
    );
    if seed_files.is_empty() {
        unknowns.push("anchor is not indexed as a file or directory".to_string());
    }
    ConeReport {
        kind: "cone_report",
        schema_version: "2",
        anchor,
        depth,
        outgoing,
        incoming,
        proof,
        contracts,
        boundary,
        hidden,
        unknowns,
        expand: vec![
            format!("codemap cone {} --depth {}", shell_quote(&rel), depth + 1),
            format!("codemap ls {} --include-hidden", shell_quote(&rel)),
        ],
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
        edges.push(StructuralEdge {
            from: info.rel.clone(),
            to: target.clone(),
            edge_type: "imports".to_string(),
            evidence: "resolved_import".to_string(),
            strength: EvidenceStrength::High,
        });
    }
    if let Some(importers) = project.reverse_imports.get(&info.rel) {
        for importer in importers {
            edges.push(StructuralEdge {
                from: importer.clone(),
                to: info.rel.clone(),
                edge_type: "imported_by".to_string(),
                evidence: "reverse_import".to_string(),
                strength: EvidenceStrength::High,
            });
        }
    }
    for (test, evidence, strength) in strict_test_edges_for_file(project, &info.rel, 4) {
        edges.push(StructuralEdge {
            from: test,
            to: info.rel.clone(),
            edge_type: "tests".to_string(),
            evidence,
            strength,
        });
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
    if !include_hidden && info.symbols.len() > anchor.symbols.len() {
        hidden.push(HiddenGroup {
            reason: "symbols hidden by limit".to_string(),
            count: info.symbols.len() - anchor.symbols.len(),
            expand: format!("codemap ls {} --include-hidden", shell_quote(&info.rel)),
        });
    }
    LsReport {
        kind: "ls_report",
        schema_version: "2",
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
    for dir in immediate_child_dirs(project, rel) {
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
            DirectorySurface {
                kind,
                count: files.len(),
                examples: files.into_iter().take(5).collect(),
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
            expand: format!("codemap ls {} --include-hidden", shell_quote(rel)),
        });
    }
    if surface_count > surfaces.len() {
        hidden.push(HiddenGroup {
            reason: "directory surfaces hidden by limit".to_string(),
            count: surface_count - surfaces.len(),
            expand: format!("codemap ls {} --include-hidden", shell_quote(rel)),
        });
    }
    if hidden_generic_count > 0 {
        hidden.push(HiddenGroup {
            reason: "generic source files hidden".to_string(),
            count: hidden_generic_count,
            expand: format!("codemap ls {} --include-hidden", shell_quote(rel)),
        });
    }
    if hidden_support_package_count > 0 {
        hidden.push(HiddenGroup {
            reason: "support packages hidden below fixture/example/sample scopes".to_string(),
            count: hidden_support_package_count,
            expand: format!("codemap ls {} --include-hidden", shell_quote(rel)),
        });
    }
    if !include_hidden && !recursive_files.is_empty() {
        hidden.push(HiddenGroup {
            reason: "recursive files below this level hidden".to_string(),
            count: recursive_files.len(),
            expand: format!("codemap ls {} --include-hidden", shell_quote(rel)),
        });
    }
    LsReport {
        kind: "ls_report",
        schema_version: "2",
        path: rel.to_string(),
        mode: "directory".to_string(),
        anchor: None,
        directory: surfaces,
        edges,
        hidden,
        next: vec![format!("codemap cone {}", shell_quote(rel))],
    }
}

fn directory_edges(project: &Project, rel: &str, include_hidden: bool) -> Vec<StructuralEdge> {
    let mut grouped: BTreeMap<(String, String, String, String, EvidenceStrength), usize> =
        BTreeMap::new();
    let scope_is_support = is_support_artifact_path(rel);
    for file in files_under_directory(project, rel) {
        for target in &file.resolved_imports {
            if !include_hidden
                && !scope_is_support
                && (is_support_artifact_path(&file.rel) || is_support_artifact_path(target))
            {
                continue;
            }
            let from = directory_edge_endpoint(rel, &file.rel);
            let to = directory_edge_endpoint(rel, target);
            if from != to {
                add_directory_edge(
                    &mut grouped,
                    from,
                    to,
                    "outgoing_import",
                    "resolved_import",
                    EvidenceStrength::High,
                );
            }
        }
        if let Some(importers) = project.reverse_imports.get(&file.rel) {
            for importer in importers {
                if path_under_scope(importer, rel) {
                    continue;
                }
                if !include_hidden
                    && !scope_is_support
                    && (is_support_artifact_path(&file.rel) || is_support_artifact_path(importer))
                {
                    continue;
                }
                let from = directory_edge_endpoint(rel, importer);
                let to = directory_edge_endpoint(rel, &file.rel);
                if from != to {
                    add_directory_edge(
                        &mut grouped,
                        from,
                        to,
                        "incoming_import",
                        "reverse_import",
                        EvidenceStrength::High,
                    );
                }
            }
        }
    }
    for edge in &project.package_edges {
        if !include_hidden
            && !scope_is_support
            && (is_support_artifact_path(&edge.from_manifest)
                || edge
                    .to_manifest
                    .as_ref()
                    .map(|to| is_support_artifact_path(to))
                    .unwrap_or_else(|| is_support_artifact_path(&edge.to)))
        {
            continue;
        }
        let from_in = path_under_scope(&edge.from_manifest, rel);
        let to_in = edge
            .to_manifest
            .as_ref()
            .map(|to| path_under_scope(to, rel))
            .unwrap_or_else(|| path_under_scope(&edge.to, rel));
        if from_in || to_in {
            add_directory_edge(
                &mut grouped,
                directory_edge_endpoint(rel, &edge.from_manifest),
                directory_edge_endpoint(
                    rel,
                    &edge.to_manifest.clone().unwrap_or_else(|| edge.to.clone()),
                ),
                if from_in && to_in {
                    "package_internal"
                } else if from_in {
                    "package_outgoing"
                } else {
                    "package_incoming"
                },
                &format!("package_manifest:{}", edge.dependency),
                EvidenceStrength::High,
            );
        }
    }
    let mut edges = grouped
        .into_iter()
        .map(
            |((from, to, edge_type, evidence, strength), count)| StructuralEdge {
                from,
                to,
                edge_type,
                evidence: if count > 1 {
                    format!("{evidence}:{count}")
                } else {
                    evidence
                },
                strength,
            },
        )
        .collect::<Vec<_>>();
    sort_edges(&mut edges);
    edges
}

fn add_directory_edge(
    grouped: &mut BTreeMap<(String, String, String, String, EvidenceStrength), usize>,
    from: String,
    to: String,
    edge_type: &str,
    evidence: &str,
    strength: EvidenceStrength,
) {
    if from == to {
        return;
    }
    *grouped
        .entry((
            from,
            to,
            edge_type.to_string(),
            evidence.to_string(),
            strength,
        ))
        .or_insert(0) += 1;
}

fn directory_edge_endpoint(scope: &str, path: &str) -> String {
    let scope = repo::normalize_rel_path(scope);
    let path = repo::normalize_rel_path(path);
    if scope == "." {
        return top_level_endpoint(&path);
    }
    if let Some(rest) = path.strip_prefix(&format!("{}/", scope.trim_end_matches('/'))) {
        if let Some((dir, _)) = rest.split_once('/') {
            return format!("{}/{dir}/", scope.trim_end_matches('/'));
        }
        return format!("{}/", scope.trim_end_matches('/'));
    }
    top_level_endpoint(&path)
}

fn top_level_endpoint(path: &str) -> String {
    let mut parts = path.split('/');
    if let (Some(first), Some(second)) = (parts.next(), parts.next())
        && matches!(
            first,
            "apps" | "packages" | "services" | "domains" | "crates" | "modules"
        )
    {
        return format!("{first}/{second}/");
    }
    if let Some((dir, _)) = path.split_once('/') {
        format!("{dir}/")
    } else {
        path.to_string()
    }
}

fn path_under_scope(path: &str, scope: &str) -> bool {
    let path = repo::normalize_rel_path(path);
    let scope = repo::normalize_rel_path(scope);
    scope == "." || path == scope || path.starts_with(&format!("{}/", scope.trim_end_matches('/')))
}

fn cone_anchor(
    project: &Project,
    rel: &str,
    include_hidden: bool,
    limit: usize,
) -> (FileSummary, Vec<String>, Vec<String>, Vec<HiddenGroup>) {
    if let Some(info) = project.files.get(rel) {
        let summary = file_summary(project, info, include_hidden, limit);
        let mut hidden = Vec::new();
        if !include_hidden && info.symbols.len() > summary.symbols.len() {
            hidden.push(HiddenGroup {
                reason: "symbols hidden by limit".to_string(),
                count: info.symbols.len() - summary.symbols.len(),
                expand: format!("codemap cone {} --include-hidden", shell_quote(rel)),
            });
        }
        return (summary, vec![info.rel.clone()], Vec::new(), hidden);
    }
    if directory_has_files(project, rel) {
        let mut files = files_under_directory(project, rel)
            .into_iter()
            .filter(|file| {
                !file.has_role("generated") && (include_hidden || !is_generic_noise(file))
            })
            .map(|file| file.rel.clone())
            .collect::<Vec<_>>();
        files.sort();
        let count = files.len();
        if !include_hidden {
            files.truncate(limit);
        }
        let mut hidden = Vec::new();
        if count > files.len() {
            hidden.push(HiddenGroup {
                reason: "directory anchor files hidden by limit".to_string(),
                count: count - files.len(),
                expand: format!("codemap cone {} --include-hidden", shell_quote(rel)),
            });
        }
        return (
            FileSummary {
                path: rel.to_string(),
                kind: "directory".to_string(),
                package: package_name_for_file(project, rel),
                language: "mixed".to_string(),
                lines: 0,
                roles: Vec::new(),
                symbols: Vec::new(),
                exports: Vec::new(),
                imports: Vec::new(),
                imported_by_count: 0,
            },
            files,
            vec!["directory anchor summarizes indexed files under this path".to_string()],
            hidden,
        );
    }
    (
        FileSummary {
            path: rel.to_string(),
            kind: "missing".to_string(),
            package: package_name_for_file(project, rel),
            language: "unknown".to_string(),
            lines: 0,
            roles: Vec::new(),
            symbols: Vec::new(),
            exports: Vec::new(),
            imports: Vec::new(),
            imported_by_count: 0,
        },
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
}

fn cone_depths(project: &Project, seeds: &[String], max_depth: usize) -> BTreeMap<String, usize> {
    let mut depths = BTreeMap::new();
    let mut queue = VecDeque::new();
    for seed in seeds {
        if project.files.contains_key(seed) && depths.insert(seed.clone(), 0).is_none() {
            queue.push_back(seed.clone());
        }
    }
    while let Some(rel) = queue.pop_front() {
        let depth = depths.get(&rel).copied().unwrap_or(0);
        if depth >= max_depth {
            continue;
        }
        for neighbor in structural_neighbors(project, &rel) {
            if depths.contains_key(&neighbor) {
                continue;
            }
            depths.insert(neighbor.clone(), depth + 1);
            queue.push_back(neighbor);
        }
    }
    depths
}

fn structural_neighbors(project: &Project, rel: &str) -> Vec<String> {
    let mut neighbors = Vec::new();
    if let Some(file) = project.files.get(rel) {
        neighbors.extend(file.resolved_imports.iter().cloned());
    }
    if let Some(importers) = project.reverse_imports.get(rel) {
        neighbors.extend(importers.iter().cloned());
    }
    neighbors.extend(
        same_package_symbol_reference_consumers(project, rel)
            .into_iter()
            .map(|edge| edge.from),
    );
    unique(neighbors)
        .into_iter()
        .filter(|neighbor| project.files.contains_key(neighbor))
        .collect()
}

fn cone_outgoing_edges(
    project: &Project,
    depths: &BTreeMap<String, usize>,
    max_depth: usize,
) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    for (rel, depth) in depths {
        if *depth >= max_depth {
            continue;
        }
        let Some(file) = project.files.get(rel) else {
            continue;
        };
        for target in &file.resolved_imports {
            if project.files.contains_key(target) {
                edges.push(StructuralEdge {
                    from: file.rel.clone(),
                    to: target.clone(),
                    edge_type: "imports".to_string(),
                    evidence: "resolved_import".to_string(),
                    strength: EvidenceStrength::High,
                });
            }
        }
    }
    edges
}

fn cone_incoming_edges(project: &Project, seeds: &[String]) -> Vec<StructuralEdge> {
    let seed_set = seeds.iter().cloned().collect::<BTreeSet<_>>();
    let mut edges = Vec::new();
    for seed in seeds {
        if let Some(importers) = project.reverse_imports.get(seed) {
            for importer in importers {
                if project
                    .files
                    .get(importer)
                    .map(|file| file.has_role("test"))
                    .unwrap_or(false)
                {
                    continue;
                }
                if !seed_set.contains(importer) {
                    edges.push(StructuralEdge {
                        from: importer.clone(),
                        to: seed.clone(),
                        edge_type: "imported_by".to_string(),
                        evidence: "reverse_import".to_string(),
                        strength: EvidenceStrength::High,
                    });
                }
            }
        }
        for edge in same_package_symbol_reference_consumers(project, seed) {
            if !seed_set.contains(&edge.from) {
                edges.push(edge);
            }
        }
    }
    edges
}

fn cone_proof_edges(project: &Project, seeds: &[String]) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    for seed in seeds {
        for (test, evidence, strength) in strict_test_edges_for_file(project, seed, 4) {
            edges.push(StructuralEdge {
                from: test,
                to: seed.clone(),
                edge_type: "tests".to_string(),
                evidence,
                strength,
            });
        }
    }
    edges
}

fn cone_proof_edges_with_direct_consumers(
    project: &Project,
    seeds: &[String],
) -> Vec<StructuralEdge> {
    let mut edges = cone_proof_edges(project, seeds);
    for seed in seeds {
        if !edges.iter().any(|edge| edge.to == *seed) {
            edges.extend(proof_edges_via_direct_dependencies(project, seed, 4));
        }
        for consumer in direct_consumer_edges(project, seed).into_iter().take(4) {
            for (test, evidence, strength) in strict_test_edges_for_file(project, &consumer.from, 4)
            {
                let Some(test_file) = project.files.get(&test) else {
                    continue;
                };
                if !test_mentions_anchor(project, seed, test_file) {
                    continue;
                }
                edges.push(StructuralEdge {
                    from: test,
                    to: seed.clone(),
                    edge_type: "tests".to_string(),
                    evidence: format!("{evidence}_via_direct_consumer"),
                    strength,
                });
            }
        }
    }
    dedupe_proof_edges_by_endpoint(edges)
}

fn proof_edges_via_direct_dependencies(
    project: &Project,
    seed: &str,
    limit: usize,
) -> Vec<StructuralEdge> {
    if limit == 0 {
        return Vec::new();
    }
    let Some(anchor) = project.files.get(seed) else {
        return Vec::new();
    };
    if !anchor_can_use_dependency_proof(anchor) {
        return Vec::new();
    }
    let mut edges = Vec::new();
    for dependency in direct_dependency_edges(project, seed)
        .into_iter()
        .take(limit)
    {
        let Some(dep_file) = project.files.get(&dependency.to) else {
            continue;
        };
        if !dependency_can_prove_anchor(project, anchor, dep_file) {
            continue;
        }
        for (test, evidence, strength) in strict_test_edges_for_file(project, &dependency.to, limit)
            .into_iter()
            .filter(|(_, evidence, _)| dependency_proof_can_transfer(evidence))
        {
            edges.push(StructuralEdge {
                from: test,
                to: seed.to_string(),
                edge_type: "tests".to_string(),
                evidence: format!("{evidence}_via_direct_dependency"),
                strength,
            });
        }
    }
    edges
}

fn dependency_proof_can_transfer(evidence: &str) -> bool {
    evidence == "e2e_surface_phrase"
}

fn anchor_can_use_dependency_proof(anchor: &FileInfo) -> bool {
    anchor.has_role("renderer_ui")
        || matches!(anchor.ext.as_str(), "tsx" | "jsx" | "vue" | "svelte")
}

fn dependency_can_prove_anchor(
    project: &Project,
    anchor: &FileInfo,
    dependency: &FileInfo,
) -> bool {
    if dependency.has_role("test") || dependency.has_role("test_support") {
        return false;
    }
    if package_for_rel(project, &anchor.rel).map(|package| package.path.clone())
        != package_for_rel(project, &dependency.rel).map(|package| package.path.clone())
    {
        return false;
    }
    if Path::new(&anchor.rel).parent() != Path::new(&dependency.rel).parent() {
        return false;
    }
    if !anchor_renders_dependency(anchor, dependency) {
        return false;
    }
    dependency.has_role("renderer_ui")
        || !dependency.surface_phrases.is_empty()
        || dependency
            .symbols
            .iter()
            .any(|symbol| symbol.kind == "component")
}

fn anchor_renders_dependency(anchor: &FileInfo, dependency: &FileInfo) -> bool {
    if anchor.jsx_tags.is_empty() {
        return false;
    }
    let Some(bindings) = anchor.resolved_import_bindings.get(&dependency.rel) else {
        return false;
    };
    let exported_components = dependency
        .symbols
        .iter()
        .filter(|symbol| symbol.exported && symbol.kind == "component")
        .map(|symbol| &symbol.name)
        .collect::<BTreeSet<_>>();
    bindings.iter().any(|(local, imported)| {
        anchor.jsx_tags.contains(local)
            && !anchor_declares_symbol(anchor, local)
            && exported_components.contains(imported)
    })
}

fn anchor_declares_symbol(anchor: &FileInfo, name: &str) -> bool {
    anchor.symbols.iter().any(|symbol| symbol.name == name) || anchor.local_bindings.contains(name)
}

fn test_mentions_anchor(project: &Project, rel: &str, test: &FileInfo) -> bool {
    let Some(anchor) = project.files.get(rel) else {
        return false;
    };
    if anchor_symbol_reference_names(anchor)
        .iter()
        .any(|name| test.references.contains(name))
    {
        return true;
    }
    let anchor_terms = anchor_terms(project, rel);
    let anchor_core_terms = anchor_core_terms(project, rel);
    if structural_test_surface_match(project, rel, &anchor_terms, &anchor_core_terms, test)
        .is_some()
    {
        return true;
    }
    if anchor_core_terms.is_empty() {
        return false;
    }
    let mut reference_terms = BTreeSet::new();
    for reference in &test.references {
        reference_terms.extend(semantic_name_terms(reference));
    }
    anchor_core_terms.intersection(&reference_terms).count() >= 1
}

fn dedupe_proof_edges_by_endpoint(edges: Vec<StructuralEdge>) -> Vec<StructuralEdge> {
    let mut seen = BTreeMap::new();
    let mut out: Vec<StructuralEdge> = Vec::new();
    for edge in edges {
        let key = (edge.from.clone(), edge.to.clone(), edge.edge_type.clone());
        if let Some(index) = seen.get(&key).copied() {
            if proof_edge_precedence(&edge) > proof_edge_precedence(&out[index]) {
                out[index] = edge;
            }
        } else {
            seen.insert(key, out.len());
            out.push(edge);
        }
    }
    out
}

fn proof_edge_precedence(edge: &StructuralEdge) -> (EvidenceStrength, usize) {
    (edge.strength, proof_evidence_precedence(&edge.evidence))
}

fn cone_contract_edges(project: &Project, outgoing: &[StructuralEdge]) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    for edge in outgoing {
        let Some(target) = project.files.get(&edge.to) else {
            continue;
        };
        if let Some(evidence) = contract_evidence(target) {
            edges.push(StructuralEdge {
                from: edge.from.clone(),
                to: edge.to.clone(),
                edge_type: "contract".to_string(),
                evidence,
                strength: EvidenceStrength::High,
            });
        }
    }
    edges
}

fn contract_evidence(file: &FileInfo) -> Option<String> {
    for role in [
        "schema_contract",
        "public_boundary",
        "semantic_anchor",
        "build_ci",
    ] {
        if file.has_role(role) {
            return Some(format!("role:{role}"));
        }
    }
    (file.language == "config").then(|| "language:config".to_string())
}

fn cone_boundary_edges(
    project: &Project,
    rel: &str,
    depths: &BTreeMap<String, usize>,
) -> Vec<StructuralEdge> {
    let node_set = depths.keys().cloned().collect::<BTreeSet<_>>();
    let directory_prefix = directory_has_files(project, rel).then(|| {
        if rel == "." {
            String::new()
        } else {
            format!("{}/", rel.trim_end_matches('/'))
        }
    });
    boundary_findings(project, None)
        .into_iter()
        .filter(|finding| {
            node_set.contains(&finding.from)
                || node_set.contains(&finding.to)
                || directory_prefix
                    .as_ref()
                    .map(|prefix| {
                        if prefix.is_empty() {
                            true
                        } else {
                            finding.from.starts_with(prefix) || finding.to.starts_with(prefix)
                        }
                    })
                    .unwrap_or(false)
        })
        .map(|finding| StructuralEdge {
            from: finding.from,
            to: finding.to,
            edge_type: "boundary".to_string(),
            evidence: finding.provenance,
            strength: if finding.strength == "hard" {
                EvidenceStrength::Hard
            } else {
                EvidenceStrength::Medium
            },
        })
        .collect()
}

fn sort_edges(edges: &mut Vec<StructuralEdge>) {
    edges.sort_by(|a, b| {
        a.from
            .cmp(&b.from)
            .then_with(|| a.edge_type.cmp(&b.edge_type))
            .then_with(|| a.to.cmp(&b.to))
            .then_with(|| a.evidence.cmp(&b.evidence))
    });
    edges.dedup_by(|a, b| {
        a.from == b.from && a.to == b.to && a.edge_type == b.edge_type && a.evidence == b.evidence
    });
}

fn balanced_edge_prefix_by_source(edges: &[StructuralEdge], limit: usize) -> Vec<StructuralEdge> {
    if edges.len() <= limit {
        return edges.to_vec();
    }

    let mut buckets: BTreeMap<String, VecDeque<StructuralEdge>> = BTreeMap::new();
    for edge in edges {
        buckets
            .entry(edge.from.clone())
            .or_default()
            .push_back(edge.clone());
    }

    let mut balanced = Vec::with_capacity(limit);
    while balanced.len() < limit && !buckets.is_empty() {
        let keys = buckets.keys().cloned().collect::<Vec<_>>();
        let mut progressed = false;

        for key in keys {
            if balanced.len() == limit {
                break;
            }

            let mut empty = false;
            if let Some(bucket) = buckets.get_mut(&key) {
                if let Some(edge) = bucket.pop_front() {
                    balanced.push(edge);
                    progressed = true;
                }
                empty = bucket.is_empty();
            }
            if empty {
                buckets.remove(&key);
            }
        }

        if !progressed {
            break;
        }
    }

    balanced
}

fn limit_edge_section(
    edges: &mut Vec<StructuralEdge>,
    hidden: &mut Vec<HiddenGroup>,
    include_hidden: bool,
    limit: usize,
    reason: &str,
    expand: &str,
) {
    if include_hidden {
        return;
    }
    let count = edges.len();
    edges.truncate(limit);
    if count > edges.len() {
        hidden.push(HiddenGroup {
            reason: reason.to_string(),
            count: count - edges.len(),
            expand: expand.to_string(),
        });
    }
}

fn directory_has_files(project: &Project, rel: &str) -> bool {
    if rel == "." {
        return !project.files.is_empty();
    }
    let prefix = format!("{}/", rel.trim_end_matches('/'));
    project.files.keys().any(|file| file.starts_with(&prefix))
}

fn parent_anchor_for_missing(rel: &str) -> String {
    Path::new(rel)
        .parent()
        .map(|parent| repo::normalize_rel_path(&parent.to_string_lossy()))
        .filter(|parent| !parent.is_empty())
        .unwrap_or_else(|| ".".to_string())
}

fn files_under_directory<'a>(project: &'a Project, rel: &str) -> Vec<&'a FileInfo> {
    let prefix = (rel != ".").then(|| format!("{}/", rel.trim_end_matches('/')));
    project
        .files
        .values()
        .filter(|file| {
            prefix
                .as_ref()
                .map(|prefix| file.rel.starts_with(prefix))
                .unwrap_or(true)
        })
        .collect()
}

fn direct_files_under_directory<'a>(project: &'a Project, rel: &str) -> Vec<&'a FileInfo> {
    project
        .files
        .values()
        .filter(|file| direct_child_name(rel, &file.rel).is_some_and(|name| !name.ends_with('/')))
        .collect()
}

fn immediate_child_dirs(project: &Project, rel: &str) -> Vec<String> {
    let mut dirs = BTreeSet::new();
    for file in project.files.values() {
        if let Some(name) = direct_child_name(rel, &file.rel)
            && let Some(dir) = name.strip_suffix('/')
        {
            dirs.insert(if rel == "." {
                format!("{dir}/")
            } else {
                format!("{}/{dir}/", rel.trim_end_matches('/'))
            });
        }
    }
    dirs.into_iter().collect()
}

fn direct_child_name(scope: &str, path: &str) -> Option<String> {
    let scope = repo::normalize_rel_path(scope);
    let path = repo::normalize_rel_path(path);
    let rest = if scope == "." {
        path.as_str()
    } else {
        path.strip_prefix(&format!("{}/", scope.trim_end_matches('/')))?
    };
    if rest.is_empty() {
        return None;
    }
    if let Some((dir, _)) = rest.split_once('/') {
        return Some(format!("{dir}/"));
    }
    Some(rest.to_string())
}

fn directory_role_surface(project: &Project, dir: &str) -> Option<String> {
    let prefix = dir.trim_end_matches('/');
    let files = files_under_directory(project, prefix);
    if files.is_empty() {
        return None;
    }
    for role in [
        "e2e_test",
        "test_support",
        "fixture",
        "schema_contract",
        "build_ci",
        "docs",
        "test",
        "map_engine",
        "repo_discovery",
        "cache",
    ] {
        if files.iter().any(|file| file.has_role(role)) {
            return Some(role.to_string());
        }
    }
    None
}

fn file_summary(
    project: &Project,
    info: &FileInfo,
    include_hidden: bool,
    limit: usize,
) -> FileSummary {
    let mut symbols = info.symbols.clone();
    if !include_hidden {
        symbols.truncate(limit);
    }
    FileSummary {
        path: info.rel.clone(),
        kind: file_kind_for_ls(info),
        package: package_name_for_file(project, &info.rel),
        language: info.language.clone(),
        lines: info.line_count,
        roles: structural_roles_for_ls(info),
        symbols,
        exports: info.exports.iter().cloned().collect(),
        imports: info.imports.iter().cloned().collect(),
        imported_by_count: project
            .reverse_imports
            .get(&info.rel)
            .map(|importers| importers.len())
            .unwrap_or(0),
    }
}

fn structural_roles_for_ls(info: &FileInfo) -> Vec<String> {
    info.roles.iter().cloned().collect()
}

fn package_name_for_file(project: &Project, rel: &str) -> Option<String> {
    project
        .packages
        .iter()
        .filter(|package| {
            rel == package.path
                || rel == package.manifest
                || package.path == "."
                || rel.starts_with(&format!("{}/", package.path.trim_end_matches('/')))
        })
        .max_by_key(|package| {
            if package.path == "." {
                0
            } else {
                package.path.len()
            }
        })
        .map(|package| package.name.clone())
}

fn file_kind_for_ls(info: &FileInfo) -> String {
    for role in [
        "e2e_test",
        "test_support",
        "test",
        "schema_contract",
        "public_boundary",
        "runtime_state",
        "adapter",
        "parser",
        "renderer_ui",
        "persistence",
        "map_engine",
        "repo_discovery",
        "cache",
        "build_ci",
        "semantic_anchor",
        "agent_bootstrap",
        "fixture",
        "example",
        "generated",
    ] {
        if info.has_role(role) {
            return role.to_string();
        }
    }
    if repo::is_source_ext(&info.ext) {
        "source".to_string()
    } else if info.language == "config" {
        "config".to_string()
    } else if info.language == "markdown" {
        "docs".to_string()
    } else {
        "file".to_string()
    }
}

fn is_generic_noise(info: &FileInfo) -> bool {
    repo::is_source_ext(&info.ext)
        && info.roles.is_empty()
        && info.imports.is_empty()
        && info.exports.is_empty()
        && info.symbols.is_empty()
}

fn strict_test_edges_for_file(
    project: &Project,
    rel: &str,
    limit: usize,
) -> Vec<(String, String, EvidenceStrength)> {
    if limit == 0 {
        return Vec::new();
    }
    let source_domain = scoped_domain_path_for_rel(project, rel, domain_by_rel(project, rel));
    let anchor_terms = anchor_terms(project, rel);
    let anchor_core_terms = anchor_core_terms(project, rel);
    let lower_stem = source_stem(rel);
    let allow_name_match = meaningful_stem(&lower_stem);
    let mut scored = Vec::new();
    for file in project.files.values() {
        if !file.has_role("test") || file.has_role("test_support") {
            continue;
        }
        let test_domain =
            scoped_domain_path_for_rel(project, &file.rel, domain_by_rel(project, rel));
        if source_domain.is_some() && source_domain != test_domain {
            continue;
        }
        if !swift_test_can_prove_anchor(project, rel, file) {
            continue;
        }
        if file.resolved_imports.contains(rel) {
            scored.push((
                80usize,
                file.rel.clone(),
                "test_import".to_string(),
                EvidenceStrength::High,
            ));
            continue;
        }
        if e2e_test_visits_route(rel, file) {
            scored.push((
                79usize,
                file.rel.clone(),
                "e2e_route".to_string(),
                EvidenceStrength::High,
            ));
            continue;
        }
        if test_imports_support_consuming_anchor(project, rel, file) {
            scored.push((
                76usize,
                file.rel.clone(),
                "test_support_import".to_string(),
                EvidenceStrength::High,
            ));
            continue;
        }
        if test_references_anchor_symbol(project, rel, file) {
            scored.push((
                74usize,
                file.rel.clone(),
                "test_symbol_reference".to_string(),
                EvidenceStrength::High,
            ));
            continue;
        }
        if allow_name_match && test_name_matches_source_stem(&file.rel, &lower_stem) {
            scored.push((
                70usize,
                file.rel.clone(),
                "test_name".to_string(),
                EvidenceStrength::High,
            ));
            continue;
        }
        if let Some((score, evidence, strength)) =
            structural_test_surface_match(project, rel, &anchor_terms, &anchor_core_terms, file)
        {
            scored.push((score, file.rel.clone(), evidence, strength));
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    if scored
        .iter()
        .any(|(_, _, _, strength)| *strength == EvidenceStrength::High)
    {
        scored.retain(|(_, _, evidence, strength)| {
            *strength == EvidenceStrength::High || evidence == "e2e_surface_phrase"
        });
    }
    scored
        .into_iter()
        .map(|(_, rel, evidence, strength)| (rel, evidence, strength))
        .take(limit)
        .collect()
}

fn structural_test_surface_match(
    project: &Project,
    rel: &str,
    anchor_terms: &BTreeSet<String>,
    anchor_core_terms: &BTreeSet<String>,
    test: &FileInfo,
) -> Option<(usize, String, EvidenceStrength)> {
    if anchor_terms.is_empty() {
        return None;
    }
    let phrase_shared = shared_surface_phrases(project, rel, test);
    if test.has_role("e2e_test") {
        if phrase_shared.is_empty() {
            return None;
        }
        return Some((
            78 + phrase_shared.len().min(8) * 4,
            "e2e_surface_phrase".to_string(),
            EvidenceStrength::Medium,
        ));
    }
    if !phrase_shared.is_empty() {
        return Some((
            72 + phrase_shared.len().min(8) * 4,
            "test_surface_phrase".to_string(),
            EvidenceStrength::Medium,
        ));
    }
    let test_terms = test_surface_terms(test);
    let shared = anchor_terms
        .intersection(&test_terms)
        .cloned()
        .collect::<BTreeSet<_>>();
    let shared_count = shared.len();
    if shared_count < 2 {
        return None;
    }
    let core_shared_count = anchor_core_terms.intersection(&test_terms).count();
    if core_shared_count == 0 {
        return None;
    }
    let same_parent_signal = same_parent_or_test_scope(rel, &test.rel);
    let test_path_terms = semantic_path_terms(&test.rel);
    let core_path_shared_count = anchor_core_terms.intersection(&test_path_terms).count();
    if same_parent_signal
        || (core_path_shared_count >= 2 && (core_shared_count >= 2 || shared_count >= 3))
    {
        return Some((
            50 + shared_count.min(8) + core_shared_count.min(4) * 10,
            "test_surface_tokens".to_string(),
            EvidenceStrength::Medium,
        ));
    }
    let source_package = package_for_rel(project, rel).map(|package| package.path.clone());
    let test_package = package_for_rel(project, &test.rel).map(|package| package.path.clone());
    if source_package.is_some()
        && source_package == test_package
        && core_path_shared_count >= 2
        && core_shared_count >= 2
    {
        return Some((
            42 + shared_count.min(8) + core_shared_count.min(4) * 10,
            "test_surface_tokens".to_string(),
            EvidenceStrength::Medium,
        ));
    }
    None
}

fn e2e_test_visits_route(rel: &str, test: &FileInfo) -> bool {
    if !test.has_role("e2e_test") {
        return false;
    }
    let Some(route) = next_app_route_path(rel) else {
        return false;
    };
    test.visited_route_paths.contains(&route)
}

fn next_app_route_path(rel: &str) -> Option<String> {
    let rest = rel.strip_prefix("app/")?;
    let route_dir = ["page.tsx", "page.ts", "page.jsx", "page.js"]
        .iter()
        .find_map(|suffix| {
            if rest == *suffix {
                Some("")
            } else {
                rest.strip_suffix(&format!("/{suffix}"))
            }
        })?;
    let mut segments = Vec::new();
    for segment in route_dir.split('/').filter(|segment| !segment.is_empty()) {
        if segment.starts_with('(') && segment.ends_with(')') {
            continue;
        }
        if segment.starts_with('@') || segment.contains('[') {
            return None;
        }
        segments.push(segment);
    }
    if segments.is_empty() {
        Some("/".to_string())
    } else {
        Some(format!("/{}", segments.join("/")))
    }
}

fn test_imports_support_consuming_anchor(project: &Project, rel: &str, test: &FileInfo) -> bool {
    let mut seen = BTreeSet::new();
    let mut frontier = test
        .resolved_imports
        .iter()
        .filter_map(|import| project.files.get(import))
        .filter(|file| file.has_role("test_support"))
        .map(|file| file.rel.clone())
        .collect::<Vec<_>>();
    for _ in 0..2 {
        if frontier.is_empty() {
            return false;
        }
        let mut next = Vec::new();
        for support_rel in frontier {
            if !seen.insert(support_rel.clone()) {
                continue;
            }
            let Some(support) = project.files.get(&support_rel) else {
                continue;
            };
            if support.resolved_imports.contains(rel) {
                return true;
            }
            next.extend(
                support
                    .resolved_imports
                    .iter()
                    .filter_map(|import| project.files.get(import))
                    .filter(|file| file.has_role("test_support"))
                    .map(|file| file.rel.clone()),
            );
        }
        frontier = next;
    }
    false
}

fn swift_test_can_prove_anchor(project: &Project, rel: &str, test: &FileInfo) -> bool {
    let Some(anchor) = project.files.get(rel) else {
        return true;
    };
    if anchor.ext != "swift" || test.ext != "swift" {
        return true;
    }
    let Some((root, target)) = swift_source_scope(&anchor.rel) else {
        return false;
    };
    swift_test_package_root(&test.rel)
        .map(|test_root| test_root == root)
        .unwrap_or(false)
        && test.imports.contains(&target)
}

fn test_references_anchor_symbol(project: &Project, rel: &str, test: &FileInfo) -> bool {
    let Some(anchor) = project.files.get(rel) else {
        return false;
    };
    if anchor_symbol_reference_names(anchor).is_empty() {
        return false;
    }
    let source_domain = scoped_domain_path_for_rel(project, rel, domain_by_rel(project, rel));
    let test_domain = scoped_domain_path_for_rel(project, &test.rel, domain_by_rel(project, rel));
    if source_domain.is_some() && source_domain != test_domain {
        return false;
    }
    let source_package = package_for_rel(project, rel).map(|package| package.path.clone());
    let test_package = package_for_rel(project, &test.rel).map(|package| package.path.clone());
    if source_package.is_some() && source_package != test_package {
        return false;
    }
    if !same_symbol_reference_scope(anchor, test) {
        return false;
    }
    anchor_symbol_reference_names(anchor)
        .iter()
        .any(|name| test.references.contains(name))
}

fn anchor_symbol_reference_names(anchor: &FileInfo) -> BTreeSet<String> {
    anchor
        .symbols
        .iter()
        .filter(|symbol| symbol.kind != "method")
        .filter(|symbol| symbol.exported || structural_anchor_symbol_kind(&symbol.kind))
        .map(|symbol| symbol.name.clone())
        .filter(|name| meaningful_symbol_reference_name(name))
        .collect()
}

fn meaningful_symbol_reference_name(name: &str) -> bool {
    if name == "default" || name.len() < 4 {
        return false;
    }
    let terms = semantic_name_terms(name);
    !terms.is_empty()
}

fn shared_surface_phrases(project: &Project, rel: &str, test: &FileInfo) -> BTreeSet<String> {
    project
        .files
        .get(rel)
        .map(|file| {
            let mut shared = BTreeSet::new();
            for source_phrase in &file.surface_phrases {
                if !meaningful_surface_phrase(source_phrase) {
                    continue;
                }
                for test_phrase in &test.surface_phrases {
                    if !meaningful_surface_phrase(test_phrase) {
                        continue;
                    }
                    if surface_phrases_match(source_phrase, test_phrase) {
                        shared.insert(source_phrase.clone());
                    }
                }
            }
            shared
        })
        .unwrap_or_default()
}

fn surface_phrases_match(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }
    let (shorter, longer) = if left.len() <= right.len() {
        (left, right)
    } else {
        (right, left)
    };
    let shorter_terms = surface_phrase_terms(shorter);
    shorter_terms.len() >= 3 && phrase_contains_with_boundaries(longer, shorter)
}

fn phrase_contains_with_boundaries(longer: &str, shorter: &str) -> bool {
    longer.match_indices(shorter).any(|(start, _)| {
        let before = longer[..start].chars().next_back();
        let end = start + shorter.len();
        let after = longer[end..].chars().next();
        before.map(phrase_boundary_char).unwrap_or(true)
            && after.map(phrase_boundary_char).unwrap_or(true)
    })
}

fn phrase_boundary_char(ch: char) -> bool {
    ch == '-'
}

fn source_stem(rel: &str) -> String {
    Path::new(rel)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .replace(".test", "")
        .replace(".spec", "")
        .to_ascii_lowercase()
}

fn test_name_matches_source_stem(test_rel: &str, source_stem: &str) -> bool {
    test_stem(test_rel) == source_stem
}

fn test_stem(test_rel: &str) -> String {
    let mut stem = Path::new(test_rel)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    for suffix in [".test", ".spec", "_test"] {
        if let Some(stripped) = stem.strip_suffix(suffix) {
            stem = stripped.to_string();
        }
    }
    stem
}

fn meaningful_stem(stem: &str) -> bool {
    !stem.is_empty() && !matches!(stem, "index" | "mod" | "main" | "lib" | "types")
}

fn anchor_terms(project: &Project, rel: &str) -> BTreeSet<String> {
    let mut terms = semantic_path_terms(rel);
    if let Some(file) = project.files.get(rel) {
        for symbol in &file.symbols {
            if symbol.exported || structural_anchor_symbol_kind(&symbol.kind) {
                terms.extend(semantic_name_terms(&symbol.name));
            }
        }
        for export in &file.exports {
            terms.extend(semantic_name_terms(export));
        }
        terms.extend(file.surface_tokens.iter().cloned());
    }
    terms
}

fn anchor_core_terms(project: &Project, rel: &str) -> BTreeSet<String> {
    let mut terms = semantic_name_terms(&source_stem(rel));
    if let Some(file) = project.files.get(rel) {
        for symbol in &file.symbols {
            if symbol.exported {
                terms.extend(semantic_name_terms(&symbol.name));
            }
        }
        for export in &file.exports {
            terms.extend(semantic_name_terms(export));
        }
    }
    terms
}

fn structural_anchor_symbol_kind(kind: &str) -> bool {
    matches!(
        kind,
        "component"
            | "function"
            | "class"
            | "interface"
            | "type"
            | "struct"
            | "enum"
            | "trait"
            | "method"
    )
}

fn test_surface_terms(file: &FileInfo) -> BTreeSet<String> {
    let mut terms = semantic_path_terms(&file.rel);
    terms.extend(file.surface_tokens.iter().cloned());
    terms
}

fn semantic_path_terms(path: &str) -> BTreeSet<String> {
    let normalized = repo::normalize_rel_path(path);
    let without_ext = Path::new(&normalized)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(normalized.as_str());
    repo::tokenize(&normalized)
        .into_iter()
        .chain(semantic_name_terms(without_ext))
        .filter(|term| meaningful_surface_term(term))
        .collect()
}

fn semantic_name_terms(name: &str) -> BTreeSet<String> {
    let mut expanded = String::new();
    let mut previous_lower_or_digit = false;
    for ch in name.chars() {
        if ch == '-' || ch == '_' || ch == '.' || ch == '/' {
            expanded.push(' ');
            previous_lower_or_digit = false;
            continue;
        }
        if ch.is_ascii_uppercase() && previous_lower_or_digit {
            expanded.push(' ');
        }
        previous_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        expanded.push(ch);
    }
    repo::tokenize(&expanded)
        .into_iter()
        .filter(|term| meaningful_surface_term(term))
        .collect()
}

fn meaningful_surface_term(term: &str) -> bool {
    term.len() >= 3
        && !matches!(
            term,
            "app"
                | "apps"
                | "src"
                | "lib"
                | "libs"
                | "test"
                | "tests"
                | "spec"
                | "unit"
                | "e2e"
                | "tsx"
                | "jsx"
                | "mjs"
                | "cjs"
                | "typescript"
                | "javascript"
                | "component"
                | "components"
                | "feature"
                | "features"
                | "page"
                | "pages"
                | "hook"
                | "hooks"
                | "util"
                | "utils"
                | "index"
                | "main"
                | "type"
                | "types"
                | "support"
                | "setup"
                | "helper"
                | "helpers"
                | "fixture"
                | "fixtures"
                | "blueprint"
        )
}

fn meaningful_surface_phrase(phrase: &str) -> bool {
    let terms = surface_phrase_terms(phrase);
    terms.len() >= 2
        && terms
            .iter()
            .any(|term| !matches!(term.as_str(), "frame" | "title" | "canvas" | "node"))
}

fn surface_phrase_terms(phrase: &str) -> BTreeSet<String> {
    surface_terms(&phrase.replace(['.', '#', '/', '-', '_', ':'], " "))
        .into_iter()
        .filter(|term| term.len() >= 3)
        .filter(|term| {
            !matches!(
                term.as_str(),
                "the"
                    | "and"
                    | "for"
                    | "with"
                    | "from"
                    | "true"
                    | "false"
                    | "null"
                    | "undefined"
                    | "data"
                    | "test"
                    | "testid"
                    | "aria"
                    | "label"
                    | "role"
                    | "root"
                    | "blueprint"
                    | "nodrag"
                    | "nopan"
            )
        })
        .collect()
}

fn surface_terms(value: &str) -> BTreeSet<String> {
    value
        .split(|ch: char| !(ch.is_alphanumeric() || ch == '_'))
        .map(str::to_lowercase)
        .filter(|term| term.len() >= 2)
        .collect()
}

fn same_parent_or_test_scope(source: &str, test: &str) -> bool {
    let source_parent = Path::new(source)
        .parent()
        .map(|path| repo::normalize_rel_path(&path.to_string_lossy()))
        .unwrap_or_else(|| ".".to_string());
    if test.starts_with(&format!("{}/", source_parent.trim_end_matches('/'))) {
        return true;
    }
    let source_stem = source_stem(source);
    meaningful_stem(&source_stem) && test.to_ascii_lowercase().contains(&source_stem)
}

fn surface_priority(kind: &str) -> usize {
    if kind == "domain" {
        return 0;
    }
    if kind.starts_with("package:") {
        return 1;
    }
    if kind == "dir" {
        return 2;
    }
    if kind == "script" {
        return 3;
    }
    if kind.starts_with("recursive:") {
        return 11;
    }
    if kind.starts_with("support_package:") {
        return 10;
    }
    match kind {
        "schema_contract" | "public_boundary" => 4,
        "runtime_state" | "persistence" | "adapter" | "parser" | "renderer_ui" | "map_engine" => 5,
        "test" | "e2e_test" | "test_support" => 6,
        "source" => 7,
        "config" | "build_ci" => 8,
        _ => 9,
    }
}

fn is_support_artifact_path(rel: &str) -> bool {
    let rel = repo::normalize_rel_path(rel);
    rel == "fixtures"
        || rel.starts_with("fixtures/")
        || rel.contains("/fixtures/")
        || rel == "examples"
        || rel.starts_with("examples/")
        || rel.contains("/examples/")
        || rel == "samples"
        || rel.starts_with("samples/")
        || rel.contains("/samples/")
}

pub fn impact_report(
    project: &Project,
    changed: Vec<String>,
    depth: usize,
    limit: usize,
) -> ImpactReport {
    let limit = limit.max(1);
    let changed = changed
        .into_iter()
        .map(|file| repo::normalize_rel_path(&file))
        .filter(|file| file != ".")
        .collect::<Vec<_>>();
    let mut hidden = Vec::new();
    let mut unknowns = Vec::new();
    let mut changed_summaries = Vec::new();
    let mut clusters = Vec::new();
    let changed_count = changed.len();
    for rel in changed.iter().take(limit) {
        if let Some(file) = project.files.get(rel) {
            changed_summaries.push(file_summary(project, file, false, 12));
            let (cluster, cluster_hidden) = impact_cluster(project, rel, depth, limit);
            hidden.extend(cluster_hidden);
            clusters.push(cluster);
        } else {
            unknowns.push(format!("changed anchor `{rel}` is not indexed"));
            changed_summaries.push(missing_file_summary(project, rel));
            clusters.push(ImpactCluster {
                id: format!("changed:{rel}"),
                risk: Risk::Medium.as_str().to_string(),
                changed: vec![rel.clone()],
                direct_consumers: Vec::new(),
                cross_boundary_consumers: Vec::new(),
                contract_risks: Vec::new(),
                proof: Vec::new(),
                reasons: vec!["changed file is not indexed".to_string()],
            });
        }
    }
    if changed_count > changed_summaries.len() {
        hidden.push(HiddenGroup {
            reason: "changed anchors hidden by limit".to_string(),
            count: changed_count - changed_summaries.len(),
            expand: "codemap impact --changed --limit <larger-number>".to_string(),
        });
    }
    ImpactReport {
        kind: "impact_report",
        schema_version: "2",
        changed: changed_summaries,
        clusters,
        hidden,
        unknowns,
        expand: impact_expand_commands(&changed),
    }
}

pub fn proof_report(
    project: &Project,
    target: Option<String>,
    changed: Vec<String>,
    depth: usize,
    limit: usize,
) -> ProofReport {
    let limit = limit.max(1);
    let target = target.map(|path| repo::normalize_rel_path(&path));
    let changed = changed
        .into_iter()
        .map(|file| repo::normalize_rel_path(&file))
        .filter(|file| file != ".")
        .collect::<Vec<_>>();
    let anchors = if let Some(target) = target.as_ref() {
        vec![target.clone()]
    } else {
        changed.clone()
    };
    let mut proofs = Vec::new();
    let mut risk = Risk::Low;
    if target.is_none() && !changed.is_empty() {
        let impact = impact_report(project, changed.clone(), depth, limit);
        for cluster in &impact.clusters {
            risk = risk.max(risk_from_str(&cluster.risk));
            proofs.extend(proof_surfaces_from_edges(
                project,
                &cluster.proof,
                "impact cluster",
            ));
        }
    } else {
        for anchor in &anchors {
            risk = risk.max(
                project
                    .files
                    .get(anchor)
                    .map(|_| risk_for_file(project, anchor).0)
                    .unwrap_or(Risk::Medium),
            );
            proofs.extend(proof_surfaces_for_anchor(project, anchor, depth, limit));
        }
    }
    proofs = unique_proof_surfaces(proofs);
    if proofs.len() > limit {
        proofs.truncate(limit);
    }
    let fallback = proof_fallback_commands(project, &anchors, &changed, &proofs);
    ProofReport {
        kind: "proof_plan",
        schema_version: "2",
        target,
        changed,
        risk: risk.as_str().to_string(),
        proofs,
        fallback,
        run_hint: "codemap proof prints only by default; use --run to execute proof commands"
            .to_string(),
    }
}

fn risk_from_str(value: &str) -> Risk {
    match value {
        "critical" => Risk::Critical,
        "high" => Risk::High,
        "medium-high" => Risk::MediumHigh,
        "medium" => Risk::Medium,
        _ => Risk::Low,
    }
}

fn proof_surfaces_from_edges(
    project: &Project,
    edges: &[StructuralEdge],
    scope: &str,
) -> Vec<ProofSurface> {
    edges
        .iter()
        .filter(|edge| edge.edge_type == "tests")
        .map(|edge| ProofSurface {
            command: proof_command_for_test(project, &edge.from),
            path: Some(edge.from.clone()),
            evidence: edge.evidence.clone(),
            strength: edge.strength,
            reason: proof_reason_for_evidence(&edge.evidence, scope),
        })
        .collect()
}

fn proof_surfaces_for_anchor(
    project: &Project,
    anchor: &str,
    depth: usize,
    limit: usize,
) -> Vec<ProofSurface> {
    let mut out = Vec::new();
    for (test, evidence, strength) in strict_test_edges_for_file(project, anchor, limit) {
        out.push(ProofSurface {
            command: proof_command_for_test(project, &test),
            path: Some(test),
            reason: proof_reason_for_evidence(&evidence, "anchor"),
            evidence,
            strength,
        });
    }
    if out.is_empty() {
        for edge in proof_edges_via_direct_dependencies(project, anchor, limit) {
            out.push(ProofSurface {
                command: proof_command_for_test(project, &edge.from),
                path: Some(edge.from),
                reason: proof_reason_for_evidence(&edge.evidence, "anchor"),
                evidence: edge.evidence,
                strength: edge.strength,
            });
        }
    }
    if depth <= 1 && !out.is_empty() {
        return out;
    }
    let mut consumers = direct_consumer_edges(project, anchor);
    sort_edges(&mut consumers);
    for consumer in consumers.into_iter().take(limit) {
        for (test, evidence, strength) in strict_test_edges_for_file(project, &consumer.from, limit)
        {
            out.push(ProofSurface {
                command: proof_command_for_test(project, &test),
                path: Some(test),
                reason: proof_reason_for_evidence(&evidence, "direct consumer"),
                evidence,
                strength,
            });
        }
    }
    if depth > 1 {
        for consumer in direct_consumer_edges(project, anchor)
            .into_iter()
            .take(limit)
        {
            for second in direct_consumer_edges(project, &consumer.from)
                .into_iter()
                .take(limit)
            {
                for (test, evidence, strength) in
                    strict_test_edges_for_file(project, &second.from, limit)
                {
                    out.push(ProofSurface {
                        command: proof_command_for_test(project, &test),
                        path: Some(test),
                        reason: proof_reason_for_evidence(&evidence, "depth-2 consumer"),
                        evidence,
                        strength,
                    });
                }
            }
        }
    }
    out
}

fn proof_reason_for_evidence(evidence: &str, scope: &str) -> String {
    if let Some(base) = evidence.strip_suffix("_via_direct_consumer") {
        return format!(
            "{} via direct consumer",
            proof_reason_for_evidence(base, scope)
        );
    }
    if let Some(base) = evidence.strip_suffix("_via_direct_dependency") {
        return format!(
            "{} via direct dependency",
            proof_reason_for_evidence(base, scope)
        );
    }
    match evidence {
        "test_import" => format!("test imports {scope}"),
        "e2e_route" => format!("e2e visits route for {scope}"),
        "test_name" => format!("test name matches {scope}"),
        "test_support_import" => format!("test imports support code that imports {scope}"),
        "test_symbol_reference" => format!("test references an anchor symbol from {scope}"),
        "test_surface_phrase" => format!("test uses same UI/test surface as {scope}"),
        "e2e_surface_phrase" => format!("e2e uses same UI/test surface as {scope}"),
        "test_surface_tokens" => format!("test path/symbols match {scope} surface"),
        _ => format!("structural proof for {scope}"),
    }
}

fn proof_command_for_test(project: &Project, test: &str) -> Option<String> {
    let Some(package) = package_for_rel(project, test) else {
        return project.files.get(test).and_then(|file| {
            (file.language == "python").then(|| format!("pytest {}", shell_quote(test)))
        });
    };
    match package.ecosystem.as_str() {
        "javascript" => javascript_test_file_command(project, package, test),
        "python" => Some(if package.path == "." {
            format!("pytest {}", shell_quote(test))
        } else {
            format!(
                "cd {} && pytest {}",
                shell_quote(&package.path),
                shell_quote(&strip_package_prefix(test, &package.path))
            )
        }),
        "swift" => Some(if package.path == "." {
            "swift test".to_string()
        } else {
            format!("cd {} && swift test", shell_quote(&package.path))
        }),
        "rust" => package_minimal_command(
            project,
            package,
            &[domain_for_path(project, test)],
            find_script(project, &["test"]).as_deref(),
        ),
        "go" => package_minimal_command(
            project,
            package,
            &[domain_for_path(project, test)],
            find_script(project, &["test"]).as_deref(),
        ),
        _ => package_minimal_command(
            project,
            package,
            &[domain_for_path(project, test)],
            find_script(project, &["test"]).as_deref(),
        ),
    }
}

fn javascript_test_file_command(
    project: &Project,
    package: &crate::model::PackageInfo,
    test: &str,
) -> Option<String> {
    let runner = javascript_runner_for_package(project, package);
    let test_arg = shell_quote(&strip_package_prefix(test, &package.path));
    if project
        .files
        .get(test)
        .map(|file| file.has_role("e2e_test"))
        .unwrap_or(false)
        && let Some(command) =
            javascript_e2e_test_file_command(project, package, &runner, &test_arg)
    {
        return Some(if package.path == "." {
            command
        } else {
            format!("cd {} && {command}", shell_quote(&package.path))
        });
    }
    if !javascript_package_has_script(project, package, "test") {
        return None;
    }
    let command = javascript_package_script(project, package, "test")
        .and_then(|script| javascript_test_file_command_for_script(&runner, &script, &test_arg))
        .unwrap_or_else(|| javascript_test_file_command_for_runner(&runner, &test_arg));
    Some(if package.path == "." {
        command
    } else {
        format!("cd {} && {command}", shell_quote(&package.path))
    })
}

fn javascript_e2e_test_file_command(
    project: &Project,
    package: &crate::model::PackageInfo,
    runner: &str,
    test_arg: &str,
) -> Option<String> {
    let candidates = [
        "test:e2e",
        "e2e",
        "playwright",
        "test:playwright",
        "test:e2e:ui",
        "test:e2e:ui-rails",
    ];
    if let Some((name, _)) = javascript_package_script_by_names(project, package, &candidates) {
        return Some(javascript_package_script_invocation(
            runner, &name, test_arg,
        ));
    }
    javascript_package_script_matching(project, package, |name, command| {
        let name = name.to_ascii_lowercase();
        let command = command.to_ascii_lowercase();
        (name.contains("e2e") || name.contains("playwright")) && command.contains("playwright")
    })
    .map(|(name, _)| javascript_package_script_invocation(runner, &name, test_arg))
}

fn javascript_package_script_by_names(
    project: &Project,
    package: &crate::model::PackageInfo,
    names: &[&str],
) -> Option<(String, String)> {
    for name in names {
        if let Some(command) = javascript_package_script(project, package, name) {
            return Some(((*name).to_string(), command));
        }
    }
    None
}

fn javascript_package_script_matching<F>(
    project: &Project,
    package: &crate::model::PackageInfo,
    predicate: F,
) -> Option<(String, String)>
where
    F: Fn(&str, &str) -> bool,
{
    let text = std::fs::read_to_string(project.root.join(&package.manifest)).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&text).ok()?;
    let scripts = value
        .get("scripts")
        .and_then(|scripts| scripts.as_object())?;
    scripts.iter().find_map(|(name, value)| {
        let command = value.as_str()?.trim();
        (!command.is_empty() && predicate(name, command))
            .then(|| (name.to_string(), command.to_string()))
    })
}

fn javascript_package_script_invocation(runner: &str, script_name: &str, test_arg: &str) -> String {
    if script_name == "test" {
        return javascript_test_file_command_for_runner(runner, test_arg);
    }
    match runner {
        "npm" => format!("npm run {} -- {test_arg}", shell_quote(script_name)),
        "yarn" => format!("yarn {} {test_arg}", shell_quote(script_name)),
        "bun" => format!("bun run {} {test_arg}", shell_quote(script_name)),
        _ => format!("pnpm run {} -- {test_arg}", shell_quote(script_name)),
    }
}

fn javascript_test_file_command_for_script(
    runner: &str,
    script: &str,
    test_arg: &str,
) -> Option<String> {
    let script = script.trim();
    if script.is_empty() || !is_simple_javascript_test_script(script) {
        return None;
    }
    let known = [
        "vitest",
        "jest",
        "uvu",
        "ava",
        "mocha",
        "playwright test",
        "node --test",
        "tsx",
    ];
    if !known
        .iter()
        .any(|prefix| script_starts_with(script, prefix))
    {
        return None;
    }
    Some(javascript_exec_command(runner, script, test_arg))
}

fn is_simple_javascript_test_script(script: &str) -> bool {
    !["&&", "||", ";", "|", "\n"]
        .iter()
        .any(|marker| script.contains(marker))
}

fn script_starts_with(script: &str, prefix: &str) -> bool {
    script == prefix
        || script
            .strip_prefix(prefix)
            .map(|rest| rest.starts_with(char::is_whitespace))
            .unwrap_or(false)
}

fn javascript_exec_command(runner: &str, script: &str, test_arg: &str) -> String {
    if script_starts_with(script, "npm")
        || script_starts_with(script, "pnpm")
        || script_starts_with(script, "yarn")
        || script_starts_with(script, "bun")
    {
        return format!("{script} {test_arg}");
    }
    match runner {
        "pnpm" => format!("pnpm exec {script} {test_arg}"),
        "yarn" => format!("yarn {script} {test_arg}"),
        "bun" => format!("bunx {script} {test_arg}"),
        _ => format!("npx {script} {test_arg}"),
    }
}

fn javascript_test_file_command_for_runner(runner: &str, test_arg: &str) -> String {
    match runner {
        "npm" => format!("npm test -- {test_arg}"),
        "yarn" => format!("yarn test {test_arg}"),
        "bun" => format!("bun test {test_arg}"),
        _ => format!("pnpm test {test_arg}"),
    }
}

fn javascript_package_script(
    project: &Project,
    package: &crate::model::PackageInfo,
    script: &str,
) -> Option<String> {
    let text = std::fs::read_to_string(project.root.join(&package.manifest)).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&text).ok()?;
    value
        .get("scripts")
        .and_then(|scripts| scripts.get(script))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn strip_package_prefix(rel: &str, package_path: &str) -> String {
    let prefix = package_path.trim_end_matches('/');
    if prefix == "." {
        return rel.to_string();
    }
    rel.strip_prefix(&format!("{prefix}/"))
        .unwrap_or(rel)
        .to_string()
}

fn proof_fallback_commands(
    project: &Project,
    anchors: &[String],
    changed: &[String],
    proofs: &[ProofSurface],
) -> Vec<String> {
    if anchors.is_empty() && changed.is_empty() {
        return Vec::new();
    }
    if proofs.iter().any(|proof| proof.command.is_some()) {
        return Vec::new();
    }
    let proof_commands = proofs
        .iter()
        .filter_map(|proof| proof.command.as_ref())
        .cloned()
        .collect::<Vec<_>>();
    let all_files = if anchors.is_empty() {
        changed.to_vec()
    } else {
        anchors.to_vec()
    };
    let impacted = if changed.is_empty() {
        Vec::new()
    } else {
        let impact = impact_report(project, changed.to_vec(), 1, 30);
        impact
            .clusters
            .iter()
            .flat_map(|cluster| {
                cluster
                    .direct_consumers
                    .iter()
                    .map(|edge| edge.from.clone())
                    .chain(
                        cluster
                            .contract_risks
                            .iter()
                            .filter(|edge| edge.from != edge.to)
                            .map(|edge| edge.from.clone()),
                    )
            })
            .collect::<Vec<_>>()
    };
    let plan = verification_plan(project, &all_files, &impacted);
    unique(plan.minimal)
        .into_iter()
        .filter(|command| !proof_commands.iter().any(|existing| existing == command))
        .take(3)
        .collect()
}

fn unique_proof_surfaces(values: Vec<ProofSurface>) -> Vec<ProofSurface> {
    let mut seen = BTreeMap::new();
    let mut out = Vec::new();
    for value in values {
        let key = (
            value.command.clone().unwrap_or_default(),
            value.path.clone().unwrap_or_default(),
        );
        if let Some(index) = seen.get(&key).copied() {
            if proof_surface_precedence(&value) > proof_surface_precedence(&out[index]) {
                out[index] = value;
            }
        } else {
            seen.insert(key, out.len());
            out.push(value);
        }
    }
    out
}

fn proof_surface_precedence(value: &ProofSurface) -> (EvidenceStrength, usize) {
    (value.strength, proof_evidence_precedence(&value.evidence))
}

fn proof_evidence_precedence(evidence: &str) -> usize {
    let evidence = evidence
        .strip_suffix("_via_direct_consumer")
        .or_else(|| evidence.strip_suffix("_via_direct_dependency"))
        .unwrap_or(evidence);
    match evidence {
        "test_import" => 6,
        "e2e_route" => 5,
        "test_support_import" => 5,
        "test_symbol_reference" => 4,
        "test_name" => 3,
        "e2e_surface_phrase" => 3,
        "test_surface_phrase" => 2,
        "test_surface_tokens" => 1,
        _ => 0,
    }
}

fn impact_expand_commands(changed: &[String]) -> Vec<String> {
    if changed.is_empty() {
        return Vec::new();
    }
    let files = changed
        .iter()
        .map(|file| shell_quote(file))
        .collect::<Vec<_>>()
        .join(",");
    vec![
        format!("codemap impact --files {files} --depth 2"),
        format!("codemap proof --files {files}"),
    ]
}

fn impact_cluster(
    project: &Project,
    rel: &str,
    depth: usize,
    limit: usize,
) -> (ImpactCluster, Vec<HiddenGroup>) {
    let mut direct_consumers = direct_consumer_edges(project, rel);
    let mut cross_boundary_consumers =
        cross_boundary_consumer_edges(project, rel, &direct_consumers, depth);
    let mut contract_risks = contract_risk_edges(project, rel, &direct_consumers);
    let proof_seeds = proof_seeds_for_impact(rel, &direct_consumers);
    let mut proof = cone_proof_edges(project, &proof_seeds);
    sort_edges(&mut direct_consumers);
    sort_edges(&mut cross_boundary_consumers);
    sort_edges(&mut contract_risks);
    sort_edges(&mut proof);
    let (risk, reasons) = structural_impact_risk(
        project,
        rel,
        &direct_consumers,
        &cross_boundary_consumers,
        &contract_risks,
    );
    let mut hidden = Vec::new();
    limit_impact_edges(
        &mut direct_consumers,
        limit,
        &mut hidden,
        rel,
        depth,
        "direct consumer edges hidden by limit",
    );
    limit_impact_edges(
        &mut cross_boundary_consumers,
        limit,
        &mut hidden,
        rel,
        depth,
        "cross-boundary consumer edges hidden by limit",
    );
    limit_impact_edges(
        &mut contract_risks,
        limit,
        &mut hidden,
        rel,
        depth,
        "contract risk edges hidden by limit",
    );
    limit_impact_edges(
        &mut proof,
        limit,
        &mut hidden,
        rel,
        depth,
        "proof edges hidden by limit",
    );
    (
        ImpactCluster {
            id: format!("changed:{rel}"),
            risk: risk.as_str().to_string(),
            changed: vec![rel.to_string()],
            direct_consumers,
            cross_boundary_consumers,
            contract_risks,
            proof,
            reasons,
        },
        hidden,
    )
}

fn limit_impact_edges(
    edges: &mut Vec<StructuralEdge>,
    limit: usize,
    hidden: &mut Vec<HiddenGroup>,
    rel: &str,
    depth: usize,
    reason: &str,
) {
    if edges.len() <= limit {
        return;
    }
    hidden.push(HiddenGroup {
        reason: format!("{reason} for changed:{rel}"),
        count: edges.len() - limit,
        expand: format!(
            "codemap impact --files {} --depth {depth} --limit {}",
            shell_quote(rel),
            edges.len()
        ),
    });
    edges.truncate(limit);
}

fn direct_consumer_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let mut edges = project
        .reverse_imports
        .get(rel)
        .into_iter()
        .flat_map(|importers| importers.iter())
        .filter(|importer| {
            project
                .files
                .get(*importer)
                .map(|file| !file.has_role("test"))
                .unwrap_or(true)
        })
        .map(|importer| StructuralEdge {
            from: importer.clone(),
            to: rel.to_string(),
            edge_type: "direct_consumer".to_string(),
            evidence: "reverse_import".to_string(),
            strength: EvidenceStrength::High,
        })
        .collect::<Vec<_>>();
    edges.extend(same_package_symbol_reference_consumers(project, rel));
    sort_edges(&mut edges);
    edges
}

fn direct_dependency_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let Some(file) = project.files.get(rel) else {
        return Vec::new();
    };
    let mut edges = file
        .resolved_imports
        .iter()
        .filter(|dependency| project.files.contains_key(*dependency))
        .map(|dependency| StructuralEdge {
            from: rel.to_string(),
            to: dependency.clone(),
            edge_type: "direct_dependency".to_string(),
            evidence: "resolved_import".to_string(),
            strength: EvidenceStrength::High,
        })
        .collect::<Vec<_>>();
    sort_edges(&mut edges);
    edges
}

fn same_package_symbol_reference_consumers(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let Some(anchor) = project.files.get(rel) else {
        return Vec::new();
    };
    let names = anchor_symbol_reference_names(anchor);
    if names.is_empty() {
        return Vec::new();
    }
    project
        .files
        .values()
        .filter(|file| file.rel != rel)
        .filter(|file| !file.has_role("test") && !file.has_role("test_support"))
        .filter(|file| !file.resolved_imports.contains(rel))
        .filter(|file| same_symbol_reference_scope(anchor, file))
        .filter(|file| names.iter().any(|name| file.references.contains(name)))
        .map(|file| StructuralEdge {
            from: file.rel.clone(),
            to: rel.to_string(),
            edge_type: "direct_consumer".to_string(),
            evidence: "same_package_symbol_reference".to_string(),
            strength: EvidenceStrength::High,
        })
        .collect()
}

fn same_symbol_reference_scope(anchor: &FileInfo, file: &FileInfo) -> bool {
    if anchor.ext == "go" && file.ext == "go" {
        return Path::new(&anchor.rel).parent() == Path::new(&file.rel).parent();
    }
    if anchor.ext == "swift" && file.ext == "swift" {
        return same_swift_target_reference_scope(anchor, file);
    }
    false
}

fn same_swift_target_reference_scope(anchor: &FileInfo, file: &FileInfo) -> bool {
    let Some((anchor_root, anchor_target)) = swift_source_scope(&anchor.rel) else {
        return false;
    };
    if file.has_role("test") {
        return swift_test_package_root(&file.rel)
            .map(|test_root| test_root == anchor_root)
            .unwrap_or(false)
            && file.imports.contains(&anchor_target);
    }
    swift_source_scope(&file.rel)
        .map(|scope| scope == (anchor_root, anchor_target))
        .unwrap_or(false)
}

fn swift_source_scope(rel: &str) -> Option<(String, String)> {
    let normalized = repo::normalize_rel_path(rel);
    if let Some(rest) = normalized.strip_prefix("Sources/") {
        return rest
            .split('/')
            .next()
            .map(|target| (".".to_string(), target.to_string()));
    }
    if let Some((root, rest)) = normalized.split_once("/Sources/") {
        return rest
            .split('/')
            .next()
            .map(|target| (root.to_string(), target.to_string()));
    }
    None
}

fn swift_test_package_root(rel: &str) -> Option<String> {
    let normalized = repo::normalize_rel_path(rel);
    if normalized.starts_with("Tests/") {
        return Some(".".to_string());
    }
    normalized
        .split_once("/Tests/")
        .map(|(root, _)| root.to_string())
}

fn cross_boundary_consumer_edges(
    project: &Project,
    rel: &str,
    direct_consumers: &[StructuralEdge],
    depth: usize,
) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    let changed_domain = domain_by_rel(project, rel).map(|domain| domain.path.clone());
    let changed_package = package_for_rel(project, rel).map(|package| package.path.clone());
    for edge in direct_consumers {
        let consumer_domain = domain_by_rel(project, &edge.from).map(|domain| domain.path.clone());
        let consumer_package =
            package_for_rel(project, &edge.from).map(|package| package.path.clone());
        if changed_domain != consumer_domain || changed_package != consumer_package {
            edges.push(StructuralEdge {
                from: edge.from.clone(),
                to: rel.to_string(),
                edge_type: "cross_boundary_consumer".to_string(),
                evidence: "reverse_import_cross_boundary".to_string(),
                strength: EvidenceStrength::High,
            });
        }
    }
    let package_seeds = package_consumer_seeds_for_impact(project, rel, direct_consumers);
    for manifest in package_consumer_manifests(project, &package_seeds, depth.max(1), usize::MAX) {
        edges.push(StructuralEdge {
            from: manifest,
            to: rel.to_string(),
            edge_type: "package_consumer".to_string(),
            evidence: "package_manifest_reverse_dependency".to_string(),
            strength: EvidenceStrength::High,
        });
    }
    edges
}

fn package_consumer_seeds_for_impact(
    project: &Project,
    rel: &str,
    direct_consumers: &[StructuralEdge],
) -> Vec<String> {
    let mut seeds = vec![rel.to_string()];
    for consumer in direct_consumers {
        if let Some(file) = project.files.get(&consumer.from)
            && contract_evidence(file).is_some()
        {
            seeds.push(consumer.from.clone());
        }
    }
    unique(seeds)
}

fn contract_risk_edges(
    project: &Project,
    rel: &str,
    direct_consumers: &[StructuralEdge],
) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    if let Some(file) = project.files.get(rel) {
        if let Some(evidence) = contract_evidence(file) {
            edges.push(StructuralEdge {
                from: rel.to_string(),
                to: rel.to_string(),
                edge_type: "contract_changed".to_string(),
                evidence,
                strength: EvidenceStrength::High,
            });
        }
        for target in &file.resolved_imports {
            if let Some(target_file) = project.files.get(target)
                && let Some(evidence) = contract_evidence(target_file)
            {
                edges.push(StructuralEdge {
                    from: rel.to_string(),
                    to: target.clone(),
                    edge_type: "contract_dependency".to_string(),
                    evidence,
                    strength: EvidenceStrength::High,
                });
            }
        }
    }
    for consumer in direct_consumers {
        if let Some(consumer_file) = project.files.get(&consumer.from)
            && let Some(evidence) = contract_evidence(consumer_file)
        {
            edges.push(StructuralEdge {
                from: consumer.from.clone(),
                to: rel.to_string(),
                edge_type: "contract_consumer".to_string(),
                evidence,
                strength: EvidenceStrength::High,
            });
        }
    }
    edges
}

fn proof_seeds_for_impact(rel: &str, direct_consumers: &[StructuralEdge]) -> Vec<String> {
    let mut seeds = vec![rel.to_string()];
    seeds.extend(direct_consumers.iter().map(|edge| edge.from.clone()));
    unique(seeds)
}

fn structural_impact_risk(
    project: &Project,
    rel: &str,
    direct_consumers: &[StructuralEdge],
    cross_boundary_consumers: &[StructuralEdge],
    contract_risks: &[StructuralEdge],
) -> (Risk, Vec<String>) {
    let Some(file) = project.files.get(rel) else {
        return (
            Risk::Medium,
            vec!["changed file is not indexed".to_string()],
        );
    };
    let mut risk = Risk::Low;
    let mut reasons = Vec::new();
    let mut bump = |level, reason: &str| {
        risk = risk.max(level);
        reasons.push(reason.to_string());
    };
    if file.has_role("generated") {
        bump(Risk::Critical, "generated file changed");
    }
    if file.has_role("public_boundary") {
        bump(Risk::Critical, "public boundary changed");
    }
    if file.has_role("schema_contract") {
        bump(Risk::High, "schema or DTO contract changed");
    }
    if file.has_role("semantic_anchor") {
        bump(Risk::High, "semantic anchor changed");
    }
    if file.has_role("runtime_state") {
        bump(Risk::MediumHigh, "runtime state surface changed");
    }
    if file.has_role("persistence") {
        bump(Risk::High, "persistence surface changed");
    }
    if !contract_risks.is_empty() {
        bump(Risk::High, "contract surface participates");
    }
    if !cross_boundary_consumers.is_empty() {
        bump(Risk::High, "consumer crosses package or domain boundary");
    }
    if direct_consumers.len() >= 3 {
        bump(Risk::High, "multiple direct consumers");
    } else if !direct_consumers.is_empty() {
        bump(Risk::Medium, "direct consumer exists");
    }
    if reasons.is_empty() {
        reasons.push("bounded implementation change".to_string());
    }
    (risk, unique(reasons))
}

fn missing_file_summary(project: &Project, rel: &str) -> FileSummary {
    FileSummary {
        path: rel.to_string(),
        kind: "missing".to_string(),
        package: package_name_for_file(project, rel),
        language: "unknown".to_string(),
        lines: 0,
        roles: Vec::new(),
        symbols: Vec::new(),
        exports: Vec::new(),
        imports: Vec::new(),
        imported_by_count: 0,
    }
}

pub fn verification_plan(
    project: &Project,
    changed: &[String],
    impacted: &[String],
) -> VerificationPlan {
    let all_files: Vec<String> = changed
        .iter()
        .chain(impacted.iter())
        .cloned()
        .collect::<Vec<_>>();
    let domains = if all_files.is_empty() {
        project.domains.iter().collect::<Vec<_>>()
    } else {
        impacted_domains(project, &all_files)
    };
    let max_risk = all_files
        .iter()
        .map(|f| risk_for_file(project, f).0)
        .max()
        .unwrap_or(Risk::Low);

    let mut minimal = project.anchors.verification.default.clone();
    if minimal.is_empty() {
        minimal = infer_minimal_commands(project, &domains, &all_files, changed);
    }
    let mut recommended = Vec::new();
    if matches!(max_risk, Risk::MediumHigh | Risk::High | Risk::Critical)
        && let Some(typecheck) = find_script(project, &["typecheck", "tsc", "check"])
    {
        recommended.push(typecheck);
    }
    if matches!(max_risk, Risk::High | Risk::Critical) {
        recommended.push("codemap boundaries --changed".to_string());
    }
    let mut full = Vec::new();
    if matches!(max_risk, Risk::Critical)
        && let Some(test) = find_script(project, &["test"])
    {
        full.push(test);
    }
    VerificationPlan {
        minimal: unique(minimal).into_iter().take(3).collect(),
        recommended: unique(recommended).into_iter().take(3).collect(),
        full_only_if_triggered: unique(full).into_iter().take(3).collect(),
    }
}

pub fn boundary_report(
    project: &Project,
    changed_only: Option<&BTreeSet<String>>,
) -> BoundaryReport {
    BoundaryReport {
        kind: "boundary_report",
        schema_version: "2",
        findings: boundary_findings(project, changed_only),
    }
}

pub fn boundary_findings(
    project: &Project,
    changed_only: Option<&BTreeSet<String>>,
) -> Vec<BoundaryFinding> {
    let mut findings = Vec::new();
    let root_domain = root_domain(project);
    let semantic_anchor_changed = changed_only
        .map(|changed| {
            changed
                .iter()
                .any(|rel| is_semantic_anchor_path(project, rel))
        })
        .unwrap_or(false);
    let edge_scope = if semantic_anchor_changed {
        None
    } else {
        changed_only
    };
    for file in project.files.values() {
        if file.has_role("generated")
            && let Some(changed) = changed_only
            && changed.contains(&file.rel)
        {
            findings.push(BoundaryFinding {
                from: file.rel.clone(),
                to: String::new(),
                status: "forbidden".to_string(),
                reason: "generated file edited directly".to_string(),
                recovery: vec!["Edit the source input or generator, then regenerate.".to_string()],
                provenance: "heuristic".to_string(),
                strength: "medium".to_string(),
            });
        }
        for target in &file.resolved_imports {
            for rule in &project.anchors.boundaries.forbidden {
                if rule.from.is_empty() || rule.to.is_empty() {
                    continue;
                }
                let from = resolve_domain_pattern(&root_domain, &rule.from);
                let to = resolve_domain_pattern(&root_domain, &rule.to);
                if let Some(changed) = edge_scope
                    && !file_boundary_edge_touched(file, target, changed)
                {
                    continue;
                }
                if glob_match(&from, &file.rel) && glob_match(&to, target) {
                    findings.push(BoundaryFinding {
                        from: file.rel.clone(),
                        to: target.clone(),
                        status: rule
                            .status
                            .clone()
                            .unwrap_or_else(|| "forbidden".to_string()),
                        reason: rule.reason.clone(),
                        recovery: rule.recovery.clone(),
                        provenance: "semantic_anchor".to_string(),
                        strength: "hard".to_string(),
                    });
                }
            }
        }
    }
    for edge in &project.package_edges {
        if let Some(changed) = edge_scope
            && !package_edge_touched(edge, changed)
        {
            continue;
        }
        for rule in &project.anchors.boundaries.forbidden {
            if rule.from.is_empty() || rule.to.is_empty() {
                continue;
            }
            let from = resolve_domain_pattern(&root_domain, &rule.from);
            let to = resolve_domain_pattern(&root_domain, &rule.to);
            if package_edge_matches_rule(&from, &edge.from)
                && package_edge_matches_rule(&to, &edge.to)
            {
                let mut reason = rule.reason.clone();
                if !reason.is_empty() {
                    reason.push_str("; ");
                }
                reason.push_str(&format!(
                    "package manifest dependency `{}` from {}",
                    edge.dependency, edge.source
                ));
                findings.push(BoundaryFinding {
                    from: edge.from_manifest.clone(),
                    to: edge.to_manifest.clone().unwrap_or_else(|| edge.to.clone()),
                    status: rule
                        .status
                        .clone()
                        .unwrap_or_else(|| "forbidden".to_string()),
                    reason,
                    recovery: rule.recovery.clone(),
                    provenance: "package_manifest+semantic_anchor".to_string(),
                    strength: "hard".to_string(),
                });
            }
        }
    }
    for path in package_transitive_paths(project, 4) {
        if let Some(changed) = edge_scope
            && !path
                .manifests
                .iter()
                .any(|manifest| changed.contains(manifest))
        {
            continue;
        }
        for rule in &project.anchors.boundaries.forbidden {
            if rule.from.is_empty() || rule.to.is_empty() {
                continue;
            }
            let from = resolve_domain_pattern(&root_domain, &rule.from);
            let to = resolve_domain_pattern(&root_domain, &rule.to);
            if package_edge_matches_rule(&from, &path.from)
                && package_edge_matches_rule(&to, &path.to)
            {
                let mut reason = rule.reason.clone();
                if !reason.is_empty() {
                    reason.push_str("; ");
                }
                reason.push_str(&format!(
                    "transitive package manifest dependency path `{}`",
                    path.dependencies.join(" -> ")
                ));
                findings.push(BoundaryFinding {
                    from: path.from_manifest.clone(),
                    to: path.to_manifest.clone().unwrap_or_else(|| path.to.clone()),
                    status: rule
                        .status
                        .clone()
                        .unwrap_or_else(|| "forbidden".to_string()),
                    reason,
                    recovery: rule.recovery.clone(),
                    provenance: "package_manifest_transitive+semantic_anchor".to_string(),
                    strength: "hard".to_string(),
                });
            }
        }
    }
    findings
}

fn is_semantic_anchor_path(project: &Project, rel: &str) -> bool {
    project
        .files
        .get(rel)
        .map(|file| file.has_role("semantic_anchor"))
        .unwrap_or_else(|| {
            matches!(
                Path::new(rel).file_name().and_then(|name| name.to_str()),
                Some(".ctx.yml" | ".ctx.yaml" | ".ctx.json")
            )
        })
}

fn file_boundary_edge_touched(
    file: &crate::model::FileInfo,
    target: &str,
    changed: &BTreeSet<String>,
) -> bool {
    changed.contains(&file.rel) || changed.contains(target)
}

struct PackageGraphPath {
    from: String,
    from_manifest: String,
    to: String,
    to_manifest: Option<String>,
    dependencies: Vec<String>,
    manifests: Vec<String>,
}

fn package_transitive_paths(project: &Project, max_depth: usize) -> Vec<PackageGraphPath> {
    let mut outgoing: BTreeMap<&str, Vec<&crate::model::PackageDependency>> = BTreeMap::new();
    for edge in &project.package_edges {
        outgoing.entry(&edge.from).or_default().push(edge);
    }
    let mut paths = Vec::new();
    for first in &project.package_edges {
        let mut first_manifests = vec![first.from_manifest.clone()];
        append_manifest(&mut first_manifests, first.to_manifest.as_deref());
        append_manifest(&mut first_manifests, first.workspace_manifest.as_deref());
        let mut queue = VecDeque::from([(
            first.to.clone(),
            vec![first.dependency.clone()],
            first_manifests,
            BTreeSet::from([first.from.clone(), first.to.clone()]),
            1usize,
        )]);
        while let Some((current, dependencies, manifests, seen, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            let Some(next_edges) = outgoing.get(current.as_str()) else {
                continue;
            };
            for edge in next_edges {
                if seen.contains(&edge.to) {
                    continue;
                }
                let mut next_dependencies = dependencies.clone();
                next_dependencies.push(edge.dependency.clone());
                let mut next_manifests = manifests.clone();
                append_manifest(&mut next_manifests, Some(&edge.from_manifest));
                append_manifest(&mut next_manifests, edge.to_manifest.as_deref());
                append_manifest(&mut next_manifests, edge.workspace_manifest.as_deref());
                let next_depth = depth + 1;
                paths.push(PackageGraphPath {
                    from: first.from.clone(),
                    from_manifest: first.from_manifest.clone(),
                    to: edge.to.clone(),
                    to_manifest: edge.to_manifest.clone(),
                    dependencies: next_dependencies.clone(),
                    manifests: next_manifests.clone(),
                });
                let mut next_seen = seen.clone();
                next_seen.insert(edge.to.clone());
                queue.push_back((
                    edge.to.clone(),
                    next_dependencies,
                    next_manifests,
                    next_seen,
                    next_depth,
                ));
            }
        }
    }
    paths
}

fn package_edge_touched(
    edge: &crate::model::PackageDependency,
    changed: &BTreeSet<String>,
) -> bool {
    changed.contains(&edge.from_manifest)
        || edge
            .to_manifest
            .as_ref()
            .map(|manifest| changed.contains(manifest))
            .unwrap_or(false)
        || edge
            .workspace_manifest
            .as_ref()
            .map(|manifest| changed.contains(manifest))
            .unwrap_or(false)
}

fn append_manifest(manifests: &mut Vec<String>, manifest: Option<&str>) {
    if let Some(manifest) = manifest
        && !manifests.iter().any(|existing| existing == manifest)
    {
        manifests.push(manifest.to_string());
    }
}

fn package_edge_matches_rule(pattern: &str, package_path: &str) -> bool {
    let package_path = package_path.trim_end_matches('/');
    let probes = if package_path == "." {
        vec![
            "package.json".to_string(),
            "Cargo.toml".to_string(),
            "go.mod".to_string(),
            "pyproject.toml".to_string(),
            "src/__package_dependency__".to_string(),
            "__package_dependency__".to_string(),
        ]
    } else {
        vec![
            format!("{package_path}/package.json"),
            format!("{package_path}/Cargo.toml"),
            format!("{package_path}/go.mod"),
            format!("{package_path}/pyproject.toml"),
            format!("{package_path}/src/__package_dependency__"),
            format!("{package_path}/__package_dependency__"),
        ]
    };
    probes.iter().any(|probe| glob_match(pattern, probe))
}

fn root_domain(project: &Project) -> Domain {
    project
        .domains
        .iter()
        .find(|domain| domain.path == ".")
        .cloned()
        .unwrap_or_else(|| {
            project.domains.first().cloned().unwrap_or_else(|| Domain {
                id: "repo".to_string(),
                path: ".".to_string(),
                config_path: None,
            })
        })
}

fn path_is_in_domain(rel: &str, domain: &Domain) -> bool {
    let prefix = domain.path.trim_end_matches('/');
    prefix == "." || rel == prefix || rel.starts_with(&format!("{prefix}/"))
}

fn domain_for_path<'a>(project: &'a Project, path: &str) -> &'a Domain {
    let rel = if Path::new(path).is_absolute() {
        let absolute = Path::new(path)
            .canonicalize()
            .unwrap_or_else(|_| Path::new(path).to_path_buf());
        absolute
            .strip_prefix(&project.root)
            .map(|p| repo::normalize_rel_path(&p.to_string_lossy()))
            .unwrap_or_else(|_| repo::normalize_rel_path(path))
    } else {
        repo::normalize_rel_path(path)
    };
    let mut best = project
        .domains
        .iter()
        .find(|domain| domain.path == ".")
        .unwrap_or(&project.domains[0]);
    let mut best_len = 0usize;
    for domain in &project.domains {
        if path_is_in_domain(&rel, domain) {
            let prefix = domain.path.trim_end_matches('/');
            let len = if prefix == "." { 0 } else { prefix.len() };
            if len >= best_len {
                best = domain;
                best_len = len;
            }
        }
    }
    best
}

fn domain_by_rel<'a>(project: &'a Project, rel: &str) -> Option<&'a Domain> {
    Some(domain_for_path(project, rel))
}

fn scoped_domain_path_for_rel(
    project: &Project,
    rel: &str,
    scope: Option<&Domain>,
) -> Option<String> {
    if let Some(domain) = scope
        && (domain.path == "."
            || rel == domain.path
            || rel.starts_with(&format!("{}/", domain.path.trim_end_matches('/'))))
    {
        return Some(domain.path.clone());
    }
    domain_by_rel(project, rel).map(|domain| domain.path.clone())
}

fn risk_for_file(project: &Project, rel: &str) -> (Risk, Vec<String>) {
    let Some(file) = project.files.get(rel) else {
        return (Risk::Medium, vec!["file not found in scan".to_string()]);
    };
    let mut risk = Risk::Low;
    let mut reasons = Vec::new();
    let mut bump = |level, reason: &str| {
        risk = risk.max(level);
        reasons.push(reason.to_string());
    };
    if file.has_role("generated") {
        bump(Risk::Critical, "generated file");
    }
    if file.has_role("semantic_anchor") {
        bump(Risk::High, "semantic context anchor");
    }
    if file.has_role("public_boundary") {
        bump(Risk::Critical, "public boundary");
    }
    if file.has_role("schema_contract") {
        bump(Risk::High, "schema/contract/DTO");
    }
    if file.has_role("state_model") || file.has_role("persistence") {
        bump(Risk::High, "state model / persistence");
    }
    if file.has_role("runtime_state") {
        bump(Risk::MediumHigh, "runtime state / session/controller");
    }
    if file.has_role("map_engine") {
        bump(Risk::High, "structural map engine");
    }
    if file.has_role("cli_surface") {
        bump(Risk::High, "CLI command surface");
    }
    if file.has_role("build_ci") {
        bump(Risk::MediumHigh, "build/CI configuration");
    }
    if file.has_role("repo_discovery") {
        bump(Risk::MediumHigh, "repo discovery / inventory");
    }
    if file.has_role("cache") {
        bump(Risk::Medium, "external cache / fingerprint");
    }
    let fan_in = project
        .reverse_imports
        .get(rel)
        .map(|x| x.len())
        .unwrap_or(0);
    let fan_out = file.resolved_imports.len();
    if fan_in >= 8 {
        bump(Risk::Critical, &format!("high fan-in ({fan_in} importers)"));
    } else if fan_in >= 3 {
        bump(Risk::High, &format!("multiple importers ({fan_in})"));
    }
    if fan_out >= 12 {
        bump(Risk::Medium, &format!("high fan-out ({fan_out} imports)"));
    }
    if file.has_role("test") {
        bump(Risk::Low, "test file");
    }
    if reasons.is_empty() {
        reasons.push("isolated or low-risk implementation file".to_string());
    }
    (risk, unique(reasons))
}

fn package_consumer_manifests(
    project: &Project,
    changed: &[String],
    depth: usize,
    limit: usize,
) -> Vec<String> {
    if depth == 0 || limit == 0 {
        return Vec::new();
    }
    let mut roots = BTreeSet::new();
    for rel in changed {
        if !requires_package_consumer_expansion(project, rel) {
            continue;
        }
        if let Some(package) = package_for_rel(project, rel) {
            roots.insert(package.path.clone());
        }
    }
    if roots.is_empty() {
        let workspace_roots = workspace_manifest_consumers(project, changed, depth, limit);
        return workspace_roots;
    }
    let mut traversal = PackageConsumerTraversal {
        seen: roots.clone(),
        queue: roots.into_iter().map(|path| (path, 0)).collect(),
        out: Vec::new(),
        out_seen: BTreeSet::new(),
    };
    seed_workspace_manifest_consumers(project, changed, depth, limit, &mut traversal);
    while let Some((package_path, d)) = traversal.queue.pop_front() {
        if traversal.out.len() >= limit {
            break;
        }
        for edge in project
            .package_edges
            .iter()
            .filter(|edge| edge.to == package_path)
        {
            if traversal.seen.insert(edge.from.clone()) {
                if traversal.out_seen.insert(edge.from_manifest.clone()) {
                    traversal.out.push(edge.from_manifest.clone());
                }
                if d + 1 < depth {
                    traversal.queue.push_back((edge.from.clone(), d + 1));
                }
                if traversal.out.len() >= limit {
                    break;
                }
            }
        }
    }
    traversal.out
}

struct PackageConsumerTraversal {
    seen: BTreeSet<String>,
    queue: VecDeque<(String, usize)>,
    out: Vec<String>,
    out_seen: BTreeSet<String>,
}

fn workspace_manifest_consumers(
    project: &Project,
    changed: &[String],
    depth: usize,
    limit: usize,
) -> Vec<String> {
    let mut traversal = PackageConsumerTraversal {
        seen: BTreeSet::new(),
        queue: VecDeque::new(),
        out: Vec::new(),
        out_seen: BTreeSet::new(),
    };
    seed_workspace_manifest_consumers(project, changed, depth, limit, &mut traversal);
    while let Some((package_path, d)) = traversal.queue.pop_front() {
        if traversal.out.len() >= limit {
            break;
        }
        for edge in project
            .package_edges
            .iter()
            .filter(|edge| edge.to == package_path)
        {
            if traversal.seen.insert(edge.from.clone()) {
                if traversal.out_seen.insert(edge.from_manifest.clone()) {
                    traversal.out.push(edge.from_manifest.clone());
                }
                if d + 1 < depth {
                    traversal.queue.push_back((edge.from.clone(), d + 1));
                }
                if traversal.out.len() >= limit {
                    break;
                }
            }
        }
    }
    traversal.out
}

fn seed_workspace_manifest_consumers(
    project: &Project,
    changed: &[String],
    depth: usize,
    limit: usize,
    traversal: &mut PackageConsumerTraversal,
) {
    if depth == 0 || limit == 0 {
        return;
    }
    for rel in changed {
        if !requires_package_consumer_expansion(project, rel) {
            continue;
        }
        for edge in project
            .package_edges
            .iter()
            .filter(|edge| edge.workspace_manifest.as_deref() == Some(rel.as_str()))
        {
            if traversal.seen.insert(edge.from.clone()) {
                if traversal.out_seen.insert(edge.from_manifest.clone()) {
                    traversal.out.push(edge.from_manifest.clone());
                }
                if 1 < depth {
                    traversal.queue.push_back((edge.from.clone(), 1));
                }
                if traversal.out.len() >= limit {
                    return;
                }
            }
        }
    }
}

fn requires_package_consumer_expansion(project: &Project, rel: &str) -> bool {
    let Some(file) = project.files.get(rel) else {
        return false;
    };
    file.has_role("public_boundary")
        || file.has_role("schema_contract")
        || matches!(
            Path::new(rel).file_name().and_then(|name| name.to_str()),
            Some("package.json" | "Cargo.toml" | "go.mod" | "pyproject.toml")
        )
}

fn package_for_rel<'a>(project: &'a Project, rel: &str) -> Option<&'a crate::model::PackageInfo> {
    let mut best = None;
    let mut best_len = 0usize;
    for package in &project.packages {
        let prefix = package.path.trim_end_matches('/');
        let matches = prefix == "."
            || rel == package.manifest
            || rel == prefix
            || rel.starts_with(&format!("{prefix}/"));
        if matches {
            let len = if prefix == "." { 0 } else { prefix.len() };
            if len >= best_len {
                best = Some(package);
                best_len = len;
            }
        }
    }
    best
}

fn impacted_domains<'a>(project: &'a Project, files: &[String]) -> Vec<&'a Domain> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for file in files {
        if let Some(domain) = domain_by_rel(project, file)
            && seen.insert(domain.id.clone())
        {
            out.push(domain);
        }
    }
    out
}

fn infer_minimal_commands(
    project: &Project,
    domains: &[&Domain],
    files: &[String],
    changed: &[String],
) -> Vec<String> {
    let root_test = find_script(project, &["test"]);
    let changed_source_package = single_source_package_for_files(project, changed);
    let changed_domains = impacted_domains(project, changed);
    if let Some(package) = changed_source_package
        && let Some(command) =
            package_minimal_command(project, package, &changed_domains, root_test.as_deref())
    {
        return vec![command];
    }
    if changed_source_package.is_some()
        && let Some(package) = single_package_for_files(project, files)
        && let Some(command) =
            package_minimal_command(project, package, domains, root_test.as_deref())
    {
        return vec![command];
    }
    if let Some(test) = root_test {
        if (changed.is_empty() || changed_source_package.is_some())
            && domains.len() == 1
            && domains[0].path != "."
            && project.package_manager != "bun"
        {
            return vec![format!("{test} {}", domains[0].path)];
        }
        return vec![test];
    }
    match project.package_manager.as_str() {
        "cargo" => vec!["cargo test".to_string()],
        "go" => vec!["go test ./...".to_string()],
        "python" => vec!["pytest".to_string()],
        _ => vec!["run the nearest domain tests for the changed files".to_string()],
    }
}

fn single_source_package_for_files<'a>(
    project: &'a Project,
    files: &[String],
) -> Option<&'a crate::model::PackageInfo> {
    if files.is_empty() {
        return None;
    }
    let package = single_package_for_files(project, files)?;
    files
        .iter()
        .all(|file| {
            project
                .files
                .get(file)
                .map(|info| is_package_implementation_source(file, info, package))
                .unwrap_or(false)
        })
        .then_some(package)
}

fn is_package_implementation_source(
    rel: &str,
    info: &crate::model::FileInfo,
    package: &crate::model::PackageInfo,
) -> bool {
    if rel == package.manifest || !repo::is_source_ext(&info.ext) {
        return false;
    }
    if [
        "generated",
        "build_ci",
        "semantic_anchor",
        "agent_bootstrap",
    ]
    .iter()
    .any(|role| info.roles.contains(*role))
    {
        return false;
    }
    let name = std::path::Path::new(rel)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    !is_tooling_config_source_name(&name)
}

fn is_tooling_config_source_name(name: &str) -> bool {
    name.contains(".config.")
        || name.ends_with(".config")
        || name.contains(".conf.")
        || name.ends_with(".conf")
        || name.starts_with(".eslintrc.")
        || name.starts_with(".prettierrc.")
        || name.starts_with(".babelrc.")
        || matches!(
            name,
            "gulpfile.js"
                | "gulpfile.ts"
                | "gruntfile.js"
                | "gruntfile.ts"
                | "karma.conf.js"
                | "karma.conf.ts"
        )
}

fn single_package_for_files<'a>(
    project: &'a Project,
    files: &[String],
) -> Option<&'a crate::model::PackageInfo> {
    let mut selected: Option<&crate::model::PackageInfo> = None;
    for file in files {
        let package = package_for_rel(project, file)?;
        match selected {
            Some(current) if current.path != package.path => return None,
            Some(_) => {}
            None => selected = Some(package),
        }
    }
    selected
}

fn package_minimal_command(
    project: &Project,
    package: &crate::model::PackageInfo,
    domains: &[&Domain],
    root_test: Option<&str>,
) -> Option<String> {
    match package.ecosystem.as_str() {
        "javascript" => javascript_package_test_command(project, package, domains, root_test),
        "rust" => Some(if package.path == "." {
            "cargo test".to_string()
        } else if root_cargo_workspace_includes(project, &package.path) {
            format!("cargo test -p {}", shell_quote(&package.name))
        } else {
            format!("cd {} && cargo test", shell_quote(&package.path))
        }),
        "go" => Some(if package.path == "." {
            "go test ./...".to_string()
        } else {
            format!("cd {} && go test ./...", shell_quote(&package.path))
        }),
        "python" => Some(if package.path == "." {
            "pytest".to_string()
        } else {
            format!("cd {} && pytest", shell_quote(&package.path))
        }),
        "swift" => Some(if package.path == "." {
            "swift test".to_string()
        } else {
            format!("cd {} && swift test", shell_quote(&package.path))
        }),
        _ => None,
    }
}

fn root_cargo_workspace_includes(project: &Project, package_path: &str) -> bool {
    let Ok(text) = std::fs::read_to_string(project.root.join("Cargo.toml")) else {
        return false;
    };
    cargo_workspace_values(&text, "members")
        .into_iter()
        .any(|pattern| cargo_workspace_pattern_matches(&pattern, package_path))
        && !cargo_workspace_values(&text, "exclude")
            .into_iter()
            .any(|pattern| cargo_workspace_pattern_matches(&pattern, package_path))
}

fn cargo_workspace_values(text: &str, wanted_key: &str) -> Vec<String> {
    toml::from_str::<toml::Value>(text)
        .ok()
        .and_then(|value| value.get("workspace").cloned())
        .and_then(|workspace| workspace.get(wanted_key).cloned())
        .and_then(|value| toml_string_array(&value))
        .unwrap_or_default()
}

fn cargo_workspace_pattern_matches(pattern: &str, package_path: &str) -> bool {
    let pattern = repo::normalize_rel_path(pattern.trim().trim_start_matches("./"));
    !pattern.is_empty() && (pattern == package_path || glob_match(&pattern, package_path))
}

fn toml_string_array(value: &toml::Value) -> Option<Vec<String>> {
    Some(
        value
            .as_array()?
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .filter(|item| !item.is_empty())
            .collect(),
    )
}

fn javascript_package_test_command(
    project: &Project,
    package: &crate::model::PackageInfo,
    domains: &[&Domain],
    root_test: Option<&str>,
) -> Option<String> {
    if !javascript_package_has_script(project, package, "test") {
        return None;
    }
    if is_javascript_package_manager(&project.package_manager)
        && let Some(test) = root_test
        && domains.len() == 1
        && domains[0].path == package.path
        && package.path != "."
        && project.package_manager != "bun"
    {
        return Some(format!("{test} {}", package.path));
    }
    let runner = javascript_runner_for_package(project, package);
    let command = javascript_test_command(&runner);
    Some(if package.path == "." {
        command
    } else {
        format!("cd {} && {command}", shell_quote(&package.path))
    })
}

fn javascript_package_has_script(
    project: &Project,
    package: &crate::model::PackageInfo,
    script: &str,
) -> bool {
    let Ok(text) = std::fs::read_to_string(project.root.join(&package.manifest)) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    value
        .get("scripts")
        .and_then(|scripts| scripts.get(script))
        .and_then(|value| value.as_str())
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

fn javascript_runner_for_package(project: &Project, package: &crate::model::PackageInfo) -> String {
    for rel in ancestor_paths(&package.path) {
        let dir = if rel == "." {
            project.root.clone()
        } else {
            project.root.join(&rel)
        };
        if dir.join("pnpm-workspace.yaml").exists() || dir.join("pnpm-lock.yaml").exists() {
            return "pnpm".to_string();
        }
        if dir.join("yarn.lock").exists() {
            return "yarn".to_string();
        }
        if dir.join("bun.lockb").exists() {
            return "bun".to_string();
        }
        if dir.join("package-lock.json").exists() {
            return "npm".to_string();
        }
    }
    if is_javascript_package_manager(&project.package_manager) {
        project.package_manager.clone()
    } else {
        "npm".to_string()
    }
}

fn ancestor_paths(rel: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = repo::normalize_rel_path(rel);
    loop {
        out.push(if current.is_empty() {
            ".".to_string()
        } else {
            current.clone()
        });
        if current.is_empty() || current == "." {
            break;
        }
        let parent = Path::new(&current)
            .parent()
            .map(|path| repo::normalize_rel_path(&path.to_string_lossy()))
            .unwrap_or_else(|| ".".to_string());
        if parent == current {
            break;
        }
        current = parent;
    }
    if !out.iter().any(|path| path == ".") {
        out.push(".".to_string());
    }
    out
}

fn is_javascript_package_manager(value: &str) -> bool {
    matches!(value, "npm" | "pnpm" | "yarn" | "bun")
}

fn javascript_test_command(runner: &str) -> String {
    match runner {
        "yarn" => "yarn test".to_string(),
        "bun" => "bun test".to_string(),
        "pnpm" => "pnpm test".to_string(),
        _ => "npm test".to_string(),
    }
}

fn find_script(project: &Project, names: &[&str]) -> Option<String> {
    project
        .scripts
        .iter()
        .filter_map(|script| {
            script_match_rank(script, names).map(|rank| (rank, script.command.clone()))
        })
        .min_by(|(left_rank, left_command), (right_rank, right_command)| {
            left_rank
                .cmp(right_rank)
                .then_with(|| left_command.cmp(right_command))
        })
        .map(|(_, command)| command)
}

fn script_match_rank(script: &crate::model::ScriptInfo, names: &[&str]) -> Option<usize> {
    let script_name = script.name.to_ascii_lowercase();
    let script_command = script.command.to_ascii_lowercase();
    let wanted: Vec<String> = names.iter().map(|name| name.to_ascii_lowercase()).collect();

    for (index, name) in wanted.iter().enumerate() {
        if script_name == name.as_str() {
            return Some(index);
        }
    }
    for (index, name) in wanted.iter().enumerate() {
        if script_name.contains(name) {
            return Some(10 + index);
        }
    }
    for (index, name) in wanted.iter().enumerate() {
        if script_command.contains(name) {
            return Some(20 + index);
        }
    }
    None
}

pub fn resolve_anchor_path(project: &Project, pattern: &str) -> String {
    let domain = root_domain(project);
    resolve_domain_pattern(&domain, pattern)
}

fn resolve_domain_pattern(domain: &Domain, pattern: &str) -> String {
    let p = pattern.trim().trim_start_matches("./");
    if p.starts_with('/') {
        return repo::normalize_rel_path(p);
    }
    if domain.path == "." {
        return repo::normalize_rel_path(p);
    }
    let domain_path = domain.path.trim_end_matches('/');
    if p == domain_path || p.starts_with(&format!("{domain_path}/")) {
        return repo::normalize_rel_path(p);
    }
    if p.starts_with("domains/")
        || p.starts_with("packages/")
        || p.starts_with("apps/")
        || p.starts_with("services/")
        || p.starts_with("libs/")
        || p.starts_with("crates/")
        || p.starts_with("modules/")
        || p.starts_with("cmd/")
        || p.starts_with("components/")
    {
        repo::normalize_rel_path(p)
    } else {
        repo::normalize_rel_path(&format!("{domain_path}/{p}"))
    }
}

fn glob_match(pattern: &str, value: &str) -> bool {
    glob_match_parts(
        &pattern.split('/').collect::<Vec<_>>(),
        &value.split('/').collect::<Vec<_>>(),
    )
}

fn glob_match_parts(pattern: &[&str], value: &[&str]) -> bool {
    if pattern.is_empty() {
        return value.is_empty();
    }
    if pattern[0] == "**" {
        return glob_match_parts(&pattern[1..], value)
            || (!value.is_empty() && glob_match_parts(pattern, &value[1..]));
    }
    if value.is_empty() {
        return false;
    }
    segment_match(pattern[0], value[0]) && glob_match_parts(&pattern[1..], &value[1..])
}

fn segment_match(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if !pattern.contains('*') {
        return pattern == value;
    }
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut rest = value;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 && !rest.starts_with(part) {
            return false;
        }
        if let Some(pos) = rest.find(part) {
            rest = &rest[pos + part.len()..];
        } else {
            return false;
        }
    }
    pattern.ends_with('*')
        || parts
            .last()
            .map(|last| value.ends_with(last))
            .unwrap_or(true)
}

fn unique(items: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for item in items {
        if !item.is_empty() && seen.insert(item.clone()) {
            out.push(item);
        }
    }
    out
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proof(
        command: &str,
        path: &str,
        evidence: &str,
        strength: EvidenceStrength,
    ) -> ProofSurface {
        ProofSurface {
            command: Some(command.to_string()),
            path: Some(path.to_string()),
            evidence: evidence.to_string(),
            strength,
            reason: format!("{evidence} reason"),
        }
    }

    #[test]
    fn proof_surfaces_dedupe_by_command_path_and_keep_strongest_evidence() {
        let proofs = unique_proof_surfaces(vec![
            proof(
                "pnpm exec vitest run tests/a.test.ts",
                "tests/a.test.ts",
                "test_surface_tokens",
                EvidenceStrength::Medium,
            ),
            proof(
                "pnpm exec vitest run tests/a.test.ts",
                "tests/a.test.ts",
                "test_name",
                EvidenceStrength::High,
            ),
            proof(
                "pnpm exec vitest run tests/a.test.ts",
                "tests/a.test.ts",
                "test_import",
                EvidenceStrength::High,
            ),
            proof(
                "pnpm exec vitest run tests/b.test.ts",
                "tests/b.test.ts",
                "test_surface_tokens",
                EvidenceStrength::Medium,
            ),
        ]);

        assert_eq!(proofs.len(), 2);
        assert_eq!(proofs[0].path.as_deref(), Some("tests/a.test.ts"));
        assert_eq!(proofs[0].evidence, "test_import");
        assert_eq!(proofs[0].strength, EvidenceStrength::High);
        assert_eq!(proofs[1].path.as_deref(), Some("tests/b.test.ts"));
    }
}
