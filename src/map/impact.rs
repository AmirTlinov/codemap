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
    let mut proof = dedupe_impact_proof_edges(
        cone_proof_edges_with_direct_consumers(project, &proof_seeds),
        rel,
    );
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

fn dedupe_impact_proof_edges(edges: Vec<StructuralEdge>, changed_rel: &str) -> Vec<StructuralEdge> {
    let mut seen = BTreeMap::new();
    let mut out: Vec<StructuralEdge> = Vec::new();
    for edge in edges {
        let key = (edge.from.clone(), edge.edge_type.clone());
        if let Some(index) = seen.get(&key).copied() {
            if impact_proof_precedence(&edge, changed_rel)
                > impact_proof_precedence(&out[index], changed_rel)
            {
                out[index] = edge;
            }
        } else {
            seen.insert(key, out.len());
            out.push(edge);
        }
    }
    sort_edges(&mut out);
    out
}

fn impact_proof_precedence(
    edge: &StructuralEdge,
    changed_rel: &str,
) -> (bool, EvidenceStrength, usize) {
    (
        edge.to == changed_rel,
        edge.strength,
        proof_evidence_precedence(&edge.evidence),
    )
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
        .map(|importer| {
            import_edge(
                project,
                importer.clone(),
                rel.to_string(),
                "direct_consumer",
                "reverse_import",
                EvidenceStrength::High,
            )
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
        .map(|dependency| {
            import_edge(
                project,
                rel.to_string(),
                dependency.clone(),
                "direct_dependency",
                "resolved_import",
                EvidenceStrength::High,
            )
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
        .map(|file| {
            let name = names.iter().next().map(String::as_str).unwrap_or(rel);
            structural_edge_with_locations(
                file.rel.clone(),
                rel.to_string(),
                "direct_consumer",
                "same_package_symbol_reference",
                EvidenceStrength::High,
                first_identifier_reference_location(
                    project,
                    &file.rel,
                    name,
                    "symbol_reference",
                ),
            )
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
            edges.push(structural_edge_with_locations(
                edge.from.clone(),
                rel.to_string(),
                "cross_boundary_consumer",
                "reverse_import_cross_boundary",
                EvidenceStrength::High,
                edge.locations.clone(),
            ));
        }
    }
    let package_seeds = package_consumer_seeds_for_impact(project, rel, direct_consumers);
    for manifest in package_consumer_manifests(project, &package_seeds, depth.max(1), usize::MAX) {
        edges.push(edge_with_path_location(
            manifest.clone(),
            rel.to_string(),
            "package_consumer",
            "package_manifest_reverse_dependency",
            EvidenceStrength::High,
            manifest,
            "package_manifest",
        ));
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
            edges.push(edge_with_path_location(
                rel.to_string(),
                rel.to_string(),
                "contract_changed",
                evidence,
                EvidenceStrength::High,
                rel.to_string(),
                "contract_file",
            ));
        }
        for target in &file.resolved_imports {
            if let Some(target_file) = project.files.get(target)
                && let Some(evidence) = contract_evidence(target_file)
            {
                edges.push(import_edge(
                    project,
                    rel.to_string(),
                    target.clone(),
                    "contract_dependency",
                    evidence,
                    EvidenceStrength::High,
                ));
            }
        }
    }
    for consumer in direct_consumers {
        if let Some(consumer_file) = project.files.get(&consumer.from)
            && let Some(evidence) = contract_evidence(consumer_file)
        {
            edges.push(structural_edge_with_locations(
                consumer.from.clone(),
                rel.to_string(),
                "contract_consumer",
                evidence,
                EvidenceStrength::High,
                consumer.locations.clone(),
            ));
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
