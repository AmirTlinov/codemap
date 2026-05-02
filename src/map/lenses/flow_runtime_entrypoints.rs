fn runtime_entrypoint_surface_for_file(project: &Project, rel: &str) -> Option<Surface> {
    let mut surfaces = Vec::new();
    for package in &project.packages {
        let Some(manifest) = project.files.get(&package.manifest) else {
            continue;
        };
        surfaces.extend(runtime_manifest_entrypoints(project, manifest));
    }
    surfaces
        .into_iter()
        .find(|surface| surface.path.as_deref() == Some(rel))
}

fn runtime_entrypoint_symbol_step(project: &Project, file: &FileInfo) -> Option<FlowStep> {
    let symbol = file.symbols.iter().find(|symbol| {
        file.language == "rust"
            && symbol.name == "main"
            && symbol.kind == "function"
            && symbol_is_top_level(project, file, symbol)
    })?;
    Some(FlowStep {
        index: 0,
        anchor: format!("{}#main", file.rel),
        kind: "entry_symbol".to_string(),
        evidence: "rust_main_symbol".to_string(),
        locations: vec![EvidenceLocation {
            path: file.rel.clone(),
            line_start: Some(symbol.line_start),
            line_end: Some(symbol.line_end),
            kind: "entry_symbol".to_string(),
        }],
    })
}

fn runtime_entrypoint_locations(
    project: &Project,
    rel: &str,
    surface: &Surface,
) -> Vec<EvidenceLocation> {
    let Some((manifest, command)) = cli_entrypoint_manifest_and_command(surface) else {
        return vec![EvidenceLocation::path(rel, "runtime_entrypoint")];
    };
    if surface.evidence == "cargo_bin_target"
        && let Some(line) = cargo_bin_target_line(project, manifest, rel, command)
    {
        return vec![EvidenceLocation::line(
            manifest,
            line,
            "cargo_bin_target",
        )];
    }
    vec![EvidenceLocation::path(rel, "runtime_entrypoint")]
}

fn cli_entrypoint_manifest_and_command(surface: &Surface) -> Option<(&str, &str)> {
    let rest = surface.id.strip_prefix("surface:cli_entrypoint:")?;
    rest.rsplit_once(':')
}

fn cargo_bin_target_line(
    project: &Project,
    manifest: &str,
    rel: &str,
    command: &str,
) -> Option<usize> {
    let package_path = Path::new(manifest)
        .parent()
        .map(|parent| repo::normalize_rel_path(&parent.to_string_lossy()))
        .filter(|parent| !parent.is_empty())
        .unwrap_or_else(|| ".".to_string());
    let target = rel
        .strip_prefix(&format!("{}/", package_path.trim_end_matches('/')))
        .unwrap_or(rel);
    cargo_bin_table_path_line(project, manifest, command, target)
}

fn cargo_bin_table_path_line(
    project: &Project,
    manifest: &str,
    command: &str,
    target: &str,
) -> Option<usize> {
    let text = std::fs::read_to_string(project.root.join(manifest)).ok()?;
    let mut current = CargoBinTable::default();
    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if let Some(path_line) = current.matching_path_line(command, target) {
                return Some(path_line);
            }
            current = CargoBinTable {
                in_bin: trimmed == "[[bin]]",
                ..CargoBinTable::default()
            };
            continue;
        }
        if !current.in_bin || trimmed.starts_with('#') {
            continue;
        }
        if let Some(value) = toml_string_assignment(trimmed, "name") {
            current.name = Some(value.to_string());
        } else if let Some(value) = toml_string_assignment(trimmed, "path") {
            current.path = Some(repo::normalize_rel_path(value.trim_start_matches("./")));
            current.path_line = Some(line_number);
        }
    }
    current.matching_path_line(command, target)
}

#[derive(Default)]
struct CargoBinTable {
    in_bin: bool,
    name: Option<String>,
    path: Option<String>,
    path_line: Option<usize>,
}

impl CargoBinTable {
    fn matching_path_line(&self, command: &str, target: &str) -> Option<usize> {
        (self.name.as_deref() == Some(command)
            && self.path.as_deref() == Some(repo::normalize_rel_path(target).as_str()))
        .then_some(self.path_line?)
    }
}

fn toml_string_assignment<'a>(trimmed: &'a str, key: &str) -> Option<&'a str> {
    let (left, right) = trimmed.split_once('=')?;
    if left.trim() != key {
        return None;
    }
    let right = right.trim_start();
    let quote = right.chars().next().filter(|ch| *ch == '"' || *ch == '\'')?;
    let value = &right[quote.len_utf8()..];
    let end = value.find(quote)?;
    Some(&value[..end])
}
