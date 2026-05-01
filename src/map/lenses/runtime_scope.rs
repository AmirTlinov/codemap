fn runtime_scope_files<'a>(
    project: &'a Project,
    scope: &str,
    include_hidden: bool,
) -> (Vec<&'a FileInfo>, usize) {
    if let Some(file) = project.files.get(scope) {
        return (vec![file], 0);
    }
    let all = files_under_directory(project, scope);
    if scope != "." || include_hidden {
        return (all, 0);
    }

    let mut seen = BTreeSet::new();
    let mut current_level = Vec::new();
    for file in direct_files_under_directory(project, ".")
        .into_iter()
        .chain(files_under_directory(project, ".github"))
    {
        if seen.insert(file.rel.clone()) {
            current_level.push(file);
        }
    }
    let hidden = all.len().saturating_sub(current_level.len());
    (current_level, hidden)
}

fn dedupe_runtime_entrypoints(values: Vec<Surface>) -> Vec<Surface> {
    let manifest_targets = values
        .iter()
        .filter(|surface| surface.kind == "cli_entrypoint")
        .filter_map(|surface| surface.path.clone())
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for surface in values {
        if surface.kind == "runtime_entrypoint"
            && surface
                .path
                .as_ref()
                .is_some_and(|path| manifest_targets.contains(path))
        {
            continue;
        }
        let key = (
            surface.kind.clone(),
            surface.path.clone().unwrap_or_default(),
            surface.evidence.clone(),
            surface.examples.join("\n"),
        );
        if seen.insert(key) {
            out.push(surface);
        }
    }
    out
}
