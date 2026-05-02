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
        .map(|f| impact_level_for_file(project, f).0)
        .max()
        .unwrap_or(Risk::Low);

    let mut minimal = project.anchors.verification.default.clone();
    if minimal.is_empty() {
        minimal = infer_minimal_commands(project, &domains, &all_files, changed);
    }
    let mut supplemental = Vec::new();
    if matches!(max_risk, Risk::MediumHigh | Risk::High | Risk::Critical)
        && let Some(typecheck) = find_script(project, &["typecheck", "tsc", "check"])
    {
        supplemental.push(typecheck);
    }
    if matches!(max_risk, Risk::High | Risk::Critical) {
        supplemental.push("codemap boundaries --changed".to_string());
    }
    let mut full = Vec::new();
    if matches!(max_risk, Risk::Critical)
        && let Some(test) = find_script(project, &["test"])
    {
        full.push(test);
    }
    VerificationPlan {
        minimal: unique(minimal).into_iter().take(3).collect(),
        supplemental: unique(supplemental).into_iter().take(3).collect(),
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

fn impact_level_for_file(project: &Project, rel: &str) -> (Risk, Vec<String>) {
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
    (risk, unique(reasons))
}
