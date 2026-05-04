#[derive(Default)]
struct LineDelta {
    added: Vec<(usize, String)>,
    removed: Vec<(usize, String)>,
}

fn git_unified_zero_delta(project: &Project, rel: &str, mode: &DiffMapMode) -> LineDelta {
    if matches!(mode, DiffMapMode::WorkingTree) && git_file_is_untracked(project, rel) {
        return file_as_added_delta(project, rel);
    }
    let mut command = std::process::Command::new("git");
    command.arg("-C").arg(&project.root).arg("diff");
    match mode {
        DiffMapMode::WorkingTree => {
            command.arg("HEAD");
        }
        DiffMapMode::Staged => {
            command.arg("--cached");
        }
        DiffMapMode::Since(base) => {
            command.arg(base);
        }
    }
    let Ok(output) = command.args(["--unified=0", "--"]).arg(rel).output() else {
        return LineDelta::default();
    };
    if !output.status.success() {
        return LineDelta::default();
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut delta = LineDelta::default();
    let mut old_line = 0usize;
    let mut new_line = 0usize;
    for line in text.lines() {
        if let Some((old_start, new_start)) = parse_diff_hunk_header(line) {
            old_line = old_start;
            new_line = new_start;
            continue;
        }
        if line.starts_with("+++") || line.starts_with("---") {
            continue;
        }
        if let Some(body) = line.strip_prefix('+') {
            delta.added.push((new_line.max(1), body.to_string()));
            new_line += 1;
        } else if let Some(body) = line.strip_prefix('-') {
            delta.removed.push((old_line.max(1), body.to_string()));
            old_line += 1;
        } else if !line.starts_with("diff ") && !line.starts_with("index ") {
            old_line += 1;
            new_line += 1;
        }
    }
    delta
}

fn diff_current_runtime_code(
    project: &Project,
    rel: &str,
    mode: &DiffMapMode,
) -> BTreeMap<usize, String> {
    let text = match mode {
        DiffMapMode::Staged => git_show_file(project, ":", rel),
        DiffMapMode::WorkingTree | DiffMapMode::Since(_) => {
            std::fs::read_to_string(project.root.join(rel)).ok()
        }
    };
    text.as_deref()
        .map(runtime_code_line_lookup)
        .unwrap_or_default()
}

fn diff_base_runtime_code(
    project: &Project,
    rel: &str,
    mode: &DiffMapMode,
) -> BTreeMap<usize, String> {
    let revision = match mode {
        DiffMapMode::WorkingTree | DiffMapMode::Staged => "HEAD",
        DiffMapMode::Since(base) => base.as_str(),
    };
    git_show_file(project, revision, rel)
        .as_deref()
        .map(runtime_code_line_lookup)
        .unwrap_or_default()
}

fn git_show_file(project: &Project, revision: &str, rel: &str) -> Option<String> {
    let object = if revision == ":" {
        format!(":{rel}")
    } else {
        format!("{revision}:{rel}")
    };
    let exists = std::process::Command::new("git")
        .arg("-C")
        .arg(&project.root)
        .arg("cat-file")
        .arg("-e")
        .arg(&object)
        .output()
        .ok()?;
    if !exists.status.success() {
        return None;
    }
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(&project.root)
        .arg("show")
        .arg(object)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).to_string())
}

fn git_file_is_untracked(project: &Project, rel: &str) -> bool {
    if !project.root.join(rel).is_file() {
        return false;
    }
    let Ok(output) = std::process::Command::new("git")
        .arg("-C")
        .arg(&project.root)
        .args(["ls-files", "--error-unmatch", "--"])
        .arg(rel)
        .output()
    else {
        return false;
    };
    !output.status.success()
}

fn file_as_added_delta(project: &Project, rel: &str) -> LineDelta {
    let Ok(text) = std::fs::read_to_string(project.root.join(rel)) else {
        return LineDelta::default();
    };
    LineDelta {
        added: text
            .lines()
            .enumerate()
            .map(|(index, line)| (index + 1, line.to_string()))
            .collect(),
        removed: Vec::new(),
    }
}

fn parse_diff_hunk_header(line: &str) -> Option<(usize, usize)> {
    if !line.starts_with("@@ ") {
        return None;
    }
    let mut parts = line.split_whitespace();
    parts.next()?;
    let old = parts.next()?.trim_start_matches('-');
    let new = parts.next()?.trim_start_matches('+');
    Some((
        old.split(',').next()?.parse().ok()?,
        new.split(',').next()?.parse().ok()?,
    ))
}

fn structural_line_target(line: &str) -> Option<String> {
    let trimmed = line.trim();
    for quote in ['"', '\''] {
        if let Some(start) = trimmed.find(quote)
            && let Some(end) = trimmed[start + 1..].find(quote)
        {
            return Some(trimmed[start + 1..start + 1 + end].to_string());
        }
    }
    if let Some(rest) = trimmed.strip_prefix("from ") {
        let target = rest
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim_matches(['(', ')', ',', ';']);
        if !target.is_empty() && target != "import" {
            return Some(target.to_string());
        }
    }
    if let Some(rest) = trimmed.strip_prefix("import ") {
        let target = rest
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim_matches(['(', ')', ',', ';']);
        if !target.is_empty() && !target.starts_with('{') && target != "type" {
            return Some(target.to_string());
        }
    }
    if let Some(rest) = trimmed
        .strip_prefix("pub mod ")
        .or_else(|| trimmed.strip_prefix("mod "))
    {
        let target = rest.trim().trim_end_matches(';');
        if !target.is_empty() {
            return Some(target.to_string());
        }
    }
    if let Some(rest) = trimmed.strip_prefix("use ") {
        let target = rest.trim().trim_end_matches(';');
        if !target.is_empty() && !target.starts_with('{') {
            return Some(target.to_string());
        }
    }
    None
}

fn truncate_with_hidden<T>(
    values: &mut Vec<T>,
    limit: usize,
    hidden: &mut Vec<HiddenGroup>,
    reason: &str,
    expand: &str,
) {
    if values.len() <= limit {
        return;
    }
    hidden.push(HiddenGroup {
        reason: reason.to_string(),
        count: values.len() - limit,
        expand: expand_with_concrete_limit(expand, values.len()),
    });
    values.truncate(limit);
}

fn expand_with_concrete_limit(expand: &str, next_limit: usize) -> String {
    let next_limit = next_limit.max(1);
    if expand.contains("<larger-number>") {
        return expand.replace("<larger-number>", &next_limit.to_string());
    }
    if expand.split_whitespace().any(|part| part == "--limit") {
        return expand.to_string();
    }
    format!("{expand} --limit {next_limit}")
}

fn runtime_entrypoint_kind(file: &FileInfo) -> Option<&'static str> {
    let name = Path::new(&file.rel).file_name().and_then(|name| name.to_str())?;
    if matches!(name, "main.rs" | "main.go" | "__main__.py")
        || file.rel.ends_with("/main.py")
        || file.rel.ends_with("/app.py")
    {
        Some("runtime_entrypoint")
    } else {
        None
    }
}

fn routes_from_file_convention(project: &Project, file: &FileInfo) -> Vec<RuntimeRoute> {
    let rel = file.rel.as_str();
    let Some(route) = (if let Some(rest) = next_app_route_rest(rel) {
        next_app_route(rest)
    } else if let Some(rest) = next_pages_route_rest(rel) {
        next_pages_route(rest)
    } else {
        None
    }) else {
        return Vec::new();
    };
    if !project.files.contains_key(rel) {
        return Vec::new();
    }
    let methods = next_route_method_handlers(file);
    if methods.is_empty() {
        return vec![RuntimeRoute {
            method: if rel.ends_with("/route.ts") || rel.ends_with("/route.js") {
                Some("ANY".to_string())
            } else {
                Some("GET".to_string())
            },
            path: route,
            file: rel.to_string(),
            handler_symbol: None,
            evidence: "file_route_convention".to_string(),
            strength: EvidenceStrength::High,
            locations: vec![EvidenceLocation::path(rel, "route_file")],
        }];
    }
    methods
        .into_iter()
        .map(|method| RuntimeRoute {
            method: Some(method.clone()),
            path: route.clone(),
            file: rel.to_string(),
            handler_symbol: Some(method),
            evidence: "file_route_convention".to_string(),
            strength: EvidenceStrength::High,
            locations: vec![EvidenceLocation::path(rel, "route_file")],
        })
        .collect()
}

fn next_route_method_handlers(file: &FileInfo) -> Vec<String> {
    if !file.rel.ends_with("/route.ts")
        && !file.rel.ends_with("/route.js")
        && !file.rel.ends_with("/route.tsx")
        && !file.rel.ends_with("/route.jsx")
    {
        return Vec::new();
    }
    let mut methods = file
        .symbols
        .iter()
        .filter(|symbol| symbol.exported && next_route_method_symbol(&symbol.name))
        .map(|symbol| symbol.name.clone())
        .collect::<Vec<_>>();
    methods.sort();
    methods
}

fn next_route_method_symbol(name: &str) -> bool {
    matches!(
        name,
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS"
    )
}

fn next_app_route(rest: &str) -> Option<String> {
    let route = rest
        .strip_suffix("/page.tsx")
        .or_else(|| rest.strip_suffix("/page.jsx"))
        .or_else(|| rest.strip_suffix("/page.ts"))
        .or_else(|| rest.strip_suffix("/route.ts"))
        .or_else(|| rest.strip_suffix("/route.js"))?;
    Some(route_path(route))
}

fn next_pages_route(rest: &str) -> Option<String> {
    let route = rest
        .strip_suffix(".tsx")
        .or_else(|| rest.strip_suffix(".jsx"))
        .or_else(|| rest.strip_suffix(".ts"))
        .or_else(|| rest.strip_suffix(".js"))?;
    Some(route_path(route.trim_end_matches("/index")))
}

fn route_path(value: &str) -> String {
    let value = value.trim_matches('/');
    if value.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", value.replace("[...", ":").replace('[', ":").replace(']', ""))
    }
}

fn env_surfaces_for_file(project: &Project, file: &FileInfo) -> Vec<EnvSurface> {
    let Ok(text) = std::fs::read_to_string(project.root.join(&file.rel)) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (line_number, line) in runtime_code_lines(&text) {
        for name in static_env_names(&line) {
            out.push(EnvSurface {
                name,
                used_by: file.rel.clone(),
                declaration: env_declaration(project, &file.rel),
                evidence: "static_env_reference".to_string(),
                strength: EvidenceStrength::High,
                locations: vec![EvidenceLocation::line(
                    &file.rel,
                    line_number,
                    "env_reference",
                )],
            });
        }
    }
    out
}

fn proof_missing_should_surface(project: &Project, seed: &str) -> bool {
    project
        .files
        .get(seed)
        .map(|file| {
            file.has_role("public_boundary")
                || file.has_role("schema_contract")
                || file.has_role("runtime_state")
                || runtime_entrypoint_kind(file).is_some()
        })
        .unwrap_or(false)
}

fn package_export_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    project
        .packages
        .iter()
        .filter(|package| {
            package.manifest == rel
                || package.path == rel
                || package_public_targets(project, package)
                    .into_iter()
                    .any(|target| target == rel)
        })
        .map(|package| {
            edge_with_path_location(
                package.manifest.clone(),
                rel.to_string(),
                "package_export",
                "package_manifest",
                EvidenceStrength::Hard,
                package.manifest.clone(),
                "package_manifest",
            )
        })
        .collect()
}

fn runtime_reference_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let runtime_facts = runtime_fact_index(project);
    runtime_facts
        .routes_for_file(rel)
        .into_iter()
        .flat_map(|route| route_reference_edges_with_index(project, &route, &runtime_facts))
        .collect()
}

fn file_matches_place_kind(file: &FileInfo, kind: &str) -> bool {
    match kind {
        "route" => route_from_path(&file.rel),
        "service" => file.rel.contains("service") || file.rel.contains("services/"),
        "component" => file.symbols.iter().any(|symbol| symbol.kind == "component"),
        "test" => file.has_role("test") && !file.has_role("test_support"),
        "contract" => file.has_role("schema_contract") || file.has_role("public_boundary"),
        "lens" => repo::is_source_ext(&file.ext) && file.rel.split('/').any(|part| part == "lenses"),
        other => file_kind_for_ls(file) == other,
    }
}

fn route_from_path(rel: &str) -> bool {
    rel.contains("/routes/")
        || rel.ends_with("/route.ts")
        || rel.ends_with("/route.js")
        || next_app_route_path(rel)
        || pages_route_path(rel)
}

fn next_app_route_path(rel: &str) -> bool {
    next_app_route_rest(rel).is_some_and(|rest| {
        rest.ends_with("/page.tsx")
            || rest.ends_with("/page.jsx")
            || rest.ends_with("/page.ts")
            || rest.ends_with("/route.ts")
            || rest.ends_with("/route.js")
    })
}

fn pages_route_path(rel: &str) -> bool {
    next_pages_route_rest(rel).is_some_and(|rest| {
        matches!(
            Path::new(rest).extension().and_then(|ext| ext.to_str()),
            Some("tsx" | "jsx" | "ts" | "js")
        )
    })
}

fn placement_conventions(scope: &str, kind: &str, surfaces: &[Surface]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(surface) = surfaces.first() {
        if surface.evidence == "proof_sensor_for_scope" {
            out.push(format!(
                "{kind} proof sensors already reference `{scope}` with {} example(s)",
                surface.count.unwrap_or(surface.examples.len())
            ));
        } else {
            out.push(format!(
                "{kind} surfaces already exist under `{scope}` with {} example(s)",
                surface.count.unwrap_or(surface.examples.len())
            ));
        }
        if let Some(example) = surface.examples.first()
            && let Some(parent) = Path::new(example).parent()
        {
            let parent = repo::normalize_rel_path(&parent.to_string_lossy());
            if surface.evidence == "proof_sensor_for_scope" {
                out.push(format!("observed {kind} proof parent: `{parent}`"));
            } else {
                out.push(format!("existing {kind} parent: `{parent}`"));
            }
        }
    }
    out
}
