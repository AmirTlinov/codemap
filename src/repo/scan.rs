fn scan_files(root: &Path) -> Result<BTreeMap<String, FileInfo>> {
    let rels = git_list_files(root).unwrap_or_else(|| walk_files(root));
    let mut files = BTreeMap::new();
    for rel in rels {
        if rel.is_empty() || should_ignore_rel(&rel) {
            continue;
        }
        let path = root.join(&rel);
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.file_type().is_symlink() || !meta.is_file() || !should_scan_file(&path, meta.len())
        {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let language = language_for(&path);
        let mut info = FileInfo {
            rel: rel.clone(),
            ext,
            size: meta.len(),
            line_count: 0,
            language,
            roles: BTreeSet::new(),
            imports: BTreeSet::new(),
            import_bindings: BTreeMap::new(),
            resolved_imports: BTreeSet::new(),
            resolved_import_bindings: BTreeMap::new(),
            exports: BTreeSet::new(),
            symbols: Vec::new(),
            tokens: path_tokens(&rel),
            references: BTreeSet::new(),
            jsx_tags: BTreeSet::new(),
            local_bindings: BTreeSet::new(),
            surface_tokens: BTreeSet::new(),
            surface_phrases: BTreeSet::new(),
            visited_route_paths: BTreeSet::new(),
        };
        classify_roles(root, &mut info);
        extract_imports_exports(root, &mut info);
        files.insert(rel, info);
    }
    Ok(files)
}

fn git_list_files(root: &Path) -> Option<Vec<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-c", "--exclude-standard"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let mut rels = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(normalize_rel_path)
        .filter(|rel| !rel.is_empty())
        .filter(|rel| !should_ignore_rel(rel))
        .collect::<BTreeSet<_>>();
    rels.extend(walk_files(root));
    Some(rels.into_iter().collect())
}

fn walk_files(root: &Path) -> Vec<String> {
    let mut builder = WalkBuilder::new(root);
    builder
        .standard_filters(true)
        .hidden(false)
        .follow_links(false);
    let root_for_filter = root.to_path_buf();
    builder.filter_entry(move |entry| {
        entry
            .path()
            .strip_prefix(&root_for_filter)
            .ok()
            .map(|path| {
                let rel = normalize_rel_path(&path.to_string_lossy());
                rel.is_empty() || !should_ignore_rel(&rel)
            })
            .unwrap_or(true)
    });
    builder
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().map(|ft| ft.is_file()).unwrap_or(false))
        .filter_map(|entry| {
            entry
                .path()
                .strip_prefix(root)
                .ok()
                .map(|p| normalize_rel_path(&p.to_string_lossy()))
        })
        .collect()
}

fn should_ignore_rel(rel: &str) -> bool {
    rel.split('/')
        .any(|part| COMMON_IGNORE_DIRS.iter().any(|ignored| ignored == &part))
}

fn should_scan_file(path: &Path, size: u64) -> bool {
    if size > 900_000 {
        return false;
    }
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if matches!(
        name.as_str(),
        "package.json"
            | "pyproject.toml"
            | "cargo.toml"
            | "go.mod"
            | "go.work"
            | "agents.md"
            | "readme.md"
            | "makefile"
            | "justfile"
            | "jenkinsfile"
            | "dockerfile"
            | "earthfile"
            | "taskfile"
            | "taskfile.yml"
            | "taskfile.yaml"
            | ".env.example"
            | ".env.sample"
            | ".ctx.yml"
            | ".ctx.yaml"
            | ".ctx.json"
    ) {
        return true;
    }
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    if BINARY_EXTS.iter().any(|x| x == &ext) {
        return false;
    }
    SOURCE_EXTS.iter().any(|x| x == &ext) || TEXT_EXTS.iter().any(|x| x == &ext)
}

fn language_for(path: &Path) -> String {
    match path.extension().and_then(|s| s.to_str()).unwrap_or("") {
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte" => "javascript/typescript",
        "py" => "python",
        "rs" => "rust",
        "go" => "go",
        "swift" => "swift",
        "json" | "toml" | "yaml" | "yml" => "config",
        "md" => "markdown",
        _ => "unknown",
    }
    .to_string()
}
