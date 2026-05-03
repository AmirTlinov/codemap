fn current_level_owner_edges(
    project: &Project,
    rel: &str,
    _include_hidden: bool,
    endpoint_depth: usize,
) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    edges.extend(current_level_script_edges(project, rel, endpoint_depth));
    edges.extend(current_level_ci_edges(project, rel, endpoint_depth));
    edges.extend(current_level_workspace_edges(project, rel, endpoint_depth));
    edges.extend(current_level_manifest_lockfile_edges(
        project,
        rel,
        endpoint_depth,
    ));
    edges.extend(current_level_env_edges(project, rel, endpoint_depth));
    edges.extend(current_level_schema_edges(project, rel, endpoint_depth));
    edges
}

fn current_level_script_edges(
    project: &Project,
    rel: &str,
    endpoint_depth: usize,
) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    let scope_is_support = is_support_artifact_path(rel);
    for script in &project.scripts {
        let Some(path) = script.path.as_deref() else {
            continue;
        };
        if should_hide_owner_edge_path(path, scope_is_support) {
            continue;
        }
        if !path_under_scope(path, rel) {
            continue;
        }
        let script_id = script_target_for_path(path, &script.name);
        let from = directory_edge_endpoint_at_depth(project, rel, path, endpoint_depth);
        let line = script.line_start.unwrap_or(1);
        edges.push(structural_edge_with_locations(
            from,
            script_id.clone(),
            "declares_script",
            "script_manifest",
            EvidenceStrength::Hard,
            vec![EvidenceLocation::line(path, line, "script_manifest")],
        ));
        edges.push(structural_edge_with_locations(
            script_id,
            command_target(&script.command),
            "runs_command",
            "script_manifest",
            EvidenceStrength::Hard,
            vec![EvidenceLocation::line(path, line, "script_manifest")],
        ));
    }
    for package in &project.packages {
        if should_hide_owner_edge_path(&package.manifest, scope_is_support)
            || !path_under_scope(&package.manifest, rel)
            || package.ecosystem != "javascript"
            || package.path == "."
        {
            continue;
        }
        for (name, command, line) in package_json_scripts(project, &package.manifest)
            .into_iter()
            .filter(|(name, command, _)| manifest_script_is_proof_relevant(name, command))
        {
            let script_id = script_target_for_package(package, &name);
            let from =
                directory_edge_endpoint_at_depth(project, rel, &package.manifest, endpoint_depth);
            edges.push(structural_edge_with_locations(
                from,
                script_id.clone(),
                "declares_script",
                "package_script",
                EvidenceStrength::Hard,
                vec![EvidenceLocation::line(
                    &package.manifest,
                    line,
                    "package_script",
                )],
            ));
            if include_package_script_command_edge(rel) {
                edges.push(structural_edge_with_locations(
                    script_id,
                    command_target(&command),
                    "runs_command",
                    "package_script",
                    EvidenceStrength::Hard,
                    vec![EvidenceLocation::line(
                        &package.manifest,
                        line,
                        "package_script",
                    )],
                ));
            }
        }
    }
    edges
}

fn current_level_ci_edges(
    project: &Project,
    rel: &str,
    endpoint_depth: usize,
) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    let scripts = project.scripts.clone();
    let scope_is_support = is_support_artifact_path(rel);
    for ci in project.files.values().filter(|file| file.has_role("build_ci")) {
        if should_hide_owner_edge_path(&ci.rel, scope_is_support) {
            continue;
        }
        if !path_under_scope(&ci.rel, rel) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(project.root.join(&ci.rel)) else {
            continue;
        };
        let from = directory_edge_endpoint_at_depth(project, rel, &ci.rel, endpoint_depth);
        for step in ci_run_steps(&text) {
            for script in &scripts {
                if command_invokes_script_surface(&step.command, script) {
                    edges.push(structural_edge_with_locations(
                        from.clone(),
                        script_target_for_path(
                            script.path.as_deref().unwrap_or("script"),
                            &script.name,
                        ),
                        "ci_calls_script",
                        "ci_run_step",
                        EvidenceStrength::High,
                        vec![EvidenceLocation::line(&ci.rel, step.line, "ci_step")],
                    ));
                }
            }
            if validation_command_like(&step.command) {
                edges.push(structural_edge_with_locations(
                    from.clone(),
                    command_target(&step.command),
                    "ci_runs_command",
                    "ci_run_step",
                    EvidenceStrength::High,
                    vec![EvidenceLocation::line(&ci.rel, step.line, "ci_step")],
                ));
            }
        }
    }
    edges
}

fn current_level_workspace_edges(
    project: &Project,
    rel: &str,
    endpoint_depth: usize,
) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    let scope_is_support = is_support_artifact_path(rel);
    for file in files_under_directory(project, rel) {
        if should_hide_owner_edge_path(&file.rel, scope_is_support) {
            continue;
        }
        if !workspace_manifest_file(&file.rel) {
            continue;
        }
        edges.extend(
            owner_workspace_manifest_edges(project, &file.rel)
                .into_iter()
                .map(|edge| StructuralEdge {
                    from: directory_edge_endpoint_at_depth(
                        project,
                        rel,
                        &edge.from,
                        endpoint_depth,
                    ),
                    to: workspace_edge_directory_target(project, rel, &edge.to, endpoint_depth),
                    ..edge
                }),
        );
    }
    edges
}

fn current_level_manifest_lockfile_edges(
    project: &Project,
    rel: &str,
    endpoint_depth: usize,
) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    let scope_is_support = is_support_artifact_path(rel);
    for package in &project.packages {
        if should_hide_owner_edge_path(&package.manifest, scope_is_support) {
            continue;
        }
        if !path_under_scope(&package.manifest, rel) {
            continue;
        }
        for lockfile in lockfiles_for_package(project, package) {
            edges.push(structural_edge_with_locations(
                directory_edge_endpoint_at_depth(project, rel, &package.manifest, endpoint_depth),
                directory_edge_endpoint_at_depth(project, rel, &lockfile.rel, endpoint_depth),
                "uses_lockfile",
                "lockfile",
                EvidenceStrength::High,
                vec![
                    EvidenceLocation::path(&package.manifest, "package_manifest"),
                    EvidenceLocation::path(&lockfile.rel, "lockfile"),
                ],
            ));
        }
    }
    edges
}

fn current_level_env_edges(
    project: &Project,
    rel: &str,
    endpoint_depth: usize,
) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    let scope_is_support = is_support_artifact_path(rel);
    for file in files_under_directory(project, rel)
        .into_iter()
        .filter(|file| file.has_role("env_config"))
    {
        if should_hide_owner_edge_path(&file.rel, scope_is_support) {
            continue;
        }
        let keys = env_declared_keys(project, &file.rel);
        for (name, line) in keys {
            edges.push(structural_edge_with_locations(
                directory_edge_endpoint_at_depth(project, rel, &file.rel, endpoint_depth),
                format!("env:{name}"),
                "declares_env",
                "env_file",
                EvidenceStrength::Hard,
                vec![EvidenceLocation::line(&file.rel, line, "env_key")],
            ));
        }
    }
    edges
}

fn current_level_schema_edges(
    project: &Project,
    rel: &str,
    endpoint_depth: usize,
) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    let scope_is_support = is_support_artifact_path(rel);
    for file in files_under_directory(project, rel)
        .into_iter()
        .filter(|file| file.has_role("schema_contract") || schema_owner_path(&file.rel))
    {
        if should_hide_owner_edge_path(&file.rel, scope_is_support) {
            continue;
        }
        let owner = schema_owner_directory(&file.rel);
        let mut migrations = project
            .files
            .values()
            .filter(|candidate| candidate.has_role("migration"))
            .filter(|candidate| {
                candidate.rel.starts_with(&format!("{owner}/"))
                    || package_for_rel(project, &candidate.rel).map(|p| p.manifest.as_str())
                        == package_for_rel(project, &file.rel).map(|p| p.manifest.as_str())
            })
            .collect::<Vec<_>>();
        migrations.sort_by(|a, b| a.rel.cmp(&b.rel));
        for migration in migrations {
            edges.push(structural_edge_with_locations(
                directory_edge_endpoint_at_depth(project, rel, &file.rel, endpoint_depth),
                directory_edge_endpoint_at_depth(project, rel, &migration.rel, endpoint_depth),
                "schema_migration",
                "migration_path",
                EvidenceStrength::High,
                vec![EvidenceLocation::path(&migration.rel, "migration")],
            ));
        }
    }
    edges
}
