fn root_runtime_containers(project: &Project) -> Vec<Surface> {
    let mut out = Vec::new();
    for package in &project.packages {
        if package.path == "." {
            continue;
        }
        let Some(manifest) = project.files.get(&package.manifest) else {
            continue;
        };
        let entrypoints = runtime_manifest_entrypoints(project, manifest);
        if entrypoints.is_empty() {
            continue;
        }
        let examples = entrypoints
            .iter()
            .flat_map(|surface| surface.examples.iter().cloned())
            .take(3)
            .collect::<Vec<_>>();
        out.push(surface(SurfaceFact {
            id: format!("surface:runtime_container:{}", package.path),
            kind: "runtime_container".to_string(),
            path: Some(package.path.clone()),
            role: Some("runtime_entrypoint".to_string()),
            evidence: "current_level_runtime_container".to_string(),
            strength: EvidenceStrength::High,
            count: Some(entrypoints.len()),
            examples,
            hidden_count: entrypoints.len().saturating_sub(3),
        }));
    }
    out.sort_by(|a, b| {
        a.path
            .as_deref()
            .unwrap_or_default()
            .cmp(b.path.as_deref().unwrap_or_default())
    });
    out
}

fn runtime_expand_commands(
    scope: &str,
    root_containers: &[Surface],
    entrypoints: &[Surface],
) -> Vec<String> {
    let mut expand = vec![
        format!("codemap cone {}", shell_quote(scope)),
        format!("codemap proof-map {}", shell_quote(scope)),
    ];
    if scope == "." {
        expand.extend(root_containers.iter().take(5).filter_map(|surface| {
            let path = surface.path.as_deref()?;
            Some(format!("codemap runtime {}", shell_quote(path)))
        }));
    } else {
        expand.extend(entrypoints.iter().take(3).filter_map(|surface| {
            if surface.kind != "cli_entrypoint" && surface.kind != "runtime_entrypoint" {
                return None;
            }
            let path = surface.path.as_deref()?;
            Some(format!("codemap flow {}", shell_quote(path)))
        }));
    }
    expand
}
