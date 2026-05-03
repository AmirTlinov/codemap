fn cone_declared_env(project: &Project, rel: &str) -> Vec<EnvDeclaration> {
    let Some(file) = project.files.get(rel) else {
        return Vec::new();
    };
    if !file.has_role("env_config") {
        return Vec::new();
    }
    env_declared_keys(project, rel)
        .into_iter()
        .map(|(key, line_start)| EnvDeclaration {
            key,
            path: rel.to_string(),
            line_start,
        })
        .collect()
}

fn owner_env_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let keys = env_declared_keys(project, rel);
    let key_set = keys
        .iter()
        .map(|(key, _)| key.clone())
        .collect::<BTreeSet<_>>();
    let mut edges = keys
        .into_iter()
        .map(|(key, line)| {
            structural_edge_with_locations(
                rel.to_string(),
                format!("env:{key}"),
                "declares_env",
                "env_file",
                EvidenceStrength::Hard,
                vec![EvidenceLocation::line(rel, line, "env_declaration")],
            )
        })
        .collect::<Vec<_>>();
    for file in project.files.values() {
        if file.rel == rel
            || file.has_role("generated")
            || file.has_role("fixture")
            || file.has_role("archive")
        {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(project.root.join(&file.rel)) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            let mut names = static_env_names(line);
            names.extend(prisma_env_names(line));
            names.sort();
            names.dedup();
            for name in names {
                if key_set.contains(&name) {
                    edges.push(structural_edge_with_locations(
                        rel.to_string(),
                        file.rel.clone(),
                        "env_consumer",
                        "static_env_reference",
                        EvidenceStrength::High,
                        vec![EvidenceLocation::line(
                            &file.rel,
                            index + 1,
                            "env_reference",
                        )],
                    ));
                }
            }
        }
    }
    edges
}

fn owner_env_unknowns(project: &Project, rel: &str) -> Vec<Unknown> {
    let keys = env_declared_keys(project, rel);
    if keys.is_empty() {
        return Vec::new();
    }
    let consumer_edges = owner_env_edges(project, rel);
    keys.into_iter()
        .filter(|(key, _)| {
            !consumer_edges.iter().any(|edge| {
                edge.edge_type == "env_consumer" && owner_edge_mentions_key(project, edge, key)
            })
        })
        .map(|(key, line)| {
            unknown(
                "env_consumer_not_found",
                Some(rel),
                Some(line),
                format!("no static reader found for env key `{key}`"),
                "runtime config key is declared but no deterministic consumer edge was found",
                Some(format!("codemap runtime {}", shell_quote(rel))),
            )
        })
        .collect()
}

fn owner_edge_mentions_key(project: &Project, edge: &StructuralEdge, key: &str) -> bool {
    edge.locations.iter().any(|location| {
        let Some(line) = location.line_start else {
            return false;
        };
        let Ok(text) = std::fs::read_to_string(project.root.join(&location.path)) else {
            return false;
        };
        text.lines()
            .nth(line.saturating_sub(1))
            .is_some_and(|line| line.contains(key))
    })
}
