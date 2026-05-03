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
