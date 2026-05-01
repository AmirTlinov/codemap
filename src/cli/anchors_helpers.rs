fn validate_anchor_domain_path(
    project: &crate::model::Project,
    id: &str,
    path: &str,
    problems: &mut Vec<String>,
) {
    let rel = repo::normalize_rel_path(path);
    if rel != "." && !project.root.join(&rel).is_dir() {
        problems.push(format!("domain `{id}` declares missing path `{rel}`"));
    }
}

fn is_glob_like(value: &str) -> bool {
    value.contains('*') || value.contains('?') || value.contains('[') || value.contains('{')
}

fn glob_static_prefix(pattern: &str) -> Option<String> {
    let wildcard = pattern.find(['*', '?', '[', '{']).unwrap_or(pattern.len());
    let prefix = &pattern[..wildcard];
    let prefix = prefix
        .rsplit_once('/')
        .map(|(head, _)| head)
        .unwrap_or(prefix)
        .trim_end_matches('/');
    if prefix.is_empty() {
        Some(".".to_string())
    } else {
        Some(prefix.to_string())
    }
}

fn anchor_pattern_matches_project(project: &crate::model::Project, raw: &str) -> bool {
    anchor_pattern_match_count(project, raw) > 0
}

fn anchor_pattern_match_count(project: &crate::model::Project, raw: &str) -> usize {
    let pattern = map::resolve_anchor_path(project, raw);
    if !is_glob_like(&pattern) {
        let mut targets = BTreeSet::new();
        if project.files.contains_key(&pattern) {
            targets.insert(pattern.clone());
        }
        for package in &project.packages {
            if package.path == pattern {
                targets.insert(package.path.clone());
            }
            if package.manifest == pattern {
                targets.insert(package.manifest.clone());
            }
        }
        for domain in &project.domains {
            if domain.path == pattern || domain.id == pattern {
                targets.insert(domain.path.clone());
            }
        }
        if pattern != "." && project.root.join(&pattern).exists() {
            targets.insert(pattern);
        }
        return targets.len();
    }
    let Ok(glob) = GlobBuilder::new(&pattern).literal_separator(true).build() else {
        return 0;
    };
    let matcher = glob.compile_matcher();
    let mut targets = BTreeSet::new();
    for rel in project.files.keys() {
        if matcher.is_match(rel) {
            targets.insert(rel.clone());
        }
    }
    for package in &project.packages {
        if matcher.is_match(&package.path) {
            targets.insert(package.path.clone());
        }
        if matcher.is_match(&package.manifest) {
            targets.insert(package.manifest.clone());
        }
    }
    for domain in &project.domains {
        if matcher.is_match(&domain.path) {
            targets.insert(domain.path.clone());
        }
    }
    targets.len()
}

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for value in values {
        if !value.is_empty() && seen.insert(value.clone()) {
            out.push(value);
        }
    }
    out
}

fn shell_quote_arg(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '-' | '_'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

