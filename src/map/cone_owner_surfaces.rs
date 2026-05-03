fn cone_owner_outgoing_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let Some(file) = project.files.get(rel) else {
        return Vec::new();
    };
    let mut edges = Vec::new();
    if file.has_role("manifest") {
        edges.extend(owner_manifest_edges(project, rel));
    }
    if file.has_role("schema_contract") || schema_owner_path(rel) {
        edges.extend(owner_schema_edges(project, rel));
    }
    if file.has_role("env_config") {
        edges.extend(owner_env_edges(project, rel));
    }
    if file.has_role("build_ci") {
        edges.extend(owner_ci_edges(project, rel));
    }
    sort_edges(&mut edges);
    edges
}

fn cone_owner_incoming_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let Some(file) = project.files.get(rel) else {
        return Vec::new();
    };
    let mut edges = Vec::new();
    if file.has_role("manifest") {
        edges.extend(owner_manifest_incoming_edges(project, rel));
    }
    if file.has_role("schema_contract") || schema_owner_path(rel) {
        edges.extend(owner_schema_incoming_edges(project, rel));
    }
    sort_edges(&mut edges);
    edges
}

fn cone_owner_proof_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let mut edges = owner_surface_proof_surfaces(project, rel)
        .into_iter()
        .map(|proof| {
            let from = proof.path.clone().unwrap_or_else(|| rel.to_string());
            let to = proof
                .command
                .clone()
                .or_else(|| proof.path.clone())
                .unwrap_or_else(|| rel.to_string());
            structural_edge_with_locations(
                from,
                to,
                "proof_surface",
                proof.evidence,
                proof.strength,
                proof.locations,
            )
        })
        .collect::<Vec<_>>();
    sort_edges(&mut edges);
    edges
}

fn cone_owner_unknowns(project: &Project, rel: &str) -> Vec<Unknown> {
    let Some(file) = project.files.get(rel) else {
        return Vec::new();
    };
    let mut unknowns = Vec::new();
    if file.has_role("env_config") {
        unknowns.extend(owner_env_unknowns(project, rel));
    }
    if file.has_role("manifest") && workspace_manifest_file(rel) {
        if workspace_manifest_member_packages(project, rel).is_empty() {
            unknowns.push(unknown(
                "workspace_members_not_found",
                Some(rel),
                None,
                "no workspace member package manifests matched this workspace manifest",
                "workspace manifest cone cannot show package membership edges",
                Some(format!("codemap ls {}", shell_quote(rel))),
            ));
        }
        if owner_surface_proof_surfaces(project, rel).is_empty() {
            unknowns.push(unknown(
                "workspace_proof_not_found",
                Some(rel),
                None,
                "no workspace root script or CI run step was found for this workspace manifest",
                "proof may fall back to broader repo commands",
                Some(format!("codemap proof {}", shell_quote(rel))),
            ));
        }
    }
    if (file.has_role("schema_contract") || schema_owner_path(rel))
        && owner_schema_edges(project, rel).is_empty()
        && owner_surface_proof_surfaces(project, rel).is_empty()
    {
        unknowns.push(unknown(
            "schema_owner_neighborhood_missing",
            Some(rel),
            None,
            "no migrations, schema scripts, CI references, or schema env links were found",
            "schema cone cannot show producer/proof rails for this owner surface",
            Some(format!("codemap proof-map {}", shell_quote(rel))),
        ));
    }
    unknowns
}

fn owner_manifest_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    let Some(package) = project.packages.iter().find(|package| package.manifest == rel) else {
        if workspace_manifest_file(rel) {
            edges.extend(owner_workspace_manifest_edges(project, rel));
        }
        return edges;
    };
    let package_line = first_line_containing(project, rel, &["\"name\"", "name ="]).unwrap_or(1);
    edges.push(structural_edge_with_locations(
        rel.to_string(),
        format!("package:{}", package.name),
        "declares_package",
        "package_manifest",
        EvidenceStrength::Hard,
        vec![EvidenceLocation::line(rel, package_line, "package_manifest")],
    ));
    for script in project
        .scripts
        .iter()
        .filter(|script| script.path.as_deref() == Some(rel))
        .filter(|_| manifest_file_name(rel) != "package.json")
    {
        let line = script.line_start.unwrap_or(1);
        let script_id = script_target_for_path(rel, &script.name);
        edges.push(structural_edge_with_locations(
            rel.to_string(),
            script_id.clone(),
            "declares_script",
            "script_manifest",
            EvidenceStrength::Hard,
            vec![EvidenceLocation::line(rel, line, "script_manifest")],
        ));
        edges.push(structural_edge_with_locations(
            script_id,
            command_target(&script.command),
            "runs_command",
            "script_manifest",
            EvidenceStrength::Hard,
            vec![EvidenceLocation::line(rel, line, "script_manifest")],
        ));
    }
    for lockfile in lockfiles_for_package(project, package) {
        edges.push(structural_edge_with_locations(
            rel.to_string(),
            lockfile.rel.clone(),
            "uses_lockfile",
            "lockfile",
            EvidenceStrength::High,
            vec![
                EvidenceLocation::path(rel, "package_manifest"),
                EvidenceLocation::path(&lockfile.rel, "lockfile"),
            ],
        ));
    }
    for (name, command, line) in package_json_scripts(project, rel) {
        let script_id = format!("script:{name}");
        edges.push(structural_edge_with_locations(
            rel.to_string(),
            script_id.clone(),
            "declares_script",
            "manifest_script",
            EvidenceStrength::Hard,
            vec![EvidenceLocation::line(rel, line, "package_script")],
        ));
        edges.push(structural_edge_with_locations(
            script_id,
            command_target(&command),
            "runs_command",
            "manifest_script",
            EvidenceStrength::Hard,
            vec![EvidenceLocation::line(rel, line, "package_script")],
        ));
    }
    for edge in project
        .package_edges
        .iter()
        .filter(|edge| edge.from_manifest == rel)
    {
        let target = edge.to_manifest.as_deref().unwrap_or(edge.to.as_str());
        edges.push(structural_edge_with_locations(
            rel.to_string(),
            target.to_string(),
            "package_dependency",
            edge.source.clone(),
            EvidenceStrength::Hard,
            vec![EvidenceLocation::line(
                rel,
                owner_line_containing(project, rel, &[&format!("\"{}\"", edge.dependency), &edge.dependency]),
                "package_dependency",
            )],
        ));
    }
    for target in package_public_targets(project, package) {
        edges.push(structural_edge_with_locations(
            rel.to_string(),
            target,
            "package_export",
            "package_manifest_export",
            EvidenceStrength::Hard,
            vec![EvidenceLocation::line(
                rel,
                first_line_containing(project, rel, &["\"exports\"", "\"bin\""]).unwrap_or(1),
                "package_export",
            )],
        ));
    }
    edges
}

fn owner_manifest_incoming_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    project
        .package_edges
        .iter()
        .filter(|edge| edge.to_manifest.as_deref() == Some(rel))
        .map(|edge| {
            structural_edge_with_locations(
                edge.from_manifest.clone(),
                rel.to_string(),
                "package_consumer",
                edge.source.clone(),
                EvidenceStrength::Hard,
                vec![EvidenceLocation::line(
                    &edge.from_manifest,
                    owner_line_containing(
                        project,
                        &edge.from_manifest,
                        &[&format!("\"{}\"", edge.dependency), &edge.dependency],
                    ),
                    "package_dependency",
                )],
            )
        })
        .collect()
}

fn owner_schema_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    edges.extend(owner_schema_env_edges(project, rel));
    edges.extend(owner_schema_migration_edges(project, rel));
    edges.extend(owner_prisma_generator_edges(project, rel));
    edges
}

fn owner_schema_env_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let Ok(text) = std::fs::read_to_string(project.root.join(rel)) else {
        return Vec::new();
    };
    let mut edges = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if !line.contains("env(") {
            continue;
        }
        for name in prisma_env_names(line) {
            edges.push(structural_edge_with_locations(
                rel.to_string(),
                format!("env:{name}"),
                "reads_env",
                "prisma_env",
                EvidenceStrength::Hard,
                vec![EvidenceLocation::line(rel, index + 1, "env_reference")],
            ));
        }
    }
    edges
}

fn owner_schema_migration_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let path = Path::new(rel);
    let mut edges = Vec::new();
    if path.file_name().and_then(|name| name.to_str()) == Some("schema.prisma") {
        let migration_prefix = path
            .parent()
            .map(|parent| parent.join("migrations"))
            .and_then(|path| path.to_str().map(repo::normalize_rel_path));
        if let Some(prefix) = migration_prefix {
            for file in project.files.values() {
                if file.rel.starts_with(&format!("{}/", prefix.trim_end_matches('/'))) {
                    edges.push(edge_with_path_location(
                        rel.to_string(),
                        file.rel.clone(),
                        "schema_migration",
                        "prisma_migration_file",
                        EvidenceStrength::High,
                        file.rel.clone(),
                        "migration_file",
                    ));
                }
            }
        }
    } else if rel.contains("/migrations/")
        && let Some(schema) = path
            .parent()
            .and_then(|parent| parent.parent())
            .map(|parent| parent.join("schema.prisma"))
            .and_then(|path| path.to_str().map(repo::normalize_rel_path))
        && project.files.contains_key(&schema)
    {
        edges.push(edge_with_path_location(
            rel.to_string(),
            schema,
            "migration_schema_owner",
            "prisma_schema_file",
            EvidenceStrength::High,
            rel.to_string(),
            "migration_file",
        ));
    }
    edges
}

fn owner_prisma_generator_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let Ok(text) = std::fs::read_to_string(project.root.join(rel)) else {
        return Vec::new();
    };
    text.lines()
        .enumerate()
        .filter(|(_, line)| line.contains("prisma-client-js"))
        .map(|(index, _)| {
            structural_edge_with_locations(
                rel.to_string(),
                "generated:@prisma/client".to_string(),
                "generates_client",
                "prisma_generator",
                EvidenceStrength::High,
                vec![EvidenceLocation::line(rel, index + 1, "prisma_generator")],
            )
        })
        .collect()
}

fn owner_schema_incoming_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let Some(owner_package) = package_for_rel(project, rel) else {
        return Vec::new();
    };
    project
        .files
        .values()
        .filter(|file| file.rel != rel && repo::is_source_ext(&file.ext))
        .filter(|file| file.imports.contains("@prisma/client"))
        .filter(|file| {
            package_for_rel(project, &file.rel)
                .is_some_and(|package| package.path == owner_package.path)
        })
        .map(|file| {
            structural_edge_with_locations(
                file.rel.clone(),
                rel.to_string(),
                "schema_client_consumer",
                "prisma_client_import",
                EvidenceStrength::High,
                first_line_locations_containing(
                    project,
                    &file.rel,
                    &["@prisma/client", "PrismaClient"],
                    "prisma_client_import",
                ),
            )
        })
        .collect()
}

fn owner_ci_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let Ok(text) = std::fs::read_to_string(project.root.join(rel)) else {
        return Vec::new();
    };
    ci_run_steps(&text)
        .into_iter()
        .filter_map(|step| {
            let kind = ci_owner_step_kind(&step.command)?;
            Some(structural_edge_with_locations(
                rel.to_string(),
                step.command,
                kind.edge_type(),
                kind.evidence(),
                EvidenceStrength::Hard,
                vec![EvidenceLocation::line(rel, step.line, "ci_step")],
            ))
        })
        .collect()
}

fn owner_line_containing(project: &Project, rel: &str, needles: &[&str]) -> usize {
    first_line_containing(project, rel, needles).unwrap_or(1)
}

fn first_line_locations_containing(
    project: &Project,
    rel: &str,
    needles: &[&str],
    kind: &str,
) -> Vec<EvidenceLocation> {
    vec![EvidenceLocation::line(
        rel,
        owner_line_containing(project, rel, needles),
        kind,
    )]
}
