fn load_ctx_config(path: &Path) -> Result<CtxConfig> {
    let text = fs::read_to_string(path)?;
    if path.extension().and_then(|x| x.to_str()) == Some("json") {
        Ok(serde_json::from_str(&text)?)
    } else {
        Ok(yaml_serde::from_str(&text)?)
    }
}

fn load_ctx_configs(root: &Path) -> (CtxConfig, Option<String>, Vec<ConfigLoadError>) {
    let paths = find_config_paths(root);
    let mut merged = CtxConfig::default();
    let mut loaded = Vec::new();
    let mut errors = Vec::new();
    for path in paths {
        let mut config = match load_ctx_config(&root.join(&path)) {
            Ok(config) => config,
            Err(error) => {
                errors.push(ConfigLoadError {
                    path,
                    error: error.to_string(),
                });
                continue;
            }
        };
        if let Some(error) = ctx_config_version_error(&config) {
            errors.push(ConfigLoadError { path, error });
            continue;
        }
        let base = config_base_dir(&path);
        normalize_ctx_config(&mut config, &base);
        merge_ctx_config(&mut merged, config, &base);
        loaded.push(path);
    }
    let summary = match loaded.as_slice() {
        [] => None,
        [only] => Some(only.clone()),
        [first, rest @ ..] => Some(format!("{} (+{} more)", first, rest.len())),
    };
    (merged, summary, errors)
}

fn ctx_config_version_error(config: &CtxConfig) -> Option<String> {
    match config.version {
        Some(1) => None,
        Some(version) => Some(format!(
            "unsupported .ctx version `{version}`; expected `1`"
        )),
        None => Some("missing required .ctx `version: 1`".to_string()),
    }
}

fn find_config_paths(root: &Path) -> Vec<String> {
    let mut paths = BTreeSet::new();
    for name in [".ctx.yml", ".ctx.yaml", ".ctx.json"] {
        if root.join(name).exists() {
            paths.insert(name.to_string());
        }
    }
    let rels = git_list_files(root).unwrap_or_else(|| walk_files(root));
    for rel in rels {
        let name = Path::new(&rel).file_name().and_then(|s| s.to_str());
        if matches!(name, Some(".ctx.yml" | ".ctx.yaml" | ".ctx.json")) {
            paths.insert(normalize_rel_path(&rel));
        }
    }
    let mut out: Vec<String> = paths.into_iter().collect();
    out.sort_by_key(|path| (path.matches('/').count(), path.clone()));
    out
}

fn config_base_dir(config_path: &str) -> String {
    Path::new(config_path)
        .parent()
        .map(|p| normalize_rel_path(&p.to_string_lossy()))
        .filter(|p| p != ".")
        .unwrap_or_else(|| ".".to_string())
}

fn normalize_ctx_config(config: &mut CtxConfig, base: &str) {
    if config.domain.is_none() && base != "." {
        config.domain = Some(AnchorDomain {
            id: Some(
                Path::new(base)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("domain")
                    .to_string(),
            ),
            path: Some(base.to_string()),
            purpose: None,
        });
    }
    if let Some(domain) = &mut config.domain
        && base != "."
    {
        domain.path = Some(prefix_config_path(
            base,
            domain.path.as_deref().unwrap_or("."),
        ));
    }
    for domain in config.domains.values_mut() {
        if base != "." {
            domain.path = Some(prefix_config_path(
                base,
                domain.path.as_deref().unwrap_or("."),
            ));
        }
    }
    if base == "." {
        return;
    }
    for concept in config.concepts.values_mut() {
        concept.files = concept
            .files
            .iter()
            .map(|file| prefix_config_path(base, file))
            .collect();
    }
    for rule in &mut config.boundaries.forbidden {
        rule.from = prefix_config_path(base, &rule.from);
        rule.to = prefix_config_path(base, &rule.to);
    }
}

fn prefix_config_path(base: &str, value: &str) -> String {
    let raw = value.trim();
    if raw.is_empty() {
        return String::new();
    }
    let rel = raw.trim_start_matches("./");
    if base == "." || is_repo_relative_anchor_path(base, rel) {
        normalize_rel_path(rel)
    } else {
        normalize_rel_path(&format!("{base}/{rel}"))
    }
}

fn is_repo_relative_anchor_path(base: &str, rel: &str) -> bool {
    let base = base.trim_end_matches('/');
    rel == base
        || rel.starts_with(&format!("{base}/"))
        || rel.starts_with("domains/")
        || rel.starts_with("packages/")
        || rel.starts_with("apps/")
        || rel.starts_with("services/")
        || rel.starts_with("libs/")
        || rel.starts_with("crates/")
        || rel.starts_with("modules/")
        || rel.starts_with("cmd/")
        || rel.starts_with("components/")
}

fn merge_ctx_config(merged: &mut CtxConfig, mut config: CtxConfig, base: &str) {
    if merged.version.is_none() {
        merged.version = config.version;
    }
    if let Some(domain) = config.domain.take() {
        let id = domain.id.clone().unwrap_or_else(|| {
            if base == "." {
                "repo".to_string()
            } else {
                Path::new(base)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("domain")
                    .to_string()
            }
        });
        if base == "." && merged.domain.is_none() {
            merged.domain = Some(domain);
        } else {
            merged.domains.insert(id, domain);
        }
    }
    merged.domains.extend(config.domains);
    merged.owns.extend(config.owns);
    merged.does_not_own.extend(config.does_not_own);
    merged.concepts.extend(config.concepts);
    merged
        .boundaries
        .forbidden
        .extend(config.boundaries.forbidden);
    merged
        .verification
        .default
        .extend(config.verification.default);
}

