fn validate_anchors(project: &crate::model::Project) -> AnchorValidation {
    let mut problems = project
        .config_errors
        .iter()
        .map(|error| format!("{}: {}", error.path, error.error))
        .collect::<Vec<_>>();
    problems.extend(semantic_anchor_problems(project));
    let warnings = semantic_anchor_warnings(project);
    let ok = problems.is_empty();
    let details = semantic_anchor_details(project, ok);
    AnchorValidation {
        kind: "anchor_validation",
        schema_version: "4",
        ok,
        config: project.config_path.clone(),
        summary: AnchorValidationSummary {
            domains: project.anchors.domain.iter().count() + project.anchors.domains.len(),
            concepts: project.anchors.concepts.len(),
            forbidden_boundaries: project.anchors.boundaries.forbidden.len(),
            verification_defaults: project.anchors.verification.default.len(),
        },
        problems,
        warnings,
        details,
    }
}

fn semantic_anchor_problems(project: &crate::model::Project) -> Vec<String> {
    let mut problems = Vec::new();
    if project.config_path.is_some() {
        match project.anchors.version {
            Some(1) => {}
            Some(version) => problems.push(format!(
                ".ctx.yml declares unsupported version `{version}`; expected `1`"
            )),
            None => problems.push(".ctx.yml is missing required `version: 1`".to_string()),
        }
    }
    if let Some(domain) = &project.anchors.domain
        && let Some(path) = &domain.path
    {
        validate_anchor_domain_path(
            project,
            domain.id.as_deref().unwrap_or("repo"),
            path,
            &mut problems,
        );
    }
    for (id, domain) in &project.anchors.domains {
        if let Some(path) = &domain.path {
            validate_anchor_domain_path(project, id, path, &mut problems);
        }
    }
    for (id, concept) in &project.anchors.concepts {
        if concept.files.is_empty() {
            problems.push(format!("concept `{id}` must declare at least one file"));
        }
        for file in &concept.files {
            let rel = map::resolve_anchor_path(project, file);
            if !is_glob_like(file) && !project.files.contains_key(&rel) {
                problems.push(format!("concept `{id}` declares missing file `{rel}`"));
            }
        }
    }
    for (idx, edge) in project.anchors.boundaries.forbidden.iter().enumerate() {
        let number = idx + 1;
        if edge.from.trim().is_empty() {
            problems.push(format!("forbidden boundary #{number} is missing `from`"));
        }
        if edge.to.trim().is_empty() {
            problems.push(format!("forbidden boundary #{number} is missing `to`"));
        }
        if edge.reason.trim().is_empty() {
            problems.push(format!("forbidden boundary #{number} is missing `reason`"));
        }
        if let Some(status) = &edge.status
            && !matches!(status.as_str(), "forbidden" | "warn" | "warning")
        {
            problems.push(format!(
                "forbidden boundary #{number} has unsupported status `{status}`"
            ));
        }
    }
    for (index, command) in project.anchors.verification.default.iter().enumerate() {
        if command.trim().is_empty() {
            problems.push(format!(
                "verification.default #{} is empty",
                index.saturating_add(1)
            ));
        }
    }
    problems
}

fn semantic_anchor_warnings(project: &crate::model::Project) -> Vec<String> {
    let mut warnings = Vec::new();
    if project.config_path.is_none() && project.config_errors.is_empty() {
        warnings
            .push("no .ctx.yml found; codemap will use zero-config structural maps".to_string());
        return warnings;
    }
    for (idx, edge) in project.anchors.boundaries.forbidden.iter().enumerate() {
        let number = idx + 1;
        if !edge.from.trim().is_empty() && !anchor_pattern_matches_project(project, &edge.from) {
            warnings.push(format!(
                "forbidden boundary #{number} `from` pattern `{}` matches no indexed files or packages",
                edge.from
            ));
        }
        if !edge.to.trim().is_empty() && !anchor_pattern_matches_project(project, &edge.to) {
            warnings.push(format!(
                "forbidden boundary #{number} `to` pattern `{}` matches no indexed files or packages",
                edge.to
            ));
        }
        if edge.recovery.is_empty() {
            warnings.push(format!(
                "forbidden boundary #{number} has no recovery steps; violation output will be less actionable"
            ));
        }
    }
    for (id, concept) in &project.anchors.concepts {
        for file in &concept.files {
            if is_glob_like(file) && !anchor_pattern_matches_project(project, file) {
                warnings.push(format!(
                    "concept `{id}` glob `{file}` matches no indexed files"
                ));
            }
        }
        if concept.invariants.is_empty() {
            warnings.push(format!(
                "concept `{id}` has no invariants; it can anchor files but not behavior"
            ));
        }
    }
    warnings
}

