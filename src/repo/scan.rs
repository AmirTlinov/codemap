fn scan_files(root: &Path) -> Result<(BTreeMap<String, FileInfo>, ScanStats)> {
    let mut stats = ScanStatsBuilder::default();
    let rels = list_candidate_files_with_stats(root, &mut stats);
    let mut files = BTreeMap::new();
    for rel in rels {
        if rel.is_empty() {
            continue;
        }
        if let Some(reason) = ignore_reason(&rel) {
            stats.record_ignored(&reason, &rel);
            continue;
        }
        stats.files_visited += 1;
        let path = root.join(&rel);
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.file_type().is_symlink() || !meta.is_file() {
            stats.record_skipped("not_regular_file", &rel);
            continue;
        }
        if let Some(reason) = scan_file_rejection(&path, meta.len()) {
            stats.record_skipped(reason, &rel);
            continue;
        }
        stats.bytes_scanned += meta.len();
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
            unresolved_imports: BTreeSet::new(),
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
        if info.has_role("generated") {
            stats.record_generated("generated_path_or_header", &rel);
        }
        files.insert(rel, info);
    }
    stats.files_scanned = files.len();
    Ok((files, stats.finish()))
}

fn list_candidate_files(root: &Path) -> Vec<String> {
    let mut stats = ScanStatsBuilder::default();
    list_candidate_files_with_stats(root, &mut stats)
}

fn list_visible_candidate_files(root: &Path) -> Vec<String> {
    list_candidate_files(root)
        .into_iter()
        .filter(|rel| !should_ignore_rel(rel))
        .collect()
}

fn list_candidate_files_with_stats(root: &Path, stats: &mut ScanStatsBuilder) -> Vec<String> {
    git_list_files(root, stats).unwrap_or_else(|| walk_files(root, stats))
}

fn git_list_files(root: &Path, stats: &mut ScanStatsBuilder) -> Option<Vec<String>> {
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
        .collect::<BTreeSet<_>>();
    rels.extend(walk_files(root, stats));
    Some(rels.into_iter().collect())
}

fn walk_files(root: &Path, stats: &mut ScanStatsBuilder) -> Vec<String> {
    let mut builder = WalkBuilder::new(root);
    builder
        .standard_filters(true)
        .hidden(false)
        .follow_links(false);
    let root_for_filter = root.to_path_buf();
    let ignored_entries = Arc::new(Mutex::new(Vec::<(String, String)>::new()));
    let ignored_for_filter = Arc::clone(&ignored_entries);
    builder.filter_entry(move |entry| {
        entry
            .path()
            .strip_prefix(&root_for_filter)
            .ok()
            .map(|path| {
                let rel = normalize_rel_path(&path.to_string_lossy());
                if rel.is_empty() {
                    return true;
                }
                if let Some(reason) = ignore_reason(&rel) {
                    if let Ok(mut entries) = ignored_for_filter.lock() {
                        entries.push((reason, rel));
                    }
                    return false;
                }
                true
            })
            .unwrap_or(true)
    });
    let out = builder
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
        .collect();
    if let Ok(entries) = ignored_entries.lock() {
        for (reason, rel) in entries.iter() {
            stats.record_ignored(reason, rel);
        }
    }
    out
}

fn should_ignore_rel(rel: &str) -> bool {
    ignore_reason(rel).is_some()
}

fn ignore_reason(rel: &str) -> Option<String> {
    rel.split('/').find_map(|part| {
        COMMON_IGNORE_DIRS
            .iter()
            .any(|ignored| ignored == &part)
            .then(|| format!("common_ignore_dir:{part}"))
    })
}

fn scan_file_rejection(path: &Path, size: u64) -> Option<&'static str> {
    if size > 900_000 {
        return Some("too_large");
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
        return None;
    }
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    if BINARY_EXTS.iter().any(|x| x == &ext) {
        return Some("binary_extension");
    }
    if SOURCE_EXTS.iter().any(|x| x == &ext) || TEXT_EXTS.iter().any(|x| x == &ext) {
        None
    } else {
        Some("unsupported_extension")
    }
}

fn language_for(path: &Path) -> String {
    match path.extension().and_then(|s| s.to_str()).unwrap_or("") {
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte" => "javascript/typescript",
        "py" => "python",
        "rs" => "rust",
        "go" => "go",
        "swift" => "swift",
        "json" | "toml" | "yaml" | "yml" => "config",
        "css" | "scss" | "sass" | "less" => "style",
        "md" => "markdown",
        _ => "unknown",
    }
    .to_string()
}

#[derive(Default)]
struct ScanStatsBuilder {
    files_visited: usize,
    files_scanned: usize,
    bytes_scanned: u64,
    ignored: BTreeMap<String, ScanGroupBuilder>,
    skipped: BTreeMap<String, ScanGroupBuilder>,
    generated: BTreeMap<String, ScanGroupBuilder>,
}

impl ScanStatsBuilder {
    fn record_ignored(&mut self, reason: &str, rel: &str) {
        self.record_group(GroupKind::Ignored, reason, &ignored_group_unit(reason, rel));
    }

    fn record_skipped(&mut self, reason: &str, rel: &str) {
        self.record_group(GroupKind::Skipped, reason, rel);
    }

    fn record_generated(&mut self, reason: &str, rel: &str) {
        self.record_group(GroupKind::Generated, reason, rel);
    }

    fn record_group(&mut self, kind: GroupKind, reason: &str, rel: &str) {
        let groups = match kind {
            GroupKind::Ignored => &mut self.ignored,
            GroupKind::Skipped => &mut self.skipped,
            GroupKind::Generated => &mut self.generated,
        };
        let group = groups.entry(reason.to_string()).or_default();
        if group.seen.insert(rel.to_string()) {
            group.count += 1;
            if group.examples.len() < 5 {
                group.examples.push(rel.to_string());
            }
        }
    }

    fn finish(self) -> ScanStats {
        ScanStats {
            files_visited: self.files_visited,
            files_scanned: self.files_scanned,
            files_skipped: self.skipped.values().map(|group| group.count).sum(),
            bytes_scanned: self.bytes_scanned,
            ignored: finish_groups(self.ignored),
            generated: finish_groups(self.generated),
        }
    }
}

#[derive(Clone, Copy)]
enum GroupKind {
    Ignored,
    Skipped,
    Generated,
}

#[derive(Default)]
struct ScanGroupBuilder {
    count: usize,
    examples: Vec<String>,
    seen: BTreeSet<String>,
}

fn finish_groups(groups: BTreeMap<String, ScanGroupBuilder>) -> Vec<ScanGroup> {
    groups
        .into_iter()
        .map(|(reason, group)| ScanGroup {
            reason,
            count: group.count,
            examples: group.examples,
        })
        .collect()
}

fn ignored_group_unit(reason: &str, rel: &str) -> String {
    let Some(dir) = reason.strip_prefix("common_ignore_dir:") else {
        return rel.to_string();
    };
    let mut parts = Vec::new();
    for part in rel.split('/') {
        parts.push(part);
        if part == dir {
            return parts.join("/");
        }
    }
    rel.to_string()
}
