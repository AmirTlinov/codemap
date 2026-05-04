fn scan_files(root: &Path) -> Result<(BTreeMap<String, FileInfo>, ScanStats)> {
    let mut stats = ScanStatsBuilder::default();
    let rels = list_candidate_files_with_stats(root, &mut stats);
    let (files, scan_stats) = scan_candidate_rels(root, rels);
    stats.merge(scan_stats);
    stats.files_scanned = files.len();
    Ok((files, stats.finish()))
}

fn scan_candidate_rels(root: &Path, rels: Vec<String>) -> (BTreeMap<String, FileInfo>, ScanStatsBuilder) {
    let worker_count = scan_worker_count(rels.len());
    if worker_count <= 1 {
        return scan_candidate_rels_sequential(root, rels);
    }

    let chunk_size = rels.len().div_ceil(worker_count).max(1);
    let mut results = Vec::new();
    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in rels.chunks(chunk_size) {
            let chunk = chunk.to_vec();
            handles.push(scope.spawn(move || scan_candidate_rels_sequential(root, chunk)));
        }
        for handle in handles {
            results.push(handle.join().expect("scan worker should not panic"));
        }
    });

    let mut files = BTreeMap::new();
    let mut stats = ScanStatsBuilder::default();
    for (worker_files, worker_stats) in results {
        files.extend(worker_files);
        stats.merge(worker_stats);
    }
    (files, stats)
}

fn scan_candidate_rels_sequential(
    root: &Path,
    rels: Vec<String>,
) -> (BTreeMap<String, FileInfo>, ScanStatsBuilder) {
    let mut stats = ScanStatsBuilder::default();
    let mut files = BTreeMap::new();
    for rel in rels {
        if rel.is_empty() {
            continue;
        }
        if let Some(reason) = ignore_reason(&rel) {
            stats.record_ignored(&reason, &rel);
            continue;
        }
        if let Some(info) = scan_file(root, &rel, &mut stats) {
            files.insert(rel, info);
        }
    }
    (files, stats)
}

fn scan_worker_count(file_count: usize) -> usize {
    if file_count < 256 {
        return 1;
    }
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(8)
        .min(file_count)
}

fn scan_selected_files(root: &Path, rels: &BTreeSet<String>) -> (BTreeMap<String, FileInfo>, ScanStats) {
    let mut stats = ScanStatsBuilder::default();
    let mut files = BTreeMap::new();
    for rel in rels {
        if let Some(info) = scan_file(root, rel, &mut stats) {
            files.insert(rel.clone(), info);
        }
    }
    stats.files_scanned = files.len();
    (files, stats.finish())
}

fn scan_file(root: &Path, rel: &str, stats: &mut ScanStatsBuilder) -> Option<FileInfo> {
    stats.files_visited += 1;
    let path = root.join(rel);
    let Ok(meta) = fs::symlink_metadata(&path) else {
        return None;
    };
    if meta.file_type().is_symlink() || !meta.is_file() {
        stats.record_skipped("not_regular_file", rel);
        return None;
    }
    if let Some(reason) = scan_file_rejection(&path, meta.len()) {
        stats.record_skipped(reason, rel);
        return None;
    }
    stats.bytes_scanned += meta.len();
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let language = language_for(&path);
    let mut info = FileInfo {
        rel: rel.to_string(),
        ext,
        size: meta.len(),
        content_hash: None,
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
        tokens: path_tokens(rel),
        references: BTreeSet::new(),
        jsx_tags: BTreeSet::new(),
        local_bindings: BTreeSet::new(),
        surface_tokens: BTreeSet::new(),
        surface_phrases: BTreeSet::new(),
        visited_route_paths: BTreeSet::new(),
    };
    classify_roles(root, &mut info);
    if is_asset_ext(&info.ext) && info.ext != "svg" {
        if let Ok(bytes) = fs::read(&path) {
            info.content_hash = Some(scan_content_hash(&bytes));
        }
    } else {
        extract_imports_exports(root, &mut info);
    }
    if info.has_role("generated") {
        stats.record_generated("generated_path_or_header", rel);
    }
    Some(info)
}

fn cache_candidate_files(root: &Path) -> Vec<String> {
    let candidates =
        git_cache_candidate_files(root).unwrap_or_else(|| list_visible_candidate_files(root));
    let mut files = candidates
        .into_iter()
        .filter(|rel| is_cache_candidate_file(root, rel))
        .collect::<Vec<_>>();
    files.sort();
    files
}

pub(crate) fn structural_inventory_candidate_files(root: &Path) -> Vec<String> {
    cache_candidate_files(root)
}

fn is_cache_candidate_file(root: &Path, rel: &str) -> bool {
    if should_ignore_rel(rel) {
        return false;
    }
    let path = root.join(rel);
    fs::symlink_metadata(&path)
        .ok()
        .filter(|meta| !meta.file_type().is_symlink() && meta.is_file())
        .is_some_and(|meta| scan_file_rejection(&path, meta.len()).is_none())
}

fn git_cache_candidate_files(root: &Path) -> Option<Vec<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-c", "-o", "--exclude-standard"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(normalize_rel_path)
            .filter(|rel| !rel.is_empty() && !should_ignore_rel(rel))
            .collect(),
    )
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
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if is_env_surface_name(&name) || is_lockfile_name(&name) {
        return None;
    }
    if size > 900_000 && !is_asset_ext(&ext) {
        return Some("too_large");
    }
    if size > 5_000_000 && is_asset_ext(&ext) {
        return Some("too_large_asset");
    }
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
    if BINARY_EXTS.iter().any(|x| x == &ext) && !is_asset_ext(&ext) {
        return Some("binary_extension");
    }
    if SOURCE_EXTS.iter().any(|x| x == &ext)
        || TEXT_EXTS.iter().any(|x| x == &ext)
        || is_asset_ext(&ext)
        || is_snapshot_ext(&ext)
    {
        None
    } else {
        Some("unsupported_extension")
    }
}

fn language_for(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if is_env_surface_name(&name) {
        return "env".to_string();
    }
    if is_lockfile_name(&name) {
        return "lockfile".to_string();
    }
    if matches!(name.as_str(), "dockerfile" | "jenkinsfile" | "earthfile") {
        return "config".to_string();
    }
    match ext.as_str() {
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte" => "javascript/typescript",
        "py" => "python",
        "rs" => "rust",
        "go" => "go",
        "swift" => "swift",
        "json" | "toml" | "yaml" | "yml" => "config",
        "prisma" | "graphql" | "gql" | "proto" | "avsc" => "schema",
        "sql" => "sql",
        "css" | "scss" | "sass" | "less" => "style",
        ext if is_script_ext(ext) => "shell",
        ext if is_asset_ext(ext) => "asset",
        ext if is_snapshot_ext(ext) => "snapshot",
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
    fn merge(&mut self, other: ScanStatsBuilder) {
        self.files_visited += other.files_visited;
        self.files_scanned += other.files_scanned;
        self.bytes_scanned += other.bytes_scanned;
        self.merge_groups(GroupKind::Ignored, other.ignored);
        self.merge_groups(GroupKind::Skipped, other.skipped);
        self.merge_groups(GroupKind::Generated, other.generated);
    }

    fn merge_groups(&mut self, kind: GroupKind, groups: BTreeMap<String, ScanGroupBuilder>) {
        for (reason, group) in groups {
            for rel in group.seen {
                self.record_group(kind, &reason, &rel);
            }
        }
    }

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
