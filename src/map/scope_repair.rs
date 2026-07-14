fn nearest_proof_scope(project: &Project, scope: &str) -> Option<String> {
    let normalized = repo::normalize_rel_path(scope);
    let start = if project.files.contains_key(&normalized) {
        parent_scope(&normalized)
    } else {
        Some(normalized.clone())
    }?;
    ancestor_scopes(&start)
        .into_iter()
        .filter(|candidate| candidate != &normalized)
        .find(|candidate| proof_scope_has_sensors(project, candidate))
}

fn nearest_proof_scope_unknown(scope: &str, nearest: &str, expand: String) -> Unknown {
    unknown(
        "nearest_proof_scope",
        Some(scope),
        None,
        "no direct linked verification sensors found at this exact scope",
        format!("nearest parent proof scope is `{nearest}`; expand there to inspect broader sensors"),
        Some(expand),
    )
}

fn proof_scope_has_sensors(project: &Project, scope: &str) -> bool {
    files_under_directory(project, scope)
        .into_iter()
        .any(|file| file.has_role("test") && !file.has_role("test_support"))
        || !proof_surfaces_for_directory(project, scope, 1, 1).is_empty()
}

fn ancestor_scopes(scope: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = repo::normalize_rel_path(scope);
    loop {
        if current.is_empty() {
            current = ".".to_string();
        }
        out.push(current.clone());
        if current == "." {
            break;
        }
        let Some(parent) = parent_scope(&current) else {
            break;
        };
        current = parent;
    }
    out
}

fn parent_scope(scope: &str) -> Option<String> {
    let path = Path::new(scope);
    let parent = path.parent()?;
    let value = repo::normalize_rel_path(&parent.to_string_lossy());
    Some(if value.is_empty() { ".".to_string() } else { value })
}
