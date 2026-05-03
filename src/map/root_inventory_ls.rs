pub(crate) fn root_inventory_ls_report(root: &Path, files: &[String], limit: usize) -> LsReport {
    let mut grouped: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut hidden_support = BTreeSet::new();
    let mut recursive_hidden = 0usize;
    let mut source_edge_hidden = 0usize;

    for dir in inventory_top_level_dirs(files) {
        if is_support_artifact_path(&dir) {
            hidden_support.insert(dir);
            continue;
        }
        if let Some(role) = inventory_dir_role(&dir) {
            inventory_push(&mut grouped, &role, &dir);
        }
        inventory_push(&mut grouped, "dir", &dir);
    }

    let (script_labels, mut edges) = inventory_root_script_edges(root, files);
    for label in script_labels {
        inventory_push(&mut grouped, "script", &label);
    }

    for rel in files {
        if is_support_artifact_path(rel) {
            hidden_support.insert(inventory_support_unit(rel));
            continue;
        }
        let direct = !rel.contains('/');
        let kind = inventory_file_kind(rel);
        if let Some(package_kind) = inventory_package_kind(root, rel) {
            inventory_push(&mut grouped, &package_kind, rel);
        }
        if direct || inventory_recursive_structural_kind(&kind, rel) {
            inventory_push(&mut grouped, &kind, rel);
        } else {
            recursive_hidden += 1;
            if kind == "source" {
                source_edge_hidden += 1;
            }
        }
    }

    edges.extend(inventory_workspace_edges(root, files));
    edges.sort_by(|a, b| {
        a.from
            .cmp(&b.from)
            .then_with(|| inventory_edge_priority(a).cmp(&inventory_edge_priority(b)))
            .then_with(|| a.edge_type.cmp(&b.edge_type))
            .then_with(|| a.to.cmp(&b.to))
            .then_with(|| a.evidence.cmp(&b.evidence))
    });
    edges.dedup_by(|a, b| {
        a.from == b.from && a.to == b.to && a.edge_type == b.edge_type && a.evidence == b.evidence
    });

    let mut hidden = Vec::new();
    let mut surfaces = inventory_surfaces(".", grouped);
    let surface_count = surfaces.len();
    surfaces.truncate(limit);
    if surface_count > surfaces.len() {
        hidden.push(HiddenGroup {
            reason: "directory surfaces hidden by limit".to_string(),
            count: surface_count - surfaces.len(),
            expand: "codemap ls . --all".to_string(),
        });
    }

    let edge_count = edges.len();
    if edge_count > limit {
        edges.truncate(limit);
        hidden.push(HiddenGroup {
            reason: "inventory edges hidden by limit".to_string(),
            count: edge_count - edges.len(),
            expand: "codemap ls . --all".to_string(),
        });
    }
    if !hidden_support.is_empty() {
        hidden.push(HiddenGroup {
            reason: "support artifacts hidden".to_string(),
            count: hidden_support.len(),
            expand: "codemap ls . --all".to_string(),
        });
    }
    if recursive_hidden > 0 {
        hidden.push(HiddenGroup {
            reason: "recursive files below this level hidden".to_string(),
            count: recursive_hidden,
            expand: "codemap ls . --all".to_string(),
        });
    }
    if source_edge_hidden > 0 {
        hidden.push(HiddenGroup {
            reason: "full-index source edges hidden by bounded root inventory".to_string(),
            count: source_edge_hidden,
            expand: "codemap ls . --all".to_string(),
        });
    }

    LsReport {
        kind: "ls_report",
        schema_version: "3",
        path: ".".to_string(),
        mode: "directory".to_string(),
        anchor: None,
        directory: surfaces,
        edges,
        hidden,
        next: directory_next_commands("."),
    }
}

pub(crate) fn root_inventory_graph_lens(
    root: &Path,
    files: &[String],
    limit: usize,
    lens: &str,
) -> GraphLens {
    let report = root_inventory_ls_report(root, files, usize::MAX / 2);
    let mut graph_edges = Vec::new();
    let mut seen_edges = BTreeSet::new();

    for surface in &report.directory {
        for node in inventory_graph_surface_examples(surface) {
            inventory_graph_push_edge(
                &mut graph_edges,
                &mut seen_edges,
                GraphEdge {
                    from: ".".to_string(),
                    to: node.clone(),
                    edge_type: "contains".to_string(),
                    evidence: "current_level_inventory_surface".to_string(),
                    strength: surface.strength,
                    locations: vec![inventory_graph_surface_location(&node, &surface.evidence)],
                },
            );
        }
    }

    let mut nodes = inventory_graph_node_order(&report, &graph_edges);

    for edge in report.edges {
        inventory_graph_push_edge(
            &mut graph_edges,
            &mut seen_edges,
            GraphEdge {
                from: edge.from,
                to: edge.to,
                edge_type: edge.edge_type,
                evidence: edge.evidence,
                strength: edge.strength,
                locations: edge.locations,
            },
        );
    }

    let total_nodes = nodes.len();
    let limit = limit.max(1);
    if nodes.len() > limit {
        nodes.truncate(limit);
    }
    let node_set = nodes.iter().cloned().collect::<BTreeSet<_>>();
    graph_edges.retain(|edge| node_set.contains(&edge.from) && node_set.contains(&edge.to));

    let mut hidden = Vec::new();
    if total_nodes > nodes.len() {
        hidden.push(HiddenGroup {
            reason: "graph nodes hidden by limit".to_string(),
            count: total_nodes - nodes.len(),
            expand: format!("codemap graph --lens {} --limit {total_nodes}", shell_quote(lens)),
        });
    }
    hidden.extend(report.hidden.into_iter().filter(|group| {
        group.reason == "full-index source edges hidden by bounded root inventory"
            || group.reason == "inventory edges hidden by limit"
            || group.reason == "support artifacts hidden"
    }));

    GraphLens {
        kind: "graph_lens",
        schema_version: "4",
        domain: (&Domain {
            id: "repo".to_string(),
            path: ".".to_string(),
            config_path: None,
        })
            .into(),
        lens: lens.to_string(),
        nodes,
        edges: graph_edges,
        hidden,
    }
}

fn inventory_graph_node_order(report: &LsReport, graph_edges: &[GraphEdge]) -> Vec<String> {
    let mut nodes = Vec::new();
    inventory_graph_push_node(&mut nodes, ".".to_string());
    let (primary_surfaces, secondary_surfaces) = inventory_graph_surface_node_rounds(&report.directory);
    for node in primary_surfaces {
        inventory_graph_push_node(&mut nodes, node);
    }
    for node in inventory_graph_edge_node_order(&report.edges) {
        inventory_graph_push_node(&mut nodes, node);
    }
    for node in secondary_surfaces {
        inventory_graph_push_node(&mut nodes, node);
    }
    for edge in graph_edges {
        inventory_graph_push_node(&mut nodes, edge.from.clone());
        inventory_graph_push_node(&mut nodes, edge.to.clone());
    }
    nodes
}

fn inventory_graph_surface_node_rounds(surfaces: &[DirectorySurface]) -> (Vec<String>, Vec<String>) {
    let mut by_surface = surfaces
        .iter()
        .map(inventory_graph_surface_examples)
        .filter(|examples| !examples.is_empty())
        .collect::<Vec<_>>();
    let mut primary = Vec::new();
    let mut secondary = Vec::new();
    let mut round = 0usize;
    loop {
        let mut pushed = false;
        for examples in &mut by_surface {
            if let Some(node) = examples.get(round) {
                if round == 0 {
                    primary.push(node.clone());
                } else {
                    secondary.push(node.clone());
                }
                pushed = true;
            }
        }
        if !pushed {
            break;
        }
        round += 1;
    }
    (primary, secondary)
}

fn inventory_graph_surface_examples(surface: &DirectorySurface) -> Vec<String> {
    let mut examples = surface.examples.clone();
    if surface.kind == "script" {
        examples.retain(|example| inventory_graph_example_is_path_like(example));
    }
    if surface.kind != "dir" {
        examples.sort_by(|a, b| {
            inventory_graph_surface_example_priority(a)
                .cmp(&inventory_graph_surface_example_priority(b))
                .then_with(|| a.cmp(b))
        });
    }
    examples
}

fn inventory_graph_surface_example_priority(example: &str) -> usize {
    if example.ends_with('/') {
        2
    } else if inventory_graph_example_is_path_like(example) {
        0
    } else {
        1
    }
}

fn inventory_graph_example_is_path_like(example: &str) -> bool {
    example.contains('/') || !example.contains(':')
}

fn inventory_graph_edge_node_order(edges: &[StructuralEdge]) -> Vec<String> {
    let mut sources = Vec::new();
    let mut by_source: BTreeMap<String, Vec<&StructuralEdge>> = BTreeMap::new();
    for edge in edges {
        if !by_source.contains_key(&edge.from) {
            sources.push(edge.from.clone());
        }
        by_source.entry(edge.from.clone()).or_default().push(edge);
    }

    let mut nodes = Vec::new();
    let mut round = 0usize;
    loop {
        let mut pushed = false;
        for source in &sources {
            let Some(source_edges) = by_source.get(source) else {
                continue;
            };
            if let Some(edge) = source_edges.get(round) {
                nodes.push(edge.from.clone());
                nodes.push(edge.to.clone());
                pushed = true;
            }
        }
        if !pushed {
            break;
        }
        round += 1;
    }
    nodes
}

fn inventory_graph_push_node(nodes: &mut Vec<String>, node: String) {
    if !node.is_empty() && !nodes.contains(&node) {
        nodes.push(node);
    }
}

fn inventory_graph_push_edge(
    edges: &mut Vec<GraphEdge>,
    seen: &mut BTreeSet<(String, String, String)>,
    edge: GraphEdge,
) {
    if edge.from == edge.to {
        return;
    }
    if seen.insert((edge.from.clone(), edge.to.clone(), edge.edge_type.clone())) {
        edges.push(edge);
    }
}

fn inventory_graph_surface_location(node: &str, kind: &str) -> EvidenceLocation {
    if node.ends_with('/') {
        EvidenceLocation::path(node, "directory_inventory")
    } else if node.contains(':') && !node.contains('/') {
        EvidenceLocation::aggregate(kind)
    } else {
        EvidenceLocation::path(node, kind)
    }
}

fn inventory_edge_priority(edge: &StructuralEdge) -> usize {
    match edge.edge_type.as_str() {
        "runs_command" => 0,
        "declares_script" => 1,
        "workspace_member" => 2,
        "declares_run_block" => 8,
        _ => 9,
    }
}

fn inventory_top_level_dirs(files: &[String]) -> Vec<String> {
    let mut dirs = BTreeSet::new();
    for rel in files {
        if let Some((dir, _)) = rel.split_once('/') {
            dirs.insert(format!("{dir}/"));
        }
    }
    dirs.into_iter().collect()
}

fn inventory_push(grouped: &mut BTreeMap<String, BTreeSet<String>>, kind: &str, value: &str) {
    grouped
        .entry(kind.to_string())
        .or_default()
        .insert(value.to_string());
}

fn inventory_surfaces(
    scope: &str,
    grouped: BTreeMap<String, BTreeSet<String>>,
) -> Vec<DirectorySurface> {
    let mut surfaces = grouped
        .into_iter()
        .map(|(kind, files)| {
            let count = files.len();
            let examples = files.into_iter().take(5).collect::<Vec<_>>();
            DirectorySurface {
                id: directory_surface_id(scope, &kind, &examples),
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
    surfaces
}

fn inventory_dir_role(dir: &str) -> Option<String> {
    let name = dir.trim_end_matches('/').to_ascii_lowercase();
    match name.as_str() {
        ".github" | ".circleci" | ".buildkite" => Some("build_ci".to_string()),
        "docs" | "doc" | "documentation" => Some("docs".to_string()),
        "contracts" | "schemas" | "schema" | "migrations" => Some("schema_contract".to_string()),
        "deploy" | "deployment" | "infra" | "k8s" => Some("deploy".to_string()),
        "fixtures" | "examples" | "samples" => Some("fixture".to_string()),
        _ => None,
    }
}

fn inventory_file_kind(rel: &str) -> String {
    let path = Path::new(rel);
    let name = manifest_file_name(rel).to_ascii_lowercase();
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if repo::is_env_surface_name(&name) {
        return "env_config".to_string();
    }
    if inventory_lockfile_name(&name) {
        return "lockfile".to_string();
    }
    if matches!(
        name.as_str(),
        "package.json"
            | "cargo.toml"
            | "go.mod"
            | "go.work"
            | "pyproject.toml"
            | "requirements.txt"
            | "package.swift"
            | "pnpm-workspace.yaml"
            | "pnpm-workspace.yml"
    ) {
        return "manifest".to_string();
    }
    if inventory_ci_path(rel) {
        return "build_ci".to_string();
    }
    if inventory_runtime_config_path(rel, &name) {
        return "runtime_config".to_string();
    }
    if inventory_schema_path(rel, &ext) {
        return "schema_contract".to_string();
    }
    if inventory_migration_path(rel, &ext) {
        return "migration".to_string();
    }
    if ext == "md" {
        return "docs".to_string();
    }
    if repo::is_script_ext(&ext) || matches!(name.as_str(), "makefile" | "justfile") {
        return "script".to_string();
    }
    if repo::is_source_ext(&ext) {
        return "source".to_string();
    }
    if repo::is_asset_ext(&ext) {
        return "asset".to_string();
    }
    if matches!(ext.as_str(), "json" | "toml" | "yaml" | "yml") {
        return "config".to_string();
    }
    "file".to_string()
}
