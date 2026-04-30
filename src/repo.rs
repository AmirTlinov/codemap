use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use globset::GlobBuilder;
use ignore::WalkBuilder;
use regex::Regex;

use crate::cache;
use crate::model::{
    AnchorDomain, ConfigLoadError, CtxConfig, Domain, FileInfo, ImportBindingsBySpec,
    PackageDependency, PackageInfo, Project, ScriptInfo, SymbolInfo,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const ROOT_MARKERS: &[&str] = &[
    ".ctx.yml",
    ".ctx.yaml",
    ".ctx.json",
    "package.json",
    "pnpm-workspace.yaml",
    "yarn.lock",
    "package-lock.json",
    "Cargo.toml",
    "go.mod",
    "go.work",
    "pyproject.toml",
    "requirements.txt",
    "Package.swift",
    "Makefile",
    "justfile",
];

const COMMON_IGNORE_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    ".idea",
    ".vscode",
    "node_modules",
    ".pnpm-store",
    ".yarn",
    "bower_components",
    "dist",
    "build",
    "out",
    "target",
    ".next",
    ".nuxt",
    ".turbo",
    ".cache",
    "coverage",
    ".pytest_cache",
    "__pycache__",
    ".mypy_cache",
    ".ruff_cache",
    "vendor",
    "tmp",
    "temp",
    "logs",
];

const BINARY_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "ico", "pdf", "zip", "gz", "tgz", "rar", "7z", "exe",
    "dll", "so", "dylib", "wasm", "woff", "woff2", "ttf", "otf", "mp3", "mp4", "mov", "avi", "mkv",
    "bin", "class", "jar",
];

const SOURCE_EXTS: &[&str] = &[
    "ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "rs", "go", "java", "kt", "kts", "swift", "c",
    "cc", "cpp", "h", "hpp", "cs", "rb", "php", "vue", "svelte",
];

const TEXT_EXTS: &[&str] = &[
    "json", "toml", "yaml", "yml", "md", "txt", "sql", "graphql", "proto",
];

const DOMAIN_HINT_DIRS: &[&str] = &[
    "domains",
    "packages",
    "apps",
    "services",
    "libs",
    "crates",
    "modules",
    "cmd",
    "components",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheWriteMode {
    Enabled,
    ReadOnly,
}

#[derive(Debug, Clone)]
pub enum RootSelection {
    Auto,
    Exact(PathBuf),
    Discover(PathBuf),
}

pub fn load_project_with_cache(
    root_selection: RootSelection,
    cache_write: CacheWriteMode,
) -> Result<Project> {
    let root_hint = match &root_selection {
        RootSelection::Auto => None,
        RootSelection::Exact(path) | RootSelection::Discover(path) => Some(path),
    };
    let cwd = if let Some(path) = root_hint {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    } else {
        env::current_dir().context("failed to read current directory")?
    };
    let root = resolve_root(&root_selection, &cwd)?;
    let remote = git_remote(&root);
    let (anchors, config_path, config_errors) = load_ctx_configs(&root);
    let nearest_agents = nearest_agents(&cwd, &root);
    let mut files = scan_files(&root)?;
    let packages = detect_packages(&root, &files);
    let ts_path_aliases = detect_ts_path_aliases(&root, &files);
    resolve_imports(&root, &mut files, &packages, &ts_path_aliases);
    let reverse_imports = build_reverse_imports(&files);
    let package_edges = detect_package_edges(&root, &files, &packages);
    let scripts = detect_scripts(&root);
    let package_manager = detect_package_manager(&root);
    let languages = detect_languages(&files);
    let domains = discover_domains(&root, &files, &anchors, config_path.as_deref());
    let vcs = if is_git_repo(&root) {
        Some("git".to_string())
    } else {
        None
    };
    let cache_dir = cache::project_cache_dir(&root, remote.as_deref(), VERSION);

    let mut project = Project {
        root,
        cwd,
        vcs,
        cache_dir,
        config_path,
        config_errors,
        nearest_agents,
        files,
        reverse_imports,
        packages,
        package_edges,
        domains,
        package_manager,
        scripts,
        languages,
        anchors,
        cache_state: String::new(),
        cache_artifacts: Vec::new(),
    };
    let fingerprint = cache::fingerprint(&project, None);
    let cache_artifacts = cache::artifact_statuses(&project, &fingerprint);
    project.cache_state = cache::cache_state(&cache_artifacts);
    project.cache_artifacts = cache_artifacts;
    if cache_write == CacheWriteMode::Enabled {
        cache::write_status(&project, VERSION)?;
    }
    Ok(project)
}

fn resolve_root(root_selection: &RootSelection, cwd: &Path) -> Result<PathBuf> {
    match root_selection {
        RootSelection::Exact(path) => {
            Ok(path.canonicalize().unwrap_or_else(|_| path.to_path_buf()))
        }
        RootSelection::Discover(path) => {
            let base = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            if let Some(git_root) = git_root(&base) {
                return Ok(git_root);
            }
            Ok(marker_root(&base).unwrap_or(base))
        }
        RootSelection::Auto => {
            if let Some(git_root) = git_root(cwd) {
                return Ok(git_root);
            }
            Ok(marker_root(cwd).unwrap_or_else(|| cwd.to_path_buf()))
        }
    }
}

pub fn ambient_root(start: &Path) -> Option<PathBuf> {
    git_root(start).or_else(|| marker_root(start))
}

fn git_root(start: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(start)
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if raw.is_empty() {
        None
    } else {
        Some(PathBuf::from(raw))
    }
}

fn git_remote(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .output()
        .ok()?;
    if output.status.success() {
        let remote = String::from_utf8_lossy(&output.stdout).trim().to_string();
        (!remote.is_empty()).then_some(remote)
    } else {
        None
    }
}

fn is_git_repo(root: &Path) -> bool {
    root.join(".git").exists() || git_root(root).is_some()
}

fn marker_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(path) = current {
        if ROOT_MARKERS.iter().any(|marker| path.join(marker).exists()) {
            return Some(path.to_path_buf());
        }
        current = path.parent();
    }
    None
}

fn nearest_agents(cwd: &Path, root: &Path) -> Option<String> {
    let mut current = Some(cwd);
    while let Some(path) = current {
        let candidate = path.join("AGENTS.md");
        if candidate.exists() {
            return candidate
                .strip_prefix(root)
                .ok()
                .map(|p| normalize_rel_path(&p.to_string_lossy()));
        }
        if path == root {
            break;
        }
        current = path.parent();
    }
    None
}

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
        classify_roles(&mut info);
        extract_imports_exports(root, &mut info);
        files.insert(rel, info);
    }
    Ok(files)
}

fn git_list_files(root: &Path) -> Option<Vec<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-co", "--exclude-standard"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(normalize_rel_path)
            .filter(|rel| !rel.is_empty())
            .collect(),
    )
}

fn walk_files(root: &Path) -> Vec<String> {
    WalkBuilder::new(root)
        .standard_filters(true)
        .hidden(false)
        .follow_links(false)
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

fn classify_roles(info: &mut FileInfo) {
    let rel = info.rel.to_ascii_lowercase();
    let name = Path::new(&info.rel)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if is_generated(&rel) {
        info.roles.insert("generated".to_string());
    }
    if rel.starts_with("fixtures/") || rel.contains("/fixtures/") {
        info.roles.insert("fixture".to_string());
    }
    if rel.starts_with("examples/")
        || rel.contains("/examples/")
        || rel.starts_with("samples/")
        || rel.contains("/samples/")
    {
        info.roles.insert("example".to_string());
    }
    if is_test_path(&rel) {
        info.roles.insert("test".to_string());
        if is_e2e_test_path(&rel) {
            info.roles.insert("e2e_test".to_string());
        }
        if is_test_support_path(&rel) || name == "__init__.py" || name == "conftest.py" {
            info.roles.insert("test_support".to_string());
        }
    }
    if matches!(
        name.as_str(),
        "index.ts"
            | "index.tsx"
            | "index.js"
            | "index.jsx"
            | "mod.rs"
            | "lib.rs"
            | "main.rs"
            | "main.go"
            | "__init__.py"
            | "api.ts"
            | "routes.ts"
            | "package.json"
            | "cargo.toml"
            | "go.mod"
            | "pyproject.toml"
            | "package.swift"
    ) {
        info.roles.insert("public_boundary".to_string());
    }
    add_role_if(
        &mut info.roles,
        &rel,
        &[
            "state",
            "store",
            "model",
            "entity",
            "timeline",
            "reducer",
            "machine",
            "registry",
            "repository",
            "aggregate",
        ],
        "state_model",
    );
    add_role_if(
        &mut info.roles,
        &rel,
        &["session", "cursor", "clock", "controller", "manager"],
        "runtime_state",
    );
    add_role_if(
        &mut info.roles,
        &rel,
        &[
            "schema",
            "contract",
            "dto",
            "types",
            "interface",
            "migration",
        ],
        "schema_contract",
    );
    add_role_if(
        &mut info.roles,
        &rel,
        &["adapter", "gateway", "client", "provider", "port", "driver"],
        "adapter",
    );
    add_role_if(
        &mut info.roles,
        &rel,
        &["parser", "parse", "loader", "reader", "decoder"],
        "parser",
    );
    add_role_if(
        &mut info.roles,
        &rel,
        &["render", "view", "component", "page", "screen", "ui"],
        "renderer_ui",
    );
    add_role_if(
        &mut info.roles,
        &rel,
        &["save", "load", "reopen", "persist", "storage"],
        "persistence",
    );
    add_role_if(
        &mut info.roles,
        &rel,
        &["route", "map", "lens", "impact", "proof", "cone"],
        "map_engine",
    );
    add_role_if(
        &mut info.roles,
        &rel,
        &["root", "inventory", "files", "discover"],
        "repo_discovery",
    );
    if matches!(name.as_str(), "repo.rs" | "repo.ts" | "repo.js") {
        info.roles.insert("repo_discovery".to_string());
    }
    add_role_if(&mut info.roles, &rel, &["cache", "fingerprint"], "cache");
    add_role_if(&mut info.roles, &rel, &["cli", "command"], "cli_surface");
    if is_build_ci_surface(&rel, &name, &info.tokens) {
        info.roles.insert("build_ci".to_string());
    }
    if name == "agents.md" {
        info.roles.insert("agent_bootstrap".to_string());
    }
    if matches!(name.as_str(), ".ctx.yml" | ".ctx.yaml" | ".ctx.json") {
        info.roles.insert("semantic_anchor".to_string());
    }
    if info.roles.contains("test") {
        for role in [
            "state_model",
            "runtime_state",
            "public_boundary",
            "adapter",
            "schema_contract",
            "parser",
            "renderer_ui",
            "persistence",
            "map_engine",
            "repo_discovery",
            "cache",
            "cli_surface",
            "build_ci",
        ] {
            info.roles.remove(role);
        }
    }
}

fn add_role_if(roles: &mut BTreeSet<String>, haystack: &str, needles: &[&str], role: &str) {
    if needles.iter().any(|needle| haystack.contains(needle)) {
        roles.insert(role.to_string());
    }
}

fn is_build_ci_surface(rel: &str, name: &str, tokens: &BTreeSet<String>) -> bool {
    rel.starts_with(".github/workflows/")
        || rel.starts_with(".circleci/")
        || rel.starts_with(".buildkite/")
        || rel.starts_with(".teamcity/")
        || matches!(
            name,
            ".gitlab-ci.yml"
                | ".gitlab-ci.yaml"
                | "azure-pipelines.yml"
                | "azure-pipelines.yaml"
                | "bitbucket-pipelines.yml"
                | "bitbucket-pipelines.yaml"
                | ".drone.yml"
                | ".drone.yaml"
                | ".woodpecker.yml"
                | ".woodpecker.yaml"
                | "jenkinsfile"
                | "dockerfile"
                | "docker-compose.yml"
                | "docker-compose.yaml"
                | "compose.yml"
                | "compose.yaml"
                | "makefile"
                | "justfile"
                | "taskfile"
                | "taskfile.yml"
                | "taskfile.yaml"
                | "earthfile"
        )
        || tokens.contains("build")
        || tokens.contains("ci")
        || tokens.contains("workflow")
}

fn is_generated(rel: &str) -> bool {
    rel.contains(".generated.")
        || rel.contains(".gen.")
        || rel.contains("/generated/")
        || rel.ends_with(".pb.go")
        || rel.ends_with(".g.dart")
}

fn is_test_path(rel: &str) -> bool {
    rel.contains("/tests/")
        || rel.contains("/test/")
        || rel.starts_with("tests/")
        || rel.starts_with("test/")
        || rel.contains("/__tests__/")
        || rel.contains(".test.")
        || rel.contains(".spec.")
        || rel.ends_with("_test.rs")
        || rel.ends_with("_test.go")
        || rel
            .rsplit('/')
            .next()
            .map(|name| name.starts_with("test_"))
            .unwrap_or(false)
}

fn is_e2e_test_path(rel: &str) -> bool {
    let rel = rel.to_ascii_lowercase();
    rel.contains("/e2e/")
        || rel.contains("/e2e-")
        || rel.contains(".e2e.")
        || rel.contains("/playwright/")
        || rel.contains("/cypress/")
}

fn is_test_support_path(rel: &str) -> bool {
    let rel = rel.to_ascii_lowercase();
    rel.contains("/support/")
        || rel.contains("/helpers/")
        || rel.contains("/fixtures/")
        || rel.contains("/mocks/")
        || rel.contains("/setup")
        || rel.contains(".setup.")
}

fn extract_imports_exports(root: &Path, info: &mut FileInfo) {
    let path = root.join(&info.rel);
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    info.line_count = line_count(&text);
    if !is_source_ext(&info.ext) {
        return;
    }
    let surfaces = extract_surfaces(&text, &info.ext);
    info.surface_tokens = surfaces.tokens;
    info.surface_phrases = surfaces.phrases;
    info.visited_route_paths = surfaces.visited_routes;
    info.symbols = extract_symbols(&text, &info.ext);
    info.references = extract_identifier_references(&text, &info.ext);
    info.jsx_tags = extract_jsx_tags(&text, &info.ext);
    info.local_bindings = extract_local_bindings(&text, &info.ext);
    match info.ext.as_str() {
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte" => {
            info.imports.extend(extract_js_import_specs(&text));
            info.import_bindings = extract_js_import_bindings(&text);
            let export_re = js_export_re();
            for cap in export_re.captures_iter(&text) {
                if let Some(m) = cap.get(1) {
                    info.exports.insert(m.as_str().trim().to_string());
                }
            }
        }
        "py" => {
            let import_re = py_import_re();
            for cap in import_re.captures_iter(&text) {
                if let Some(m) = cap.get(1).or_else(|| cap.get(2)) {
                    info.imports.insert(m.as_str().trim().to_string());
                }
            }
            let def_re = py_def_re();
            for cap in def_re.captures_iter(&text) {
                if let Some(m) = cap.get(1) {
                    info.exports.insert(m.as_str().trim().to_string());
                }
            }
        }
        "rs" => {
            let use_re = rust_use_re();
            for cap in use_re.captures_iter(&text) {
                if let Some(m) = cap.get(1) {
                    info.imports.insert(m.as_str().trim().to_string());
                }
            }
            let mod_re = rust_mod_re();
            for cap in mod_re.captures_iter(&text) {
                if let Some(m) = cap.get(1) {
                    info.imports.insert(m.as_str().trim().to_string());
                }
            }
        }
        "go" => {
            info.imports.extend(extract_go_imports(&text));
        }
        "swift" => {
            let import_re = swift_import_re();
            let import_text = code_without_comments_or_strings(&text, &info.ext);
            for cap in import_re.captures_iter(&import_text) {
                if let Some(m) = cap.get(1) {
                    info.imports.insert(m.as_str().trim().to_string());
                }
            }
            for symbol in &info.symbols {
                if symbol.exported {
                    info.exports.insert(symbol.name.clone());
                }
            }
        }
        _ => {}
    }
}

fn line_count(text: &str) -> usize {
    text.lines().count()
}

#[derive(Debug, Clone)]
struct SymbolStart {
    name: String,
    kind: String,
    exported: bool,
    line_start: usize,
    indent: usize,
}

fn extract_symbols(text: &str, ext: &str) -> Vec<SymbolInfo> {
    if ext == "swift" {
        let cleaned = code_without_comments_or_strings(text, ext);
        return symbols_with_ranges(extract_swift_symbols(&cleaned), &cleaned, ext);
    }
    let starts = match ext {
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte" => extract_js_symbols(text),
        "rs" => extract_rust_symbols(text),
        "py" => extract_python_symbols(text),
        "go" => extract_go_symbols(text),
        _ => Vec::new(),
    };
    symbols_with_ranges(starts, text, ext)
}

fn extract_identifier_references(text: &str, ext: &str) -> BTreeSet<String> {
    if !is_source_ext(ext) {
        return BTreeSet::new();
    }
    let cleaned = code_without_comments_or_strings(text, ext);
    identifier_re()
        .find_iter(&cleaned)
        .filter(|m| !identifier_is_selector_tail(&cleaned, m.start()))
        .map(|m| m.as_str())
        .filter(|name| !language_keyword(name))
        .map(str::to_string)
        .collect()
}

fn extract_jsx_tags(text: &str, ext: &str) -> BTreeSet<String> {
    if !matches!(ext, "tsx" | "jsx" | "vue" | "svelte") {
        return BTreeSet::new();
    }
    let cleaned = code_without_comments_or_strings(text, ext);
    let mut out = BTreeSet::new();
    let mut type_brace_depth: Option<usize> = None;
    for line in cleaned.lines() {
        let trimmed = line.trim_start();
        if let Some(depth) = type_brace_depth.as_mut() {
            *depth = js_type_brace_depth_after_line(*depth, line);
            if js_type_context_line_is_complete(trimmed, *depth) {
                type_brace_depth = None;
            }
            continue;
        }
        if js_type_context_line_starts(trimmed) {
            let depth = js_type_brace_depth_after_line(0, line);
            if !js_type_context_line_is_complete(trimmed, depth) {
                type_brace_depth = Some(depth);
            }
            continue;
        }
        let line = js_line_without_regex_literals(line);
        out.extend(
            jsx_tag_re()
                .captures_iter(&line)
                .filter(|cap| {
                    cap.get(0)
                        .zip(cap.get(1))
                        .map(|(tag, name)| jsx_tag_context_is_value(&line, tag.start(), name.end()))
                        .unwrap_or(false)
                })
                .filter_map(|cap| cap.get(1))
                .map(|m| m.as_str().to_string()),
        );
    }
    out
}

fn jsx_tag_context_is_value(text: &str, tag_start: usize, name_end: usize) -> bool {
    let before = tag_start
        .checked_sub(1)
        .and_then(|index| text.as_bytes().get(index))
        .copied();
    if before
        .map(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$' | b'.'))
        .unwrap_or(false)
    {
        return false;
    }
    let after = text[name_end..]
        .bytes()
        .find(|byte| !byte.is_ascii_whitespace());
    if matches!(after, Some(b'|' | b'&' | b',' | b'=')) {
        return false;
    }
    let after = text[name_end..].trim_start();
    !(after.starts_with("extends ")
        || after.starts_with("extends\t")
        || after.starts_with("extends\n")
        || after.starts_with(">()")
        || after.starts_with("> ()"))
}

fn js_type_context_line_starts(trimmed: &str) -> bool {
    trimmed.starts_with("type ")
        || trimmed.starts_with("export type ")
        || trimmed.starts_with("interface ")
        || trimmed.starts_with("export interface ")
}

fn js_type_context_line_is_complete(trimmed: &str, brace_depth: usize) -> bool {
    brace_depth == 0 && (trimmed.contains(';') || trimmed.ends_with('}') || trimmed.ends_with("};"))
}

fn js_type_brace_depth_after_line(mut depth: usize, code: &str) -> usize {
    for byte in code.bytes() {
        match byte {
            b'{' => depth = depth.saturating_add(1),
            b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth
}

fn js_line_without_regex_literals(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut out = String::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'/'
            && js_regex_literal_can_start(&out)
            && let Some(end) = js_regex_literal_end(bytes, index)
        {
            index = end;
            continue;
        }
        out.push(bytes[index] as char);
        index += 1;
    }
    out
}

fn js_regex_literal_can_start(prefix: &str) -> bool {
    [
        "return", "await", "throw", "yield", "case", "delete", "void", "typeof", "else",
    ]
    .iter()
    .any(|word| previous_word_is(prefix, word))
        || prefix.trim_end().ends_with("=>")
        || previous_nonspace_byte(prefix)
            .map(|byte| {
                matches!(
                    byte,
                    b'(' | b')'
                        | b'='
                        | b':'
                        | b'['
                        | b'{'
                        | b','
                        | b'!'
                        | b'?'
                        | b';'
                        | b'|'
                        | b'&'
                )
            })
            .unwrap_or(true)
}

fn previous_word_is(before: &str, word: &str) -> bool {
    let bytes = before.as_bytes();
    let mut end = bytes.len();
    while end > 0 && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    if end == 0 {
        return false;
    }
    let mut start = end;
    while start > 0 && is_identifier_byte(bytes[start - 1]) {
        start -= 1;
    }
    std::str::from_utf8(&bytes[start..end]) == Ok(word)
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$')
}

fn previous_nonspace_byte(before: &str) -> Option<u8> {
    before
        .bytes()
        .rev()
        .find(|byte| !byte.is_ascii_whitespace())
}

fn js_regex_literal_end(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'/') || matches!(bytes.get(start + 1), Some(b'/' | b'*') | None) {
        return None;
    }
    let mut index = start + 1;
    let mut in_class = false;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => {
                index = index.saturating_add(2);
                continue;
            }
            b'[' => in_class = true,
            b']' => in_class = false,
            b'/' if !in_class => {
                index += 1;
                while bytes
                    .get(index)
                    .map(|byte| byte.is_ascii_alphabetic())
                    .unwrap_or(false)
                {
                    index += 1;
                }
                return Some(index);
            }
            _ => {}
        }
        index += 1;
    }
    None
}

fn extract_local_bindings(text: &str, ext: &str) -> BTreeSet<String> {
    if !matches!(
        ext,
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte"
    ) {
        return BTreeSet::new();
    }
    let cleaned = code_without_comments_or_strings(text, ext);
    let mut out = BTreeSet::new();
    for cap in js_function_params_re().captures_iter(&cleaned) {
        if let Some(params) = cap.name("params") {
            collect_js_param_bindings(params.as_str(), &mut out);
        }
    }
    for cap in js_arrow_params_re().captures_iter(&cleaned) {
        if let Some(params) = cap.name("params") {
            collect_js_param_bindings(params.as_str(), &mut out);
        }
    }
    for cap in js_method_params_re().captures_iter(&cleaned) {
        if cap
            .name("name")
            .map(|name| language_keyword(name.as_str()))
            .unwrap_or(true)
        {
            continue;
        }
        if let Some(params) = cap.name("params") {
            collect_js_param_bindings(params.as_str(), &mut out);
        }
    }
    for cap in js_single_arrow_param_re().captures_iter(&cleaned) {
        if let Some(param) = cap.name("param") {
            let name = param.as_str();
            if !language_keyword(name) {
                out.insert(name.to_string());
            }
        }
    }
    for cap in js_for_binding_re().captures_iter(&cleaned) {
        if let Some(binding) = cap.name("binding") {
            collect_js_param_bindings(binding.as_str(), &mut out);
        }
    }
    for cap in js_catch_param_re().captures_iter(&cleaned) {
        if let Some(param) = cap.name("param") {
            collect_js_param_bindings(param.as_str(), &mut out);
        }
    }
    for pattern in js_destructuring_binding_patterns(&cleaned) {
        collect_js_param_bindings(pattern, &mut out);
    }
    out
}

fn js_destructuring_binding_patterns(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for keyword in ["const", "let", "var"] {
        for start in js_keyword_positions(text, keyword) {
            let pattern_start = skip_ascii_whitespace(text, start + keyword.len());
            let Some(open) = text.as_bytes().get(pattern_start).copied() else {
                continue;
            };
            if !matches!(open, b'{' | b'[') {
                continue;
            }
            let Some(pattern_end) = js_balanced_pattern_end(text, pattern_start) else {
                continue;
            };
            let after = skip_ascii_whitespace(text, pattern_end + 1);
            if text.as_bytes().get(after) == Some(&b'=') {
                out.push(&text[pattern_start..=pattern_end]);
            }
        }
    }
    out
}

fn js_balanced_pattern_end(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut stack = vec![match bytes.get(start).copied()? {
        b'{' => b'}',
        b'[' => b']',
        b'(' => b')',
        _ => return None,
    }];
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'{' => stack.push(b'}'),
            b'[' => stack.push(b']'),
            b'(' => stack.push(b')'),
            b'}' | b']' | b')' => {
                if stack.pop() != Some(bytes[index]) {
                    return None;
                }
                if stack.is_empty() {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

fn collect_js_param_bindings(params: &str, out: &mut BTreeSet<String>) {
    for ident in identifier_re().find_iter(params).map(|m| m.as_str()) {
        if !language_keyword(ident) {
            out.insert(ident.to_string());
        }
    }
}

#[derive(Debug, Clone)]
struct JsStaticImport {
    spec: String,
    clause: Option<String>,
    is_type: bool,
}

fn extract_js_import_specs(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for import in extract_js_static_imports(text) {
        out.insert(import.spec);
    }
    out.extend(extract_js_export_from_specs(text));
    out.extend(extract_js_call_import_specs(text));
    out
}

fn extract_js_import_bindings(text: &str) -> ImportBindingsBySpec {
    let mut out = BTreeMap::new();
    for import in extract_js_static_imports(text) {
        if import.is_type {
            continue;
        }
        let Some(clause) = import.clause.as_deref() else {
            continue;
        };
        let bindings = parse_js_import_clause_bindings(clause);
        if !bindings.is_empty() {
            out.entry(import.spec)
                .or_insert_with(BTreeMap::new)
                .extend(bindings);
        }
    }
    out
}

fn extract_js_static_imports(text: &str) -> Vec<JsStaticImport> {
    js_keyword_positions(text, "import")
        .into_iter()
        .filter_map(|start| {
            let after = skip_ascii_whitespace(text, start + "import".len());
            let next = text.as_bytes().get(after).copied();
            if matches!(next, Some(b'.' | b'(')) {
                return None;
            }
            parse_js_static_import_statement(js_statement_slice(text, start))
        })
        .collect()
}

fn parse_js_static_import_statement(statement: &str) -> Option<JsStaticImport> {
    let cap = js_static_import_statement_re().captures(statement)?;
    let spec = cap.name("spec")?.as_str().trim().to_string();
    let clause = cap.name("clause").map(|m| m.as_str().trim().to_string());
    Some(JsStaticImport {
        spec,
        clause,
        is_type: cap.name("type").is_some(),
    })
}

fn extract_js_export_from_specs(text: &str) -> BTreeSet<String> {
    js_keyword_positions(text, "export")
        .into_iter()
        .filter_map(|start| {
            js_export_from_re()
                .captures(js_statement_slice(text, start))
                .and_then(|cap| cap.name("spec").map(|m| m.as_str().trim().to_string()))
        })
        .collect()
}

fn extract_js_call_import_specs(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for start in js_keyword_positions(text, "import") {
        let after = skip_ascii_whitespace(text, start + "import".len());
        if text.as_bytes().get(after) == Some(&b'(')
            && let Some(spec) = parse_js_call_string_arg(text, after)
        {
            out.insert(spec);
        }
    }
    for start in js_keyword_positions(text, "require") {
        let after = skip_ascii_whitespace(text, start + "require".len());
        if text.as_bytes().get(after) == Some(&b'(')
            && let Some(spec) = parse_js_call_string_arg(text, after)
        {
            out.insert(spec);
        }
    }
    out
}

fn parse_js_call_string_arg(text: &str, open_paren: usize) -> Option<String> {
    let quote = skip_ascii_whitespace(text, open_paren + 1);
    let quote_byte = *text.as_bytes().get(quote)?;
    if !matches!(quote_byte, b'\'' | b'"') {
        return None;
    }
    read_js_quoted_string(text, quote)
}

fn read_js_quoted_string(text: &str, quote: usize) -> Option<String> {
    let bytes = text.as_bytes();
    let quote_byte = *bytes.get(quote)?;
    if !matches!(quote_byte, b'\'' | b'"') {
        return None;
    }
    let mut index = quote + 1;
    let mut out = String::new();
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'\\' {
            index = index.saturating_add(2);
            continue;
        }
        if byte == quote_byte {
            return Some(out);
        }
        out.push(byte as char);
        index += 1;
    }
    None
}

fn js_statement_slice(text: &str, start: usize) -> &str {
    let bytes = text.as_bytes();
    let mut index = start;
    let mut state = JsScanState::Code;
    while index < bytes.len() {
        match state {
            JsScanState::Code => {
                if bytes[index] == b';' {
                    return &text[start..=index];
                }
                if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
                    state = JsScanState::LineComment;
                    index += 2;
                    continue;
                }
                if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
                    state = JsScanState::BlockComment;
                    index += 2;
                    continue;
                }
                if matches!(bytes[index], b'\'' | b'"') {
                    state = JsScanState::Quoted(bytes[index]);
                } else if bytes[index] == b'`' {
                    state = JsScanState::Template;
                }
            }
            JsScanState::LineComment => {
                if bytes[index] == b'\n' {
                    state = JsScanState::Code;
                }
            }
            JsScanState::BlockComment => {
                if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                    state = JsScanState::Code;
                    index += 2;
                    continue;
                }
            }
            JsScanState::Quoted(quote) => {
                if bytes[index] == b'\\' {
                    index = index.saturating_add(2);
                    continue;
                }
                if bytes[index] == quote {
                    state = JsScanState::Code;
                }
            }
            JsScanState::Template => {
                if bytes[index] == b'\\' {
                    index = index.saturating_add(2);
                    continue;
                }
                if bytes[index] == b'`' {
                    state = JsScanState::Code;
                }
            }
        }
        index += 1;
    }
    &text[start..]
}

#[derive(Clone, Copy)]
enum JsScanState {
    Code,
    LineComment,
    BlockComment,
    Quoted(u8),
    Template,
}

fn js_keyword_positions(text: &str, keyword: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    let keyword_bytes = keyword.as_bytes();
    let mut out = Vec::new();
    let mut index = 0;
    let mut state = JsScanState::Code;
    while index < bytes.len() {
        match state {
            JsScanState::Code => {
                if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
                    state = JsScanState::LineComment;
                    index += 2;
                    continue;
                }
                if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
                    state = JsScanState::BlockComment;
                    index += 2;
                    continue;
                }
                if matches!(bytes[index], b'\'' | b'"') {
                    state = JsScanState::Quoted(bytes[index]);
                } else if bytes[index] == b'`' {
                    state = JsScanState::Template;
                } else if bytes[index..].starts_with(keyword_bytes)
                    && js_keyword_boundary(bytes, index, keyword_bytes.len())
                {
                    out.push(index);
                    index += keyword_bytes.len();
                    continue;
                }
            }
            JsScanState::LineComment => {
                if bytes[index] == b'\n' {
                    state = JsScanState::Code;
                }
            }
            JsScanState::BlockComment => {
                if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                    state = JsScanState::Code;
                    index += 2;
                    continue;
                }
            }
            JsScanState::Quoted(quote) => {
                if bytes[index] == b'\\' {
                    index = index.saturating_add(2);
                    continue;
                }
                if bytes[index] == quote {
                    state = JsScanState::Code;
                }
            }
            JsScanState::Template => {
                if bytes[index] == b'\\' {
                    index = index.saturating_add(2);
                    continue;
                }
                if bytes[index] == b'`' {
                    state = JsScanState::Code;
                }
            }
        }
        index += 1;
    }
    out
}

fn js_keyword_boundary(bytes: &[u8], start: usize, len: usize) -> bool {
    let before = start
        .checked_sub(1)
        .and_then(|index| bytes.get(index))
        .copied();
    let after = bytes.get(start + len).copied();
    !before.map(is_js_identifier_byte).unwrap_or(false)
        && !after.map(is_js_identifier_byte).unwrap_or(false)
}

fn is_js_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$')
}

fn skip_ascii_whitespace(text: &str, mut index: usize) -> usize {
    let bytes = text.as_bytes();
    while bytes
        .get(index)
        .map(|byte| byte.is_ascii_whitespace())
        .unwrap_or(false)
    {
        index += 1;
    }
    index
}

fn parse_js_import_clause_bindings(clause: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let clause = clause.trim();
    if clause.is_empty() || clause.starts_with('{') {
        collect_js_named_import_bindings(clause, &mut out);
        return out;
    }
    if let Some(namespace) = clause.strip_prefix("* as ") {
        if let Some(name) = first_identifier(namespace) {
            out.insert(name, "*".to_string());
        }
        return out;
    }
    if let Some((default, rest)) = clause.split_once(',') {
        if let Some(name) = first_identifier(default) {
            out.insert(name, "default".to_string());
        }
        collect_js_named_import_bindings(rest, &mut out);
    } else if let Some(name) = first_identifier(clause) {
        out.insert(name, "default".to_string());
    }
    out
}

fn collect_js_named_import_bindings(clause: &str, out: &mut BTreeMap<String, String>) {
    let Some(start) = clause.find('{') else {
        return;
    };
    let Some(end) = clause.rfind('}') else {
        return;
    };
    for part in clause[start + 1..end].split(',') {
        let part = part.trim();
        if part.is_empty() || part.starts_with("type ") {
            continue;
        }
        let (imported, local) = part
            .split_once(" as ")
            .map(|(imported, alias)| (imported.trim(), alias.trim()))
            .unwrap_or((part, part));
        let Some(imported_name) = first_identifier(imported) else {
            continue;
        };
        if let Some(local_name) = first_identifier(local) {
            out.insert(local_name, imported_name);
        }
    }
}

fn first_identifier(value: &str) -> Option<String> {
    identifier_re()
        .find(value.trim())
        .map(|m| m.as_str().to_string())
}

fn identifier_is_selector_tail(text: &str, start: usize) -> bool {
    text[..start]
        .chars()
        .rev()
        .find(|ch| !ch.is_whitespace())
        .map(|ch| ch == '.')
        .unwrap_or(false)
}

fn code_without_comments_or_strings(text: &str, ext: &str) -> String {
    let mut out = String::new();
    let mut code_state = CodeStripState::default();
    for raw_line in text.lines() {
        let comment_stripped = match ext {
            "py" => strip_python_comment_from_line(raw_line),
            "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte" | "rs" | "go"
            | "swift" => strip_c_like_code_line_for_identifier_refs(raw_line, &mut code_state),
            _ => raw_line.to_string(),
        };
        if ext == "py" {
            out.push_str(&strip_string_literals_from_line(&comment_stripped));
        } else {
            out.push_str(&comment_stripped);
        }
        out.push('\n');
    }
    out
}

#[derive(Debug, Default)]
struct CodeStripState {
    in_block_comment: bool,
    quote: Option<char>,
    escaped: bool,
}

fn strip_c_like_code_line_for_identifier_refs(line: &str, state: &mut CodeStripState) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::new();
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        let next = chars.get(index + 1).copied();
        if state.in_block_comment {
            if ch == '*' && next == Some('/') {
                state.in_block_comment = false;
                index += 2;
            } else {
                index += 1;
            }
            out.push(' ');
            continue;
        }
        if let Some(active_quote) = state.quote {
            if state.escaped {
                state.escaped = false;
            } else if ch == '\\' && active_quote != '`' {
                state.escaped = true;
            } else if ch == active_quote {
                state.quote = None;
            }
            out.push(' ');
            index += 1;
            continue;
        }
        if ch == '/' && next == Some('/') {
            break;
        }
        if ch == '/' && next == Some('*') {
            state.in_block_comment = true;
            out.push(' ');
            out.push(' ');
            index += 2;
            continue;
        }
        if matches!(ch, '"' | '\'' | '`') {
            state.quote = Some(ch);
            state.escaped = false;
            out.push(' ');
            index += 1;
            continue;
        }
        out.push(ch);
        index += 1;
    }
    out
}

fn strip_python_comment_from_line(line: &str) -> String {
    let mut out = String::new();
    let mut quote = None;
    let mut escaped = false;
    for ch in line.chars() {
        if let Some(active_quote) = quote {
            out.push(ch);
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == active_quote {
                quote = None;
            }
            continue;
        }
        if ch == '#' {
            break;
        }
        if matches!(ch, '"' | '\'') {
            quote = Some(ch);
            escaped = false;
        }
        out.push(ch);
    }
    out
}

fn strip_string_literals_from_line(line: &str) -> String {
    let mut out = String::new();
    let mut quote = None;
    let mut escaped = false;
    for ch in line.chars() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active_quote {
                quote = None;
            }
            out.push(' ');
            continue;
        }
        if matches!(ch, '"' | '\'' | '`') {
            quote = Some(ch);
            escaped = false;
            out.push(' ');
            continue;
        }
        out.push(ch);
    }
    out
}

fn language_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "async"
            | "await"
            | "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "crate"
            | "def"
            | "defer"
            | "else"
            | "enum"
            | "export"
            | "extends"
            | "false"
            | "fn"
            | "for"
            | "from"
            | "func"
            | "function"
            | "if"
            | "impl"
            | "import"
            | "in"
            | "internal"
            | "interface"
            | "let"
            | "match"
            | "mod"
            | "mut"
            | "nil"
            | "none"
            | "null"
            | "package"
            | "private"
            | "protocol"
            | "pub"
            | "public"
            | "return"
            | "self"
            | "static"
            | "struct"
            | "super"
            | "switch"
            | "this"
            | "trait"
            | "true"
            | "type"
            | "undefined"
            | "use"
            | "var"
            | "where"
            | "while"
    )
}

#[derive(Debug, Default)]
struct SurfaceExtraction {
    tokens: BTreeSet<String>,
    phrases: BTreeSet<String>,
    visited_routes: BTreeSet<String>,
}

fn extract_surfaces(text: &str, ext: &str) -> SurfaceExtraction {
    if !matches!(
        ext,
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte"
    ) {
        return SurfaceExtraction::default();
    }
    let mut surfaces = SurfaceExtraction::default();
    let mut in_block_comment = false;
    let mut jsx_visible_text_context = 0usize;
    for raw_line in text.lines() {
        let line = strip_js_comments_from_line(raw_line, &mut in_block_comment);
        let has_surface_context = line_has_surface_context(&line);
        if (jsx_visible_text_context > 0 || line_has_jsx_surface_container(&line))
            && let Some(text) = static_jsx_visible_text(&line)
        {
            surfaces
                .phrases
                .extend(surface_literal_phrases(&text, true));
        }
        if line_has_jsx_surface_container(&line) {
            jsx_visible_text_context = 4;
        } else {
            jsx_visible_text_context = jsx_visible_text_context.saturating_sub(1);
        }
        if !has_surface_context {
            continue;
        }
        let plain_label_context = line_accepts_plain_label_surface(&line);
        for quoted in quoted_strings(&line) {
            if quoted_value_is_module_specifier_context(&quoted.prefix) {
                continue;
            }
            let value = quoted.value;
            if quoted_prefix_is_page_goto_argument(&quoted.prefix)
                && let Some(route) = normalize_route_path(&value)
            {
                surfaces.visited_routes.insert(route);
            }
            let structural_literal = surface_literal_is_structural(&value)
                || (plain_label_context && surface_label_literal_is_structural(&value));
            if !structural_literal {
                continue;
            }
            surfaces.tokens.extend(surface_literal_terms(&value));
            surfaces
                .phrases
                .extend(surface_literal_phrases(&value, plain_label_context));
        }
    }
    surfaces
}

fn quoted_prefix_is_page_goto_argument(prefix: &str) -> bool {
    let lower = prefix.to_ascii_lowercase();
    let Some(index) = lower.rfind("page.goto") else {
        return false;
    };
    if lower[..index]
        .chars()
        .next_back()
        .map(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$')
        .unwrap_or(false)
    {
        return false;
    }
    let tail = lower[index + "page.goto".len()..].trim_start();
    let Some(argument_prefix) = tail.strip_prefix('(') else {
        return false;
    };
    !argument_prefix.contains(')') && argument_prefix.trim().is_empty()
}

fn normalize_route_path(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if !trimmed.starts_with('/') || trimmed.starts_with("//") || trimmed.contains("${") {
        return None;
    }
    let path = trimmed
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .trim_end_matches('/');
    let path = if path.is_empty() { "/" } else { path };
    if path
        .chars()
        .any(|ch| ch.is_whitespace() || matches!(ch, '"' | '\'' | '`'))
    {
        return None;
    }
    Some(path.to_string())
}

fn line_has_surface_context(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    [
        "classname",
        "class=",
        "contentclassname",
        "data-testid",
        "data-test",
        "aria-",
        "locator(",
        "getbytestid",
        "getbyrole",
        "getbylabel",
        "getbytext",
        "queryselector",
        "tocontaintext",
        "tohavetext",
        "getattribute(",
        "setattribute(",
        "page.goto",
        "tohaveurl",
        "href=",
        "mode=",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn strip_js_comments_from_line(line: &str, in_block_comment: &mut bool) -> String {
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let mut out = String::new();
    let mut index = 0;
    let mut quote = None;
    let mut escaped = false;
    while index < chars.len() {
        let (_, ch) = chars[index];
        let next = chars.get(index + 1).map(|(_, next)| *next);
        if *in_block_comment {
            if ch == '*' && next == Some('/') {
                *in_block_comment = false;
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }
        if let Some(active_quote) = quote {
            out.push(ch);
            index += 1;
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == active_quote {
                quote = None;
            }
            continue;
        }
        if ch == '/' && next == Some('/') {
            break;
        }
        if ch == '/' && next == Some('*') {
            *in_block_comment = true;
            index += 2;
            continue;
        }
        if matches!(ch, '"' | '\'' | '`') {
            quote = Some(ch);
            escaped = false;
        }
        out.push(ch);
        index += 1;
    }
    out
}

fn line_accepts_plain_label_surface(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    [
        "aria-label",
        "getbylabel",
        "getbyrole",
        "getbytext",
        "tocontaintext",
        "tohavetext",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn line_has_jsx_surface_container(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    line.contains('<')
        && [
            "classname",
            "class=",
            "data-testid",
            "data-test",
            "aria-",
            "role=",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
}

fn static_jsx_visible_text(line: &str) -> Option<String> {
    let mut out = String::new();
    let mut in_tag = false;
    let mut brace_depth = 0usize;
    for ch in line.chars() {
        if in_tag {
            if ch == '>' {
                in_tag = false;
                out.push(' ');
            }
            continue;
        }
        if brace_depth > 0 {
            if ch == '{' {
                brace_depth += 1;
            } else if ch == '}' {
                brace_depth = brace_depth.saturating_sub(1);
                if brace_depth == 0 {
                    out.push(' ');
                }
            }
            continue;
        }
        if ch == '<' {
            in_tag = true;
            out.push(' ');
            continue;
        }
        if ch == '{' {
            brace_depth = 1;
            out.push(' ');
            continue;
        }
        out.push(ch);
    }
    let text = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.len() < 3
        || text.len() > 180
        || !text.chars().any(|ch| ch.is_alphabetic())
        || text.contains('=')
    {
        return None;
    }
    Some(text)
}

fn quoted_value_is_module_specifier_context(prefix: &str) -> bool {
    let lower = strip_trailing_js_comments(&prefix.to_ascii_lowercase());
    let trimmed = lower.trim_end();
    if token_ends_with(trimmed, "from") || token_ends_with(trimmed, "import") {
        return true;
    }
    if let Some(before_call) = trimmed.strip_suffix('(') {
        let before_call = before_call.trim_end();
        return token_ends_with(before_call, "import") || token_ends_with(before_call, "require");
    }
    token_ends_with(trimmed, "require")
}

fn strip_trailing_js_comments(value: &str) -> String {
    let mut out = value.trim_end().to_string();
    loop {
        let trimmed = out.trim_end();
        if !trimmed.ends_with("*/") {
            return trimmed.to_string();
        }
        let Some(start) = trimmed.rfind("/*") else {
            return trimmed.to_string();
        };
        out.truncate(start);
    }
}

fn token_ends_with(value: &str, token: &str) -> bool {
    let Some(before) = value.strip_suffix(token) else {
        return false;
    };
    before
        .chars()
        .next_back()
        .map(|ch| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'))
        .unwrap_or(true)
}

#[derive(Debug)]
struct QuotedString {
    value: String,
    prefix: String,
}

fn quoted_strings(text: &str) -> Vec<QuotedString> {
    let mut values = Vec::new();
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut index = 0;
    let mut in_block_comment = false;
    while index < chars.len() {
        let (start, ch) = chars[index];
        let next = chars.get(index + 1).map(|(_, next)| *next);
        if in_block_comment {
            if ch == '*' && next == Some('/') {
                in_block_comment = false;
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }
        if ch == '/' && next == Some('/') {
            break;
        }
        if ch == '/' && next == Some('*') {
            in_block_comment = true;
            index += 2;
            continue;
        }
        if !matches!(ch, '"' | '\'' | '`') {
            index += 1;
            continue;
        }
        let quote = ch;
        let mut value = String::new();
        let mut escaped = false;
        index += 1;
        while index < chars.len() {
            let (_, inner) = chars[index];
            index += 1;
            if escaped {
                value.push(inner);
                escaped = false;
                continue;
            }
            if inner == '\\' {
                escaped = true;
                continue;
            }
            if inner == quote {
                break;
            }
            value.push(inner);
        }
        values.push(QuotedString {
            value,
            prefix: text[..start].to_string(),
        });
    }
    values
}

fn surface_literal_is_structural(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.len() < 3 || trimmed.len() > 160 {
        return false;
    }
    if surface_literal_is_module_specifier(trimmed) {
        return false;
    }
    trimmed.starts_with('.')
        || trimmed.starts_with('#')
        || trimmed.starts_with('/')
        || trimmed.contains("data-testid")
        || trimmed.contains("data-test")
        || trimmed.contains("aria-")
        || trimmed.contains('-')
        || trimmed.contains('_')
}

fn surface_literal_is_module_specifier(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.starts_with("./")
        || trimmed.starts_with("../")
        || trimmed.starts_with("@/")
        || (trimmed.starts_with('@') && trimmed.contains('/'))
        || (trimmed.contains('/')
            && !trimmed.starts_with('/')
            && !trimmed.starts_with('.')
            && !trimmed.starts_with('#')
            && !trimmed.contains(char::is_whitespace))
}

fn surface_label_literal_is_structural(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.len() < 3 || trimmed.len() > 100 {
        return false;
    }
    if surface_literal_is_module_specifier(trimmed) {
        return false;
    }
    let terms = surface_phrase_terms(&normalize_surface_phrase(trimmed).unwrap_or_default());
    !terms.is_empty()
        && terms
            .iter()
            .all(|term| term.chars().all(|ch| ch.is_alphanumeric()))
}

fn surface_literal_phrases(value: &str, preserve_whole: bool) -> BTreeSet<String> {
    let route_surface = value.trim().starts_with('/');
    if preserve_whole
        && let Some(phrase) = normalize_surface_phrase(value)
        && (surface_phrase_is_specific(&phrase)
            || (route_surface && surface_phrase_terms(&phrase).len() >= 2))
    {
        return BTreeSet::from([phrase]);
    }
    value
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '>' | '+' | '~' | ',' | '[' | ']'))
        .filter_map(normalize_surface_phrase)
        .filter(|phrase| {
            surface_phrase_is_specific(phrase)
                || (route_surface && surface_phrase_terms(phrase).len() >= 2)
        })
        .collect()
}

fn normalize_surface_phrase(value: &str) -> Option<String> {
    let mut trimmed = value
        .trim()
        .trim_matches(|ch: char| {
            matches!(
                ch,
                '.' | '#' | '"' | '\'' | '`' | '(' | ')' | '{' | '}' | ';'
            )
        })
        .replace("__", "-")
        .replace(['.', '#', '/', '_', ':', '(', ')'], "-");
    trimmed = trimmed.split_whitespace().collect::<Vec<_>>().join("-");
    while trimmed.contains("--") {
        trimmed = trimmed.replace("--", "-");
    }
    let trimmed = trimmed.trim_matches('-').to_lowercase();
    if trimmed.is_empty()
        || trimmed.contains("${")
        || trimmed.starts_with("http")
        || trimmed.starts_with("mailto")
    {
        return None;
    }
    Some(trimmed)
}

fn surface_phrase_is_specific(phrase: &str) -> bool {
    let terms = surface_phrase_terms(phrase);
    terms.len() >= 2
        && terms
            .iter()
            .any(|term| !matches!(term.as_str(), "frame" | "title" | "canvas" | "node"))
}

fn surface_phrase_terms(phrase: &str) -> BTreeSet<String> {
    surface_terms(&phrase.replace(['.', '#', '/', '-', '_', ':'], " "))
        .into_iter()
        .filter(|term| term.len() >= 3)
        .filter(|term| {
            !matches!(
                term.as_str(),
                "the"
                    | "and"
                    | "for"
                    | "with"
                    | "from"
                    | "true"
                    | "false"
                    | "null"
                    | "undefined"
                    | "data"
                    | "test"
                    | "testid"
                    | "aria"
                    | "label"
                    | "role"
                    | "root"
                    | "blueprint"
                    | "nodrag"
                    | "nopan"
            )
        })
        .collect()
}

fn surface_literal_terms(value: &str) -> BTreeSet<String> {
    surface_terms(&value.replace(['.', '#', '/', '-', '_', ':'], " "))
        .into_iter()
        .filter(|term| term.len() >= 3)
        .filter(|term| {
            !matches!(
                term.as_str(),
                "the"
                    | "and"
                    | "for"
                    | "with"
                    | "from"
                    | "true"
                    | "false"
                    | "null"
                    | "undefined"
                    | "data"
                    | "test"
                    | "testid"
                    | "aria"
                    | "label"
                    | "role"
                    | "button"
                    | "link"
                    | "input"
                    | "text"
                    | "page"
                    | "root"
                    | "blueprint"
            )
        })
        .collect()
}

fn surface_terms(value: &str) -> BTreeSet<String> {
    value
        .split(|ch: char| !(ch.is_alphanumeric() || ch == '_'))
        .map(str::to_lowercase)
        .filter(|term| term.len() >= 2)
        .collect()
}

fn symbols_with_ranges(mut starts: Vec<SymbolStart>, text: &str, ext: &str) -> Vec<SymbolInfo> {
    starts.sort_by(|a, b| {
        a.line_start
            .cmp(&b.line_start)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.kind.cmp(&b.kind))
    });
    starts
        .iter()
        .enumerate()
        .map(|(idx, symbol)| {
            let fallback_end = starts
                .iter()
                .skip(idx + 1)
                .find(|next| next.indent <= symbol.indent)
                .and_then(|next| next.line_start.checked_sub(1))
                .unwrap_or_else(|| line_count(text))
                .max(symbol.line_start);
            SymbolInfo {
                name: symbol.name.clone(),
                kind: symbol.kind.clone(),
                exported: symbol.exported,
                line_start: symbol.line_start,
                line_end: symbol_end(text, ext, symbol.line_start, fallback_end),
            }
        })
        .collect()
}

fn symbol_end(text: &str, ext: &str, line_start: usize, fallback_end: usize) -> usize {
    match ext {
        "py" => python_symbol_end(text, line_start, fallback_end),
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte" | "rs" | "go" | "swift" => {
            brace_symbol_end(text, line_start, fallback_end).unwrap_or(line_start)
        }
        _ => fallback_end,
    }
}

fn brace_symbol_end(text: &str, line_start: usize, scan_end: usize) -> Option<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let mut depth: isize = 0;
    let mut saw_open = false;
    for (idx, line) in lines
        .iter()
        .enumerate()
        .skip(line_start.saturating_sub(1))
        .take(scan_end.saturating_sub(line_start).saturating_add(1))
    {
        if !saw_open && line.trim_end().ends_with(';') {
            return Some(idx + 1);
        }
        for ch in line.chars() {
            match ch {
                '{' => {
                    saw_open = true;
                    depth += 1;
                }
                '}' if saw_open => {
                    depth -= 1;
                    if depth <= 0 {
                        return Some(idx + 1);
                    }
                }
                _ => {}
            }
        }
        if !saw_open {
            let trimmed = line.trim();
            if trimmed.ends_with("=> null") || trimmed.ends_with("=> undefined") {
                return Some(idx + 1);
            }
        }
    }
    None
}

fn python_symbol_end(text: &str, line_start: usize, fallback_end: usize) -> usize {
    let lines: Vec<&str> = text.lines().collect();
    let Some(start_line) = lines.get(line_start.saturating_sub(1)) else {
        return fallback_end;
    };
    let base_indent = leading_spaces(start_line);
    let mut last_non_blank = line_start;
    for (idx, line) in lines.iter().enumerate().skip(line_start) {
        if line.trim().is_empty() {
            continue;
        }
        let line_no = idx + 1;
        let indent = leading_spaces(line);
        if indent <= base_indent {
            return last_non_blank.max(line_start);
        }
        last_non_blank = line_no;
    }
    last_non_blank.max(line_start)
}

fn leading_spaces(line: &str) -> usize {
    line.chars().take_while(|ch| *ch == ' ').count()
}

fn extract_js_symbols(text: &str) -> Vec<SymbolStart> {
    let mut symbols = Vec::new();
    let mut import_export_block_depth = 0usize;
    for (idx, line) in text.lines().enumerate() {
        if is_noise_line(line, "//") {
            continue;
        }
        if js_import_export_block_line(line, &mut import_export_block_depth) {
            continue;
        }
        let line_start = idx + 1;
        if let Some(cap) = js_default_symbol_re().captures(line) {
            let raw_kind = cap.name("kind").map(|m| m.as_str()).unwrap_or("default");
            let name = cap
                .name("name")
                .map(|m| m.as_str())
                .unwrap_or("default")
                .to_string();
            symbols.push(SymbolStart {
                kind: js_symbol_kind(raw_kind, &name, true),
                name,
                exported: true,
                line_start,
                indent: leading_spaces(line),
            });
            continue;
        }
        if let Some(cap) = js_symbol_re().captures(line) {
            let raw_kind = cap.name("kind").map(|m| m.as_str()).unwrap_or("symbol");
            let Some(name) = cap.name("name").map(|m| m.as_str().to_string()) else {
                continue;
            };
            let exported = cap.name("export").is_some();
            symbols.push(SymbolStart {
                kind: js_symbol_kind(raw_kind, &name, exported),
                name,
                exported,
                line_start,
                indent: leading_spaces(line),
            });
        }
    }
    symbols
}

fn js_import_export_block_line(line: &str, depth: &mut usize) -> bool {
    let trimmed = line.trim_start();
    let starts_block = trimmed.starts_with("import {")
        || trimmed.starts_with("import type {")
        || trimmed.starts_with("export {")
        || trimmed.starts_with("export type {");
    if !starts_block && *depth == 0 {
        return false;
    }
    let opens = trimmed.matches('{').count();
    let closes = trimmed.matches('}').count();
    *depth = depth.saturating_add(opens).saturating_sub(closes);
    true
}

fn js_symbol_kind(raw_kind: &str, name: &str, exported: bool) -> String {
    if is_hook_name(name) && matches!(raw_kind, "function" | "const" | "let" | "var") {
        return "hook".to_string();
    }
    if exported
        && is_uppercase_symbol(name)
        && matches!(raw_kind, "function" | "const" | "let" | "var")
    {
        return "component".to_string();
    }
    match raw_kind {
        "let" | "var" => "variable",
        other => other,
    }
    .to_string()
}

fn extract_rust_symbols(text: &str) -> Vec<SymbolStart> {
    let mut symbols = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        if is_noise_line(line, "//") {
            continue;
        }
        let line_start = idx + 1;
        if let Some(cap) = rust_symbol_re().captures(line) {
            let Some(name) = cap.name("name").map(|m| m.as_str().to_string()) else {
                continue;
            };
            let raw_kind = cap.name("kind").map(|m| m.as_str()).unwrap_or("symbol");
            symbols.push(SymbolStart {
                name,
                kind: rust_symbol_kind(raw_kind).to_string(),
                exported: cap.name("pub").is_some(),
                line_start,
                indent: leading_spaces(line),
            });
            continue;
        }
        if let Some(cap) = rust_impl_re().captures(line) {
            let Some(name) = cap.name("name").map(|m| m.as_str().trim().to_string()) else {
                continue;
            };
            symbols.push(SymbolStart {
                name,
                kind: "impl".to_string(),
                exported: false,
                line_start,
                indent: leading_spaces(line),
            });
        }
    }
    symbols
}

fn rust_symbol_kind(raw_kind: &str) -> &str {
    match raw_kind {
        "fn" => "function",
        "mod" => "module",
        other => other,
    }
}

fn extract_python_symbols(text: &str) -> Vec<SymbolStart> {
    let mut symbols = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        if is_noise_line(line, "#") {
            continue;
        }
        let Some(cap) = python_symbol_re().captures(line) else {
            continue;
        };
        let Some(name) = cap.name("name").map(|m| m.as_str().to_string()) else {
            continue;
        };
        let raw_kind = cap.name("kind").map(|m| m.as_str()).unwrap_or("def");
        symbols.push(SymbolStart {
            name,
            kind: if raw_kind == "class" {
                "class".to_string()
            } else {
                "function".to_string()
            },
            exported: false,
            line_start: idx + 1,
            indent: leading_spaces(line),
        });
    }
    symbols
}

fn extract_go_symbols(text: &str) -> Vec<SymbolStart> {
    let mut symbols = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        if is_noise_line(line, "//") {
            continue;
        }
        let line_start = idx + 1;
        if let Some(cap) = go_func_symbol_re().captures(line) {
            let Some(name) = cap.name("name").map(|m| m.as_str().to_string()) else {
                continue;
            };
            symbols.push(SymbolStart {
                kind: if cap.name("receiver").is_some() {
                    "method".to_string()
                } else {
                    "function".to_string()
                },
                exported: is_uppercase_symbol(&name),
                name,
                line_start,
                indent: leading_spaces(line),
            });
            continue;
        }
        if let Some(cap) = go_type_symbol_re().captures(line) {
            let Some(name) = cap.name("name").map(|m| m.as_str().to_string()) else {
                continue;
            };
            let raw_kind = cap.name("kind").map(|m| m.as_str()).unwrap_or("type");
            let kind = match raw_kind {
                "struct" => "struct",
                "interface" => "interface",
                _ => "type",
            };
            symbols.push(SymbolStart {
                name: name.clone(),
                kind: kind.to_string(),
                exported: is_uppercase_symbol(&name),
                line_start,
                indent: leading_spaces(line),
            });
        }
    }
    symbols
}

fn extract_swift_symbols(text: &str) -> Vec<SymbolStart> {
    let mut symbols = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        if is_noise_line(line, "//") {
            continue;
        }
        let line_start = idx + 1;
        if let Some(cap) = swift_type_symbol_re().captures(line) {
            let Some(name) = cap.name("name").map(|m| m.as_str().to_string()) else {
                continue;
            };
            let raw_kind = cap.name("kind").map(|m| m.as_str()).unwrap_or("symbol");
            let modifiers = cap.name("mods").map(|m| m.as_str()).unwrap_or_default();
            symbols.push(SymbolStart {
                name,
                kind: raw_kind.to_string(),
                exported: swift_modifiers_are_exported(modifiers),
                line_start,
                indent: leading_spaces(line),
            });
            continue;
        }
        if let Some(cap) = swift_func_symbol_re().captures(line) {
            let Some(name) = cap.name("name").map(|m| m.as_str().to_string()) else {
                continue;
            };
            let modifiers = cap.name("mods").map(|m| m.as_str()).unwrap_or_default();
            symbols.push(SymbolStart {
                name,
                kind: "function".to_string(),
                exported: swift_modifiers_are_exported(modifiers),
                line_start,
                indent: leading_spaces(line),
            });
            continue;
        }
        if let Some(cap) = swift_property_symbol_re().captures(line) {
            let Some(name) = cap.name("name").map(|m| m.as_str().to_string()) else {
                continue;
            };
            let raw_kind = cap.name("kind").map(|m| m.as_str()).unwrap_or("var");
            let modifiers = cap.name("mods").map(|m| m.as_str()).unwrap_or_default();
            symbols.push(SymbolStart {
                name,
                kind: if raw_kind == "let" {
                    "constant".to_string()
                } else {
                    "property".to_string()
                },
                exported: swift_modifiers_are_exported(modifiers),
                line_start,
                indent: leading_spaces(line),
            });
        }
    }
    symbols
}

fn swift_modifiers_are_exported(modifiers: &str) -> bool {
    modifiers
        .split_whitespace()
        .any(|modifier| matches!(modifier, "public" | "open" | "package"))
}

fn is_noise_line(line: &str, comment_prefix: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with(comment_prefix) || trimmed.starts_with('*')
}

fn is_hook_name(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("use") else {
        return false;
    };
    rest.chars()
        .next()
        .map(|ch| ch.is_ascii_uppercase())
        .unwrap_or(false)
}

fn is_uppercase_symbol(name: &str) -> bool {
    name.chars()
        .next()
        .map(|ch| ch.is_ascii_uppercase())
        .unwrap_or(false)
}

struct ImportResolutionSeed {
    rel: String,
    ext: String,
    imports: Vec<String>,
    import_bindings: ImportBindingsBySpec,
}

fn resolve_imports(
    root: &Path,
    files: &mut BTreeMap<String, FileInfo>,
    packages: &[PackageInfo],
    ts_path_aliases: &[TsPathAlias],
) {
    let paths: BTreeSet<String> = files.keys().cloned().collect();
    let snapshot: Vec<ImportResolutionSeed> = files
        .values()
        .map(|f| ImportResolutionSeed {
            rel: f.rel.clone(),
            ext: f.ext.clone(),
            imports: f.imports.iter().cloned().collect(),
            import_bindings: f.import_bindings.clone(),
        })
        .collect();
    for seed in snapshot {
        let mut resolved = BTreeSet::new();
        let mut resolved_bindings = BTreeMap::new();
        for spec in seed.imports {
            if let Some(target) = resolve_import(
                root,
                &seed.rel,
                &seed.ext,
                &spec,
                &paths,
                packages,
                ts_path_aliases,
            ) {
                if let Some(bindings) = seed.import_bindings.get(&spec) {
                    resolved_bindings
                        .entry(target.clone())
                        .or_insert_with(BTreeMap::new)
                        .extend(
                            bindings
                                .iter()
                                .map(|(local, imported)| (local.clone(), imported.clone())),
                        );
                }
                resolved.insert(target);
            }
        }
        if let Some(info) = files.get_mut(&seed.rel) {
            info.resolved_imports = resolved;
            info.resolved_import_bindings = resolved_bindings;
        }
    }
}

fn resolve_import(
    root: &Path,
    from: &str,
    ext: &str,
    spec: &str,
    paths: &BTreeSet<String>,
    packages: &[PackageInfo],
    ts_path_aliases: &[TsPathAlias],
) -> Option<String> {
    if spec.starts_with('.') && ext != "py" {
        return resolve_relative(from, spec, paths);
    }
    match ext {
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte" => {
            resolve_javascript(root, from, spec, paths, packages, ts_path_aliases)
        }
        "py" => resolve_python(from, spec, paths, packages),
        "rs" => resolve_rust(from, spec, paths),
        "go" => resolve_go(spec, paths, packages),
        _ => None,
    }
}

fn resolve_relative(from: &str, spec: &str, paths: &BTreeSet<String>) -> Option<String> {
    let base_dir = Path::new(from)
        .parent()
        .map(|p| normalize_rel_path(&p.to_string_lossy()))
        .unwrap_or_default();
    let base = normalize_rel_path(&format!("{base_dir}/{spec}"));
    resolve_path_like(&base, paths)
}

fn resolve_path_like(base: &str, paths: &BTreeSet<String>) -> Option<String> {
    let base = normalize_rel_path(base);
    let mut candidates = vec![base.clone()];
    for ext in [
        "ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "rs", "go", "vue", "svelte",
    ] {
        candidates.push(format!("{base}.{ext}"));
    }
    for index in [
        "index.ts",
        "index.tsx",
        "index.js",
        "index.jsx",
        "__init__.py",
        "mod.rs",
    ] {
        candidates.push(normalize_rel_path(&format!("{base}/{index}")));
    }
    candidates.into_iter().find(|c| paths.contains(c))
}

fn resolve_javascript(
    root: &Path,
    from: &str,
    spec: &str,
    paths: &BTreeSet<String>,
    packages: &[PackageInfo],
    ts_path_aliases: &[TsPathAlias],
) -> Option<String> {
    let mut aliases = ts_path_aliases
        .iter()
        .filter(|alias| ts_alias_applies_to_importer(alias, from))
        .collect::<Vec<_>>();
    aliases.sort_by(|a, b| {
        b.config_dir
            .len()
            .cmp(&a.config_dir.len())
            .then_with(|| b.pattern.len().cmp(&a.pattern.len()))
            .then_with(|| a.pattern.cmp(&b.pattern))
    });
    for alias in aliases {
        if let Some(target) = resolve_ts_path_alias(alias, spec, paths) {
            return Some(target);
        }
    }
    let (package_name, subpath) = split_package_spec(spec)?;
    let package = packages
        .iter()
        .find(|package| package.ecosystem == "javascript" && package.name == package_name)?;
    if subpath.is_empty() {
        for entry in js_package_root_entrypoints(root, package) {
            if let Some(target) = resolve_path_like(&entry, paths) {
                return Some(target);
            }
        }
        return None;
    }
    let (exports_declared, exported_subpaths) =
        js_package_subpath_entrypoints(root, package, &subpath);
    for entry in exported_subpaths {
        if let Some(target) = resolve_path_like(&entry, paths) {
            return Some(target);
        }
    }
    if exports_declared {
        return None;
    }
    for base in [
        format!("{}/{}", package.path, subpath),
        format!("{}/src/{}", package.path, subpath),
    ] {
        if let Some(target) = resolve_path_like(&base, paths) {
            return Some(target);
        }
    }
    None
}

fn split_package_spec(spec: &str) -> Option<(String, String)> {
    if spec.is_empty() || spec.starts_with('.') || spec.starts_with('/') {
        return None;
    }
    let parts = spec.split('/').collect::<Vec<_>>();
    if parts.first()?.starts_with('@') {
        if parts.len() < 2 {
            return None;
        }
        let name = format!("{}/{}", parts[0], parts[1]);
        let rest = parts.iter().skip(2).copied().collect::<Vec<_>>().join("/");
        Some((name, rest))
    } else {
        let name = parts[0].to_string();
        let rest = parts.iter().skip(1).copied().collect::<Vec<_>>().join("/");
        Some((name, rest))
    }
}

fn js_package_root_entrypoints(root: &Path, package: &PackageInfo) -> Vec<String> {
    let mut entries = Vec::new();
    let mut exports_declared = false;
    if let Ok(text) = fs::read_to_string(root.join(&package.manifest))
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
    {
        if let Some(exports) = value.get("exports") {
            exports_declared = true;
            collect_js_root_export_targets(exports, &mut entries);
        }
        if !exports_declared {
            for key in ["module", "main", "types", "typings"] {
                if let Some(value) = value.get(key).and_then(|value| value.as_str()) {
                    entries.push(value.to_string());
                }
            }
        }
    }
    if !exports_declared {
        entries.extend([
            "src/index.ts".to_string(),
            "src/index.tsx".to_string(),
            "src/index.js".to_string(),
            "index.ts".to_string(),
            "index.tsx".to_string(),
            "index.js".to_string(),
            "src/lib.ts".to_string(),
            "lib/index.ts".to_string(),
        ]);
    }
    normalize_package_entries(package, entries)
}

fn js_package_subpath_entrypoints(
    root: &Path,
    package: &PackageInfo,
    subpath: &str,
) -> (bool, Vec<String>) {
    let Ok(text) = fs::read_to_string(root.join(&package.manifest)) else {
        return (false, Vec::new());
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return (false, Vec::new());
    };
    let Some(exports) = value.get("exports") else {
        return (false, Vec::new());
    };
    let mut entries = Vec::new();
    let key = format!("./{}", subpath.trim_start_matches("./"));
    if let Some(map) = exports.as_object() {
        if let Some(target) = map.get(&key) {
            collect_js_export_targets(target, None, &mut entries);
        } else {
            for (pattern, target) in map {
                let Some(wildcard) = match_pattern_wildcard(pattern, &key).flatten() else {
                    continue;
                };
                collect_js_export_targets(target, Some(&wildcard), &mut entries);
            }
        }
    }
    (true, normalize_package_entries(package, entries))
}

fn normalize_package_entries(package: &PackageInfo, entries: Vec<String>) -> Vec<String> {
    unique_strings(
        entries
            .into_iter()
            .map(|entry| {
                let entry = entry.trim().trim_start_matches("./");
                normalize_rel_path(&format!("{}/{}", package.path, entry))
            })
            .collect(),
    )
}

fn collect_js_root_export_targets(value: &serde_json::Value, out: &mut Vec<String>) {
    if value.as_str().is_some() {
        collect_js_export_targets(value, None, out);
        return;
    }
    let Some(map) = value.as_object() else {
        return;
    };
    if let Some(root) = map.get(".") {
        collect_js_export_targets(root, None, out);
        return;
    }
    for key in ["import", "require", "default", "types", "module"] {
        if let Some(value) = map.get(key) {
            collect_js_export_targets(value, None, out);
        }
    }
}

fn collect_js_export_targets(
    value: &serde_json::Value,
    wildcard: Option<&str>,
    out: &mut Vec<String>,
) {
    if let Some(raw) = value.as_str() {
        out.push(match wildcard {
            Some(wildcard) => raw.replace('*', wildcard),
            None => raw.to_string(),
        });
        return;
    }
    let Some(map) = value.as_object() else {
        return;
    };
    if let Some(root) = map.get(".") {
        collect_js_export_targets(root, wildcard, out);
        return;
    }
    for key in ["import", "require", "default", "types", "module"] {
        if let Some(value) = map.get(key) {
            collect_js_export_targets(value, wildcard, out);
        }
    }
}

#[derive(Debug, Clone)]
struct TsPathAlias {
    config_dir: String,
    pattern: String,
    targets: Vec<String>,
}

fn detect_ts_path_aliases(root: &Path, files: &BTreeMap<String, FileInfo>) -> Vec<TsPathAlias> {
    let mut aliases = Vec::new();
    for rel in files.keys() {
        if Path::new(rel).file_name().and_then(|name| name.to_str()) != Some("tsconfig.json") {
            continue;
        }
        aliases.extend(read_ts_path_aliases(root, rel));
    }
    aliases.sort_by(|a, b| {
        b.pattern
            .len()
            .cmp(&a.pattern.len())
            .then_with(|| a.pattern.cmp(&b.pattern))
    });
    aliases
}

fn read_ts_path_aliases(root: &Path, rel: &str) -> Vec<TsPathAlias> {
    let Ok(text) = fs::read_to_string(root.join(rel)) else {
        return Vec::new();
    };
    let Ok(value) = parse_tsconfig_json(&text) else {
        return Vec::new();
    };
    let Some(options) = value
        .get("compilerOptions")
        .and_then(|value| value.as_object())
    else {
        return Vec::new();
    };
    let config_dir = manifest_dir(rel);
    let base_url = options
        .get("baseUrl")
        .and_then(|value| value.as_str())
        .unwrap_or(".");
    let base = normalize_rel_path(&format!("{config_dir}/{base_url}"));
    let Some(paths) = options.get("paths").and_then(|value| value.as_object()) else {
        return Vec::new();
    };
    let mut aliases = Vec::new();
    for (pattern, targets) in paths {
        let Some(targets) = targets.as_array() else {
            continue;
        };
        let targets = targets
            .iter()
            .filter_map(|target| target.as_str())
            .map(|target| normalize_rel_path(&format!("{base}/{target}")))
            .collect::<Vec<_>>();
        if !targets.is_empty() {
            aliases.push(TsPathAlias {
                config_dir: config_dir.clone(),
                pattern: pattern.to_string(),
                targets,
            });
        }
    }
    aliases
}

fn parse_tsconfig_json(text: &str) -> std::result::Result<serde_json::Value, serde_json::Error> {
    serde_json::from_str(text).or_else(|strict_error| {
        let Some(json) = strip_jsonc_comments_and_trailing_commas(text) else {
            return Err(strict_error);
        };
        serde_json::from_str(&json)
    })
}

fn strip_jsonc_comments_and_trailing_commas(text: &str) -> Option<String> {
    Some(strip_json_trailing_commas(&strip_jsonc_comments(text)?))
}

fn strip_jsonc_comments(text: &str) -> Option<String> {
    let chars = text.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let mut in_string = false;
    let mut escape = false;

    while i < chars.len() {
        let ch = chars[i];
        if in_string {
            out.push(ch);
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if ch == '"' {
            in_string = true;
            out.push(ch);
            i += 1;
            continue;
        }

        if ch == '/' && chars.get(i + 1) == Some(&'/') {
            i += 2;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            if i < chars.len() {
                out.push('\n');
                i += 1;
            }
            continue;
        }

        if ch == '/' && chars.get(i + 1) == Some(&'*') {
            i += 2;
            let mut closed = false;
            while i + 1 < chars.len() {
                if chars[i] == '\n' {
                    out.push('\n');
                }
                if chars[i] == '*' && chars[i + 1] == '/' {
                    i += 2;
                    closed = true;
                    break;
                }
                i += 1;
            }
            if !closed {
                return None;
            }
            continue;
        }

        out.push(ch);
        i += 1;
    }

    Some(out)
}

fn strip_json_trailing_commas(text: &str) -> String {
    let mut out = Vec::with_capacity(text.len());
    let mut in_string = false;
    let mut escape = false;

    for ch in text.chars() {
        if in_string {
            out.push(ch);
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            out.push(ch);
            continue;
        }

        if matches!(ch, '}' | ']') {
            let mut index = out.len();
            while index > 0 && out[index - 1].is_whitespace() {
                index -= 1;
            }
            if index > 0 && out[index - 1] == ',' {
                out.remove(index - 1);
            }
        }

        out.push(ch);
    }

    out.into_iter().collect()
}

fn ts_alias_applies_to_importer(alias: &TsPathAlias, from: &str) -> bool {
    alias.config_dir == "."
        || from == alias.config_dir
        || from.starts_with(&format!("{}/", alias.config_dir.trim_end_matches('/')))
}

fn resolve_ts_path_alias(
    alias: &TsPathAlias,
    spec: &str,
    paths: &BTreeSet<String>,
) -> Option<String> {
    let wildcard = match_pattern_wildcard(&alias.pattern, spec)?;
    for target in &alias.targets {
        let base = if let Some(wildcard) = wildcard.as_deref() {
            target.replace('*', wildcard)
        } else {
            target.clone()
        };
        if let Some(resolved) = resolve_path_like(&base, paths) {
            return Some(resolved);
        }
    }
    None
}

fn match_pattern_wildcard(pattern: &str, value: &str) -> Option<Option<String>> {
    if !pattern.contains('*') {
        return (pattern == value).then_some(None);
    }
    let (prefix, suffix) = pattern.split_once('*')?;
    if !value.starts_with(prefix) || !value.ends_with(suffix) {
        return None;
    }
    let end = value.len().saturating_sub(suffix.len());
    Some(Some(value[prefix.len()..end].to_string()))
}

fn resolve_python(
    from: &str,
    spec: &str,
    paths: &BTreeSet<String>,
    packages: &[PackageInfo],
) -> Option<String> {
    if spec.starts_with('.') {
        return resolve_python_relative(from, spec, paths);
    }
    let base = spec.replace('.', "/");
    for candidate in [format!("{base}.py"), format!("{base}/__init__.py")]
        .into_iter()
        .chain([
            format!("src/{base}.py"),
            format!("src/{base}/__init__.py"),
            format!("app/{base}.py"),
            format!("app/{base}/__init__.py"),
        ])
    {
        if paths.contains(&candidate) {
            return Some(candidate);
        }
    }
    for package in packages
        .iter()
        .filter(|package| package.ecosystem == "python")
    {
        for candidate in [
            format!("{}/{base}.py", package.path),
            format!("{}/{base}/__init__.py", package.path),
            format!("{}/src/{base}.py", package.path),
            format!("{}/src/{base}/__init__.py", package.path),
        ] {
            let candidate = normalize_rel_path(&candidate);
            if paths.contains(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

fn resolve_python_relative(from: &str, spec: &str, paths: &BTreeSet<String>) -> Option<String> {
    let level = spec.chars().take_while(|ch| *ch == '.').count();
    let rest = spec.trim_start_matches('.');
    let mut dir = Path::new(from).parent().unwrap_or_else(|| Path::new("."));
    for _ in 1..level {
        dir = dir.parent().unwrap_or_else(|| Path::new("."));
    }
    let rest = rest.replace('.', "/");
    let base = if rest.is_empty() {
        normalize_rel_path(&dir.to_string_lossy())
    } else {
        normalize_rel_path(&format!("{}/{}", dir.to_string_lossy(), rest))
    };
    resolve_path_like(&base, paths)
}

fn resolve_rust(from: &str, spec: &str, paths: &BTreeSet<String>) -> Option<String> {
    let raw = spec
        .strip_prefix("crate::")
        .map(|s| format!("src/{}", s.replace("::", "/")))
        .unwrap_or_else(|| spec.replace("::", "/"));
    let base_dir = Path::new(from)
        .parent()
        .map(|p| normalize_rel_path(&p.to_string_lossy()))
        .unwrap_or_default();
    [
        format!("{raw}.rs"),
        format!("{raw}/mod.rs"),
        format!("{base_dir}/{raw}.rs"),
        format!("{base_dir}/{raw}/mod.rs"),
    ]
    .into_iter()
    .map(|p| normalize_rel_path(&p))
    .find(|c| paths.contains(c))
}

fn resolve_go(spec: &str, paths: &BTreeSet<String>, packages: &[PackageInfo]) -> Option<String> {
    let package = packages
        .iter()
        .filter(|package| package.ecosystem == "go")
        .filter(|package| spec == package.name || spec.starts_with(&format!("{}/", package.name)))
        .max_by_key(|package| package.name.len())?;
    let subpath = spec
        .strip_prefix(&package.name)
        .unwrap_or_default()
        .trim_start_matches('/');
    let base = if subpath.is_empty() {
        package.path.clone()
    } else {
        normalize_rel_path(&format!("{}/{}", package.path, subpath))
    };
    resolve_go_package_dir(&base, paths)
}

fn resolve_go_package_dir(base: &str, paths: &BTreeSet<String>) -> Option<String> {
    let base = normalize_rel_path(base);
    let basename = Path::new(&base)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("main");
    for candidate in [
        format!("{base}/{basename}.go"),
        format!("{base}/main.go"),
        format!("{base}/lib.go"),
    ] {
        let candidate = normalize_rel_path(&candidate);
        if paths.contains(&candidate) {
            return Some(candidate);
        }
    }
    let prefix = if base == "." {
        String::new()
    } else {
        format!("{}/", base.trim_end_matches('/'))
    };
    paths
        .iter()
        .find(|path| {
            path.starts_with(&prefix)
                && path.ends_with(".go")
                && !path.ends_with("_test.go")
                && Path::new(path)
                    .parent()
                    .map(|parent| normalize_rel_path(&parent.to_string_lossy()) == base)
                    .unwrap_or(base == ".")
        })
        .cloned()
}

fn build_reverse_imports(files: &BTreeMap<String, FileInfo>) -> BTreeMap<String, BTreeSet<String>> {
    let mut reverse: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for file in files.values() {
        for target in &file.resolved_imports {
            reverse
                .entry(target.clone())
                .or_default()
                .insert(file.rel.clone());
            if target.ends_with(".go") {
                for package_file in go_package_files(files, target) {
                    reverse
                        .entry(package_file)
                        .or_default()
                        .insert(file.rel.clone());
                }
            }
        }
    }
    reverse
}

fn go_package_files(files: &BTreeMap<String, FileInfo>, target: &str) -> Vec<String> {
    let package_dir = Path::new(target)
        .parent()
        .map(|parent| normalize_rel_path(&parent.to_string_lossy()))
        .unwrap_or_else(|| ".".to_string());
    files
        .values()
        .filter(|file| {
            file.ext == "go"
                && !file.rel.ends_with("_test.go")
                && Path::new(&file.rel)
                    .parent()
                    .map(|parent| normalize_rel_path(&parent.to_string_lossy()) == package_dir)
                    .unwrap_or(package_dir == ".")
        })
        .map(|file| file.rel.clone())
        .collect()
}

fn detect_packages(root: &Path, files: &BTreeMap<String, FileInfo>) -> Vec<PackageInfo> {
    let mut packages = Vec::new();
    for rel in files.keys() {
        let name = Path::new(rel).file_name().and_then(|s| s.to_str());
        match name {
            Some("package.json") => {
                if let Some(package) = read_js_package(root, rel) {
                    packages.push(package);
                }
            }
            Some("Cargo.toml") => {
                if let Some(package) = read_cargo_package(root, rel) {
                    packages.push(package);
                }
            }
            Some("go.mod") => {
                if let Some(package) = read_go_package(root, rel) {
                    packages.push(package);
                }
            }
            Some("pyproject.toml") => {
                if let Some(package) = read_python_package(root, rel) {
                    packages.push(package);
                }
            }
            Some("Package.swift") => {
                if let Some(package) = read_swift_package(root, rel) {
                    packages.push(package);
                }
            }
            _ => {}
        }
    }
    packages.sort_by(|a, b| a.path.cmp(&b.path).then_with(|| a.name.cmp(&b.name)));
    packages
}

fn read_js_package(root: &Path, rel: &str) -> Option<PackageInfo> {
    let text = fs::read_to_string(root.join(rel)).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&text).ok()?;
    let path = manifest_dir(rel);
    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| package_name_from_path(&path));
    Some(PackageInfo {
        name,
        path,
        manifest: rel.to_string(),
        ecosystem: "javascript".to_string(),
    })
}

fn read_cargo_package(root: &Path, rel: &str) -> Option<PackageInfo> {
    let text = fs::read_to_string(root.join(rel)).ok()?;
    let name = cargo_package_name(&text)?;
    Some(PackageInfo {
        name,
        path: manifest_dir(rel),
        manifest: rel.to_string(),
        ecosystem: "rust".to_string(),
    })
}

fn read_go_package(root: &Path, rel: &str) -> Option<PackageInfo> {
    let text = fs::read_to_string(root.join(rel)).ok()?;
    let name = go_module_name(&text)?;
    Some(PackageInfo {
        name,
        path: manifest_dir(rel),
        manifest: rel.to_string(),
        ecosystem: "go".to_string(),
    })
}

fn read_python_package(root: &Path, rel: &str) -> Option<PackageInfo> {
    let text = fs::read_to_string(root.join(rel)).ok()?;
    let path = manifest_dir(rel);
    let name = pyproject_package_name(&text).unwrap_or_else(|| package_name_from_path(&path));
    Some(PackageInfo {
        name,
        path,
        manifest: rel.to_string(),
        ecosystem: "python".to_string(),
    })
}

fn read_swift_package(root: &Path, rel: &str) -> Option<PackageInfo> {
    let text = fs::read_to_string(root.join(rel)).ok()?;
    let path = manifest_dir(rel);
    let name = swift_package_name(&text).unwrap_or_else(|| package_name_from_path(&path));
    Some(PackageInfo {
        name,
        path,
        manifest: rel.to_string(),
        ecosystem: "swift".to_string(),
    })
}

fn detect_package_edges(
    root: &Path,
    files: &BTreeMap<String, FileInfo>,
    packages: &[PackageInfo],
) -> Vec<PackageDependency> {
    let mut edges = Vec::new();
    let by_name: BTreeMap<String, &PackageInfo> = packages
        .iter()
        .map(|package| (package.name.clone(), package))
        .collect();
    let by_path: BTreeMap<String, &PackageInfo> = packages
        .iter()
        .map(|package| (package.path.clone(), package))
        .collect();
    let cargo_workspaces = cargo_workspace_infos(root, files, packages, &by_path);

    for package in packages {
        match package.ecosystem.as_str() {
            "javascript" => {
                edges.extend(js_package_edges(root, package, &by_name, &by_path));
            }
            "rust" => {
                edges.extend(cargo_package_edges(
                    root,
                    package,
                    &by_path,
                    &cargo_workspaces,
                ));
            }
            "go" => {
                edges.extend(go_package_edges(root, package, &by_name, &by_path));
            }
            "python" => {
                edges.extend(python_package_edges(root, package, &by_path));
            }
            "swift" => {
                edges.extend(swift_package_edges(root, package, &by_path));
            }
            _ => {}
        }
    }
    edges.sort_by(|a, b| {
        a.from
            .cmp(&b.from)
            .then_with(|| a.to.cmp(&b.to))
            .then_with(|| a.dependency.cmp(&b.dependency))
    });
    edges.dedup_by(|a, b| {
        a.from == b.from && a.to == b.to && a.dependency == b.dependency && a.source == b.source
    });
    edges
}

fn js_package_edges(
    root: &Path,
    package: &PackageInfo,
    by_name: &BTreeMap<String, &PackageInfo>,
    by_path: &BTreeMap<String, &PackageInfo>,
) -> Vec<PackageDependency> {
    let Ok(text) = fs::read_to_string(root.join(&package.manifest)) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let base = Path::new(&package.manifest)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut edges = Vec::new();
    for section in [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ] {
        let Some(map) = value.get(section).and_then(|v| v.as_object()) else {
            continue;
        };
        for (dep, spec) in map {
            if let Some(spec) = spec.as_str() {
                if let Some(path) = js_local_dependency_path(spec) {
                    if let Some(target_path) = resolve_repo_relative_path(base, &path)
                        && let Some(target) = by_path.get(&target_path)
                    {
                        edges.push(PackageDependency {
                            from: package.path.clone(),
                            from_manifest: package.manifest.clone(),
                            to: target.path.clone(),
                            to_manifest: Some(target.manifest.clone()),
                            workspace_manifest: None,
                            dependency: dep.clone(),
                            source: format!("package.json {section} local path"),
                        });
                    }
                    continue;
                }
                if js_dependency_spec_is_local_protocol(spec) {
                    continue;
                }
            }
            if let Some(target) = by_name.get(dep) {
                edges.push(PackageDependency {
                    from: package.path.clone(),
                    from_manifest: package.manifest.clone(),
                    to: target.path.clone(),
                    to_manifest: Some(target.manifest.clone()),
                    workspace_manifest: None,
                    dependency: dep.clone(),
                    source: format!("package.json {section}"),
                });
            }
        }
    }
    edges
}

fn js_local_dependency_path(spec: &str) -> Option<String> {
    let spec = spec.trim();
    for prefix in ["file:", "link:", "portal:", "workspace:"] {
        if let Some(path) = spec.strip_prefix(prefix) {
            let path = path.trim();
            if path.starts_with("./") || path.starts_with("../") || path == "." || path == ".." {
                return Some(path.to_string());
            }
        }
    }
    None
}

fn js_dependency_spec_is_local_protocol(spec: &str) -> bool {
    let spec = spec.trim();
    if ["file:", "link:", "portal:"]
        .iter()
        .any(|prefix| spec.starts_with(prefix))
    {
        return true;
    }
    let Some(path) = spec.strip_prefix("workspace:") else {
        return false;
    };
    let path = path.trim().replace('\\', "/");
    path.starts_with("./")
        || path.starts_with("../")
        || path == "."
        || path == ".."
        || path_is_absolute_like(&path)
}

fn cargo_package_edges(
    root: &Path,
    package: &PackageInfo,
    by_path: &BTreeMap<String, &PackageInfo>,
    workspaces: &[CargoWorkspaceInfo],
) -> Vec<PackageDependency> {
    let Ok(text) = fs::read_to_string(root.join(&package.manifest)) else {
        return Vec::new();
    };
    let base = Path::new(&package.manifest)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut edges: Vec<PackageDependency> = cargo_path_dependencies(&text)
        .into_iter()
        .filter_map(|(name, path)| {
            let target_path = resolve_repo_relative_path(base, &path)?;
            let target = by_path.get(&target_path)?;
            Some(PackageDependency {
                from: package.path.clone(),
                from_manifest: package.manifest.clone(),
                to: target.path.clone(),
                to_manifest: Some(target.manifest.clone()),
                workspace_manifest: None,
                dependency: name,
                source: "Cargo.toml path dependency".to_string(),
            })
        })
        .collect();
    let workspace = cargo_workspace_for_package(package, workspaces);
    let workspace_dependencies = workspace
        .map(|workspace| &workspace.dependencies)
        .cloned()
        .unwrap_or_default();
    let workspace_manifest = workspace.map(|workspace| workspace.manifest.clone());
    for name in cargo_workspace_dependency_names(&text) {
        let Some(path) = workspace_dependencies.get(&name) else {
            continue;
        };
        let Some(target) = by_path.get(path) else {
            continue;
        };
        edges.push(PackageDependency {
            from: package.path.clone(),
            from_manifest: package.manifest.clone(),
            to: target.path.clone(),
            to_manifest: Some(target.manifest.clone()),
            workspace_manifest: workspace_manifest.clone(),
            dependency: name,
            source: "Cargo.toml workspace dependency".to_string(),
        });
    }
    edges
}

#[derive(Debug, Clone)]
struct CargoWorkspaceInfo {
    manifest: String,
    path: String,
    dependencies: BTreeMap<String, String>,
    members: Vec<String>,
    exclude: Vec<String>,
    member_paths: BTreeSet<String>,
}

fn cargo_workspace_infos(
    root: &Path,
    files: &BTreeMap<String, FileInfo>,
    packages: &[PackageInfo],
    by_path: &BTreeMap<String, &PackageInfo>,
) -> Vec<CargoWorkspaceInfo> {
    let mut workspaces = Vec::new();
    for rel in files.keys() {
        if Path::new(rel).file_name().and_then(|name| name.to_str()) != Some("Cargo.toml") {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(rel)) else {
            continue;
        };
        if !cargo_workspace_declared(&text) {
            continue;
        }
        let path = manifest_dir(rel);
        let dependencies = cargo_workspace_path_dependencies(&text)
            .into_iter()
            .filter_map(|(name, dependency_path)| {
                resolve_repo_relative_path(Path::new(&path), &dependency_path)
                    .map(|resolved| (name, resolved))
            })
            .collect();
        workspaces.push(CargoWorkspaceInfo {
            manifest: rel.clone(),
            path,
            dependencies,
            members: cargo_workspace_array_values(&text, "members"),
            exclude: cargo_workspace_array_values(&text, "exclude"),
            member_paths: BTreeSet::new(),
        });
    }
    workspaces.sort_by(|a, b| a.path.cmp(&b.path));
    for workspace in &mut workspaces {
        workspace.member_paths = cargo_workspace_member_paths(root, workspace, packages, by_path);
    }
    workspaces
}

fn cargo_workspace_for_package<'a>(
    package: &PackageInfo,
    workspaces: &'a [CargoWorkspaceInfo],
) -> Option<&'a CargoWorkspaceInfo> {
    workspaces
        .iter()
        .filter(|workspace| cargo_workspace_contains_package(workspace, &package.path))
        .max_by_key(|workspace| workspace.path.len())
}

fn cargo_workspace_contains_package(workspace: &CargoWorkspaceInfo, package_path: &str) -> bool {
    workspace.member_paths.contains(package_path)
}

fn cargo_workspace_member_paths(
    root: &Path,
    workspace: &CargoWorkspaceInfo,
    packages: &[PackageInfo],
    by_path: &BTreeMap<String, &PackageInfo>,
) -> BTreeSet<String> {
    let mut members: BTreeSet<String> = packages
        .iter()
        .filter(|package| cargo_workspace_explicitly_contains_package(workspace, &package.path))
        .map(|package| package.path.clone())
        .collect();
    let mut changed = true;
    while changed {
        changed = false;
        for package_path in members.iter().cloned().collect::<Vec<_>>() {
            let Some(package) = by_path.get(&package_path) else {
                continue;
            };
            let Ok(text) = fs::read_to_string(root.join(&package.manifest)) else {
                continue;
            };
            let base = Path::new(&package.manifest)
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            for (_, path) in cargo_path_dependencies(&text) {
                let Some(target_path) = resolve_repo_relative_path(base, &path) else {
                    continue;
                };
                if by_path.contains_key(&target_path)
                    && cargo_workspace_path_allowed(workspace, &target_path)
                    && members.insert(target_path)
                {
                    changed = true;
                }
            }
            for name in cargo_workspace_dependency_names(&text) {
                let Some(target_path) = workspace.dependencies.get(&name) else {
                    continue;
                };
                if by_path.contains_key(target_path)
                    && cargo_workspace_path_allowed(workspace, target_path)
                    && members.insert(target_path.clone())
                {
                    changed = true;
                }
            }
        }
    }
    members
}

fn cargo_workspace_explicitly_contains_package(
    workspace: &CargoWorkspaceInfo,
    package_path: &str,
) -> bool {
    let rel = match path_relative_to(package_path, &workspace.path) {
        Some(rel) => rel,
        None => return false,
    };
    if !cargo_workspace_rel_allowed(workspace, &rel) {
        return false;
    }
    if rel == "." {
        return true;
    }
    workspace
        .members
        .iter()
        .any(|pattern| cargo_workspace_member_pattern_matches(&rel, pattern))
}

fn cargo_workspace_path_allowed(workspace: &CargoWorkspaceInfo, package_path: &str) -> bool {
    path_relative_to(package_path, &workspace.path)
        .map(|rel| cargo_workspace_rel_allowed(workspace, &rel))
        .unwrap_or(false)
}

fn cargo_workspace_rel_allowed(workspace: &CargoWorkspaceInfo, rel: &str) -> bool {
    !workspace
        .exclude
        .iter()
        .any(|pattern| cargo_workspace_member_pattern_matches(rel, pattern))
}

fn path_relative_to(path: &str, base: &str) -> Option<String> {
    let path = normalize_rel_path(path);
    let base = normalize_rel_path(base);
    if base == "." {
        return Some(path);
    }
    if path == base {
        return Some(".".to_string());
    }
    let prefix = format!("{}/", base.trim_end_matches('/'));
    path.strip_prefix(&prefix).map(str::to_string)
}

fn cargo_workspace_member_pattern_matches(rel: &str, pattern: &str) -> bool {
    let rel = normalize_rel_path(rel.trim().trim_start_matches("./"));
    let Some(pattern) = cargo_normalize_workspace_member_pattern(pattern) else {
        return false;
    };
    if pattern == "." {
        return rel == ".";
    }
    let mut builder = GlobBuilder::new(&pattern);
    builder.literal_separator(true);
    builder
        .build()
        .map(|glob| glob.compile_matcher().is_match(&rel))
        .unwrap_or(rel == pattern)
}

fn resolve_repo_relative_path(base: &Path, path: &str) -> Option<String> {
    let raw = path.trim().replace('\\', "/");
    if raw.is_empty() || path_is_absolute_like(&raw) {
        return None;
    }
    let base = normalize_rel_path(&base.to_string_lossy());
    let mut parts: Vec<String> = if base == "." {
        Vec::new()
    } else {
        base.split('/').map(str::to_string).collect()
    };
    for part in raw.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other.to_string()),
        }
    }
    Some(if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    })
}

fn cargo_normalize_workspace_member_pattern(pattern: &str) -> Option<String> {
    let raw = pattern.trim().trim_start_matches("./").replace('\\', "/");
    if raw.is_empty() || path_is_absolute_like(&raw) {
        return None;
    }
    let mut parts: Vec<String> = Vec::new();
    for part in raw.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other.to_string()),
        }
    }
    Some(if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    })
}

fn path_is_absolute_like(path: &str) -> bool {
    path.starts_with('/')
        || path.starts_with("//")
        || path
            .split('/')
            .next()
            .is_some_and(|part| part.ends_with(':'))
}

fn go_package_edges(
    root: &Path,
    package: &PackageInfo,
    by_name: &BTreeMap<String, &PackageInfo>,
    by_path: &BTreeMap<String, &PackageInfo>,
) -> Vec<PackageDependency> {
    let Ok(text) = fs::read_to_string(root.join(&package.manifest)) else {
        return Vec::new();
    };
    let base = Path::new(&package.manifest)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let replaces = go_replaces(&text);
    let mut edges = Vec::new();
    for dep in go_requires(&text) {
        if let Some(replacement) = replaces.get(&dep) {
            if let Some(target) = by_name.get(replacement) {
                edges.push(PackageDependency {
                    from: package.path.clone(),
                    from_manifest: package.manifest.clone(),
                    to: target.path.clone(),
                    to_manifest: Some(target.manifest.clone()),
                    workspace_manifest: None,
                    dependency: dep,
                    source: "go.mod replace".to_string(),
                });
                continue;
            }
            if let Some(target_path) = resolve_repo_relative_path(base, replacement)
                && let Some(target) = by_path.get(&target_path)
            {
                edges.push(PackageDependency {
                    from: package.path.clone(),
                    from_manifest: package.manifest.clone(),
                    to: target.path.clone(),
                    to_manifest: Some(target.manifest.clone()),
                    workspace_manifest: None,
                    dependency: dep,
                    source: "go.mod local replace".to_string(),
                });
                continue;
            }
        }
        if let Some(target) = by_name.get(&dep) {
            edges.push(PackageDependency {
                from: package.path.clone(),
                from_manifest: package.manifest.clone(),
                to: target.path.clone(),
                to_manifest: Some(target.manifest.clone()),
                workspace_manifest: None,
                dependency: dep,
                source: "go.mod require".to_string(),
            });
        }
    }
    edges
}

fn python_package_edges(
    root: &Path,
    package: &PackageInfo,
    by_path: &BTreeMap<String, &PackageInfo>,
) -> Vec<PackageDependency> {
    let Ok(text) = fs::read_to_string(root.join(&package.manifest)) else {
        return Vec::new();
    };
    let base = Path::new(&package.manifest)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut edges = Vec::new();
    for (dep, path) in pyproject_path_dependencies(&text) {
        if let Some(target_path) = resolve_repo_relative_path(base, &path)
            && let Some(target) = by_path.get(&target_path)
        {
            edges.push(PackageDependency {
                from: package.path.clone(),
                from_manifest: package.manifest.clone(),
                to: target.path.clone(),
                to_manifest: Some(target.manifest.clone()),
                workspace_manifest: None,
                dependency: dep,
                source: "pyproject local path dependency".to_string(),
            });
        }
    }
    edges
}

fn swift_package_edges(
    root: &Path,
    package: &PackageInfo,
    by_path: &BTreeMap<String, &PackageInfo>,
) -> Vec<PackageDependency> {
    let Ok(text) = fs::read_to_string(root.join(&package.manifest)) else {
        return Vec::new();
    };
    let base = Path::new(&package.manifest)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut edges = Vec::new();
    for path in swift_package_path_dependencies(&text) {
        if let Some(target_path) = resolve_repo_relative_path(base, &path)
            && let Some(target) = by_path.get(&target_path)
        {
            edges.push(PackageDependency {
                from: package.path.clone(),
                from_manifest: package.manifest.clone(),
                to: target.path.clone(),
                to_manifest: Some(target.manifest.clone()),
                workspace_manifest: None,
                dependency: package_name_from_path(&target.path),
                source: "Package.swift local path dependency".to_string(),
            });
        }
    }
    edges
}

fn manifest_dir(rel: &str) -> String {
    Path::new(rel)
        .parent()
        .map(|p| normalize_rel_path(&p.to_string_lossy()))
        .filter(|p| p != ".")
        .unwrap_or_else(|| ".".to_string())
}

fn package_name_from_path(path: &str) -> String {
    if path == "." {
        "repo".to_string()
    } else {
        Path::new(path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(path)
            .to_string()
    }
}

fn cargo_package_name(text: &str) -> Option<String> {
    parse_toml_value(text)?
        .get("package")?
        .get("name")?
        .as_str()
        .map(str::to_string)
        .filter(|s| !s.is_empty())
}

fn cargo_path_dependencies(text: &str) -> Vec<(String, String)> {
    let Some(value) = parse_toml_value(text) else {
        return Vec::new();
    };
    let mut deps = Vec::new();
    for table in cargo_dependency_tables(&value) {
        deps.extend(cargo_table_path_dependencies(table));
    }
    unique_pairs(deps)
}

fn cargo_workspace_path_dependencies(text: &str) -> BTreeMap<String, String> {
    let Some(value) = parse_toml_value(text) else {
        return BTreeMap::new();
    };
    let mut deps = BTreeMap::new();
    if let Some(table) = value
        .get("workspace")
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(toml::Value::as_table)
    {
        for (name, dependency) in table {
            if let Some(path) = toml_path_field(dependency) {
                deps.insert(name.to_string(), path);
            }
        }
    }
    deps
}

fn cargo_workspace_dependency_names(text: &str) -> Vec<String> {
    let Some(value) = parse_toml_value(text) else {
        return Vec::new();
    };
    let mut deps = Vec::new();
    for table in cargo_dependency_tables(&value) {
        for (name, dependency) in table {
            if toml_workspace_field(dependency) == Some(true) {
                deps.push(name.to_string());
            }
        }
    }
    unique_strings(deps)
}

fn cargo_workspace_declared(text: &str) -> bool {
    parse_toml_value(text)
        .and_then(|value| value.get("workspace").cloned())
        .is_some()
}

fn cargo_workspace_array_values(text: &str, key: &str) -> Vec<String> {
    parse_toml_value(text)
        .and_then(|value| value.get("workspace").cloned())
        .and_then(|workspace| workspace.get(key).cloned())
        .and_then(|value| toml_string_array(&value))
        .unwrap_or_default()
}

fn parse_toml_value(text: &str) -> Option<toml::Value> {
    toml::from_str::<toml::Value>(text).ok()
}

fn cargo_dependency_tables(value: &toml::Value) -> Vec<&toml::Table> {
    let mut tables = Vec::new();
    collect_cargo_dependency_tables(value, &mut tables);
    if let Some(targets) = value.get("target").and_then(toml::Value::as_table) {
        for target in targets.values() {
            collect_cargo_dependency_tables(target, &mut tables);
        }
    }
    tables
}

fn collect_cargo_dependency_tables<'a>(value: &'a toml::Value, out: &mut Vec<&'a toml::Table>) {
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(table) = value.get(section).and_then(toml::Value::as_table) {
            out.push(table);
        }
    }
}

fn cargo_table_path_dependencies(table: &toml::Table) -> Vec<(String, String)> {
    table
        .iter()
        .filter_map(|(name, dependency)| {
            toml_path_field(dependency).map(|path| (name.to_string(), path))
        })
        .collect()
}

fn toml_path_field(value: &toml::Value) -> Option<String> {
    value
        .get("path")
        .and_then(toml::Value::as_str)
        .map(str::to_string)
        .filter(|path| !path.is_empty())
}

fn toml_workspace_field(value: &toml::Value) -> Option<bool> {
    value.get("workspace").and_then(toml::Value::as_bool)
}

fn toml_string_array(value: &toml::Value) -> Option<Vec<String>> {
    Some(
        value
            .as_array()?
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .filter(|item| !item.is_empty())
            .collect(),
    )
}

fn go_module_name(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = strip_go_mod_comment(line).trim();
        if let Some(value) = trimmed.strip_prefix("module ") {
            return value
                .split_whitespace()
                .next()
                .map(str::to_string)
                .filter(|value| !value.is_empty());
        }
    }
    None
}

fn go_requires(text: &str) -> Vec<String> {
    let mut deps = Vec::new();
    let mut in_block = false;
    for line in text.lines() {
        let trimmed = strip_go_mod_comment(line).trim();
        if trimmed.is_empty() {
            continue;
        }
        if in_block {
            if trimmed.starts_with(')') {
                in_block = false;
                continue;
            }
            if let Some(module) = trimmed.split_whitespace().next()
                && !module.is_empty()
            {
                deps.push(module.to_string());
            }
            continue;
        }
        if trimmed == "require (" {
            in_block = true;
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("require ") {
            if value.trim_start().starts_with('(') {
                in_block = true;
                continue;
            }
            if let Some(module) = value.split_whitespace().next()
                && !module.is_empty()
            {
                deps.push(module.to_string());
            }
        }
    }
    unique_strings(deps)
}

fn go_replaces(text: &str) -> BTreeMap<String, String> {
    let mut replaces = BTreeMap::new();
    let mut in_block = false;
    for line in text.lines() {
        let trimmed = strip_go_mod_comment(line).trim();
        if trimmed.is_empty() {
            continue;
        }
        if in_block {
            if trimmed.starts_with(')') {
                in_block = false;
                continue;
            }
            if let Some((from, to)) = parse_go_replace(trimmed) {
                replaces.insert(from, to);
            }
            continue;
        }
        if trimmed == "replace (" {
            in_block = true;
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("replace ") {
            if value.trim_start().starts_with('(') {
                in_block = true;
                continue;
            }
            if let Some((from, to)) = parse_go_replace(value) {
                replaces.insert(from, to);
            }
        }
    }
    replaces
}

fn parse_go_replace(value: &str) -> Option<(String, String)> {
    let (from, to) = value.split_once("=>")?;
    let from = from.split_whitespace().next()?.to_string();
    let to = to.split_whitespace().next()?.to_string();
    (!from.is_empty() && !to.is_empty()).then_some((from, to))
}

fn strip_go_mod_comment(line: &str) -> &str {
    line.split_once("//").map(|(head, _)| head).unwrap_or(line)
}

fn extract_go_imports(text: &str) -> BTreeSet<String> {
    let mut imports = BTreeSet::new();
    let mut in_block = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if in_block {
            if trimmed.starts_with(')') {
                in_block = false;
                continue;
            }
            if let Some(path) = quoted_go_import(trimmed) {
                imports.insert(path);
            }
            continue;
        }
        let Some(value) = trimmed.strip_prefix("import") else {
            continue;
        };
        let value = value.trim_start();
        if value.starts_with('(') {
            in_block = true;
            let rest = value.trim_start_matches('(').trim();
            if !rest.is_empty()
                && !rest.starts_with(')')
                && let Some(path) = quoted_go_import(rest)
            {
                imports.insert(path);
            }
            continue;
        }
        if let Some(path) = quoted_go_import(value) {
            imports.insert(path);
        }
    }
    imports
}

fn quoted_go_import(value: &str) -> Option<String> {
    let quote_start = value.find('"')?;
    let tail = &value[quote_start + 1..];
    let quote_end = tail.find('"')?;
    let path = &tail[..quote_end];
    (!path.is_empty()).then_some(path.to_string())
}

fn pyproject_package_name(text: &str) -> Option<String> {
    let value = parse_toml_value(text)?;
    value
        .get("project")
        .and_then(|project| project.get("name"))
        .or_else(|| {
            value
                .get("tool")
                .and_then(|tool| tool.get("poetry"))
                .and_then(|poetry| poetry.get("name"))
        })
        .and_then(toml::Value::as_str)
        .map(str::to_string)
        .filter(|name| !name.is_empty())
}

fn pyproject_path_dependencies(text: &str) -> Vec<(String, String)> {
    let Some(value) = parse_toml_value(text) else {
        return Vec::new();
    };
    let mut deps = Vec::new();
    if let Some(table) = value
        .get("tool")
        .and_then(|tool| tool.get("uv"))
        .and_then(|uv| uv.get("sources"))
        .and_then(toml::Value::as_table)
    {
        for (name, dependency) in table {
            if let Some(path) = toml_path_field(dependency) {
                deps.push((name.to_string(), path));
            }
        }
    }
    if let Some(table) = value
        .get("tool")
        .and_then(|tool| tool.get("poetry"))
        .and_then(|poetry| poetry.get("dependencies"))
        .and_then(toml::Value::as_table)
    {
        for (name, dependency) in table {
            if let Some(path) = toml_path_field(dependency) {
                deps.push((name.to_string(), path));
            }
        }
    }
    unique_pairs(deps)
}

fn swift_package_name(text: &str) -> Option<String> {
    swift_package_name_re()
        .captures(text)?
        .get(1)
        .map(|m| m.as_str().to_string())
        .filter(|name| !name.is_empty())
}

fn swift_package_path_dependencies(text: &str) -> Vec<String> {
    let mut deps = swift_package_path_dependency_re()
        .captures_iter(text)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect::<Vec<_>>();
    deps.sort();
    deps.dedup();
    deps
}

fn unquote(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_end_matches(',');
    trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| {
            trimmed
                .strip_prefix('\'')
                .and_then(|s| s.strip_suffix('\''))
        })
        .map(str::to_string)
}

fn detect_package_manager(root: &Path) -> String {
    if root.join("pnpm-lock.yaml").exists() || root.join("pnpm-workspace.yaml").exists() {
        "pnpm"
    } else if root.join("yarn.lock").exists() {
        "yarn"
    } else if root.join("bun.lockb").exists() {
        "bun"
    } else if root.join("package.json").exists() {
        "npm"
    } else if root.join("Cargo.toml").exists() {
        "cargo"
    } else if root.join("go.mod").exists() || root.join("go.work").exists() {
        "go"
    } else if root.join("pyproject.toml").exists() || root.join("requirements.txt").exists() {
        "python"
    } else if root.join("Package.swift").exists() {
        "swift"
    } else {
        "unknown"
    }
    .to_string()
}

fn detect_scripts(root: &Path) -> Vec<ScriptInfo> {
    let mut scripts = Vec::new();
    if root.join("package.json").exists() {
        let pm = detect_package_manager(root);
        if let Ok(text) = fs::read_to_string(root.join("package.json"))
            && let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
            && let Some(map) = value.get("scripts").and_then(|v| v.as_object())
        {
            for (name, command) in map {
                let name_l = name.to_ascii_lowercase();
                if !(name_l.contains("test")
                    || name_l.contains("type")
                    || name_l.contains("lint")
                    || name_l.contains("check"))
                {
                    continue;
                }
                let runner = if pm == "unknown" { "npm" } else { &pm };
                let invoke = if name == "test" {
                    match runner {
                        "npm" => "npm test".to_string(),
                        "yarn" => "yarn test".to_string(),
                        "bun" => "bun test".to_string(),
                        _ => format!("{runner} test"),
                    }
                } else {
                    format!("{runner} run {name}")
                };
                scripts.push(ScriptInfo {
                    name: name.clone(),
                    command: invoke,
                    reason: format!("package.json script: {}", command.as_str().unwrap_or("")),
                });
            }
        }
    }
    if root.join("Cargo.toml").exists() {
        scripts.push(ScriptInfo {
            name: "test".to_string(),
            command: "cargo test".to_string(),
            reason: "Cargo.toml detected".to_string(),
        });
    }
    if root.join("go.mod").exists() {
        scripts.push(ScriptInfo {
            name: "test".to_string(),
            command: "go test ./...".to_string(),
            reason: "go.mod detected".to_string(),
        });
    }
    if root.join("pyproject.toml").exists() || root.join("requirements.txt").exists() {
        scripts.push(ScriptInfo {
            name: "test".to_string(),
            command: "pytest".to_string(),
            reason: "Python project files detected".to_string(),
        });
    }
    if root.join("Package.swift").exists() {
        scripts.push(ScriptInfo {
            name: "test".to_string(),
            command: "swift test".to_string(),
            reason: "Package.swift detected".to_string(),
        });
    }
    if root.join("Makefile").exists() {
        scripts.push(ScriptInfo {
            name: "test".to_string(),
            command: "make test".to_string(),
            reason: "Makefile detected".to_string(),
        });
    }
    scripts.sort_by(|a, b| a.command.cmp(&b.command));
    scripts.dedup_by(|a, b| a.command == b.command);
    scripts
}

fn detect_languages(files: &BTreeMap<String, FileInfo>) -> BTreeSet<String> {
    files
        .values()
        .filter_map(|file| match file.language.as_str() {
            "unknown" | "config" | "markdown" => None,
            other => Some(other.to_string()),
        })
        .collect()
}

fn discover_domains(
    root: &Path,
    files: &BTreeMap<String, FileInfo>,
    anchors: &CtxConfig,
    config_path: Option<&str>,
) -> Vec<Domain> {
    let mut domains = BTreeMap::<String, Domain>::new();
    if let Some(domain) = &anchors.domain {
        let id = domain.id.clone().unwrap_or_else(|| "repo".to_string());
        let path = normalize_rel_path(domain.path.as_deref().unwrap_or("."));
        domains.insert(
            id.clone(),
            Domain {
                id,
                path,
                config_path: config_path.map(str::to_string),
            },
        );
    }
    for (id, domain) in &anchors.domains {
        let path = normalize_rel_path(domain.path.as_deref().unwrap_or(id));
        domains.insert(
            id.clone(),
            Domain {
                id: id.clone(),
                path,
                config_path: config_path.map(str::to_string),
            },
        );
    }

    for rel in workspace_domain_paths(root, files) {
        let id = Path::new(&rel)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&rel)
            .to_string();
        domains.entry(id.clone()).or_insert(Domain {
            id,
            path: rel,
            config_path: None,
        });
    }

    for hint in DOMAIN_HINT_DIRS {
        let base = root.join(hint);
        if !base.is_dir() {
            continue;
        }
        let Ok(children) = fs::read_dir(base) else {
            continue;
        };
        for child in children.flatten() {
            let path = child.path();
            if !path.is_dir() {
                continue;
            }
            let rel =
                normalize_rel_path(&path.strip_prefix(root).unwrap_or(&path).to_string_lossy());
            if should_ignore_rel(&rel) {
                continue;
            }
            let has_files = files
                .keys()
                .any(|file| file.starts_with(&format!("{rel}/")));
            let has_markers = [
                "src",
                "tests",
                "test",
                "package.json",
                "Cargo.toml",
                "go.mod",
                ".ctx.yml",
            ]
            .iter()
            .any(|marker| path.join(marker).exists());
            if has_files || has_markers {
                let id = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&rel)
                    .to_string();
                domains.entry(id.clone()).or_insert(Domain {
                    id,
                    path: rel,
                    config_path: None,
                });
            }
        }
    }

    if domains.is_empty() {
        let id = anchors
            .domain
            .as_ref()
            .and_then(|d| d.id.clone())
            .unwrap_or_else(|| "repo".to_string());
        domains.insert(
            id.clone(),
            Domain {
                id,
                path: ".".to_string(),
                config_path: config_path.map(str::to_string),
            },
        );
    }

    domains.into_values().collect()
}

fn workspace_domain_paths(root: &Path, files: &BTreeMap<String, FileInfo>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for pattern in workspace_patterns(root) {
        expand_workspace_pattern(root, files, &pattern, &mut out);
    }
    out
}

fn workspace_patterns(root: &Path) -> Vec<String> {
    let mut patterns = Vec::new();
    if let Ok(text) = fs::read_to_string(root.join("package.json"))
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
        && let Some(workspaces) = value.get("workspaces")
    {
        if let Some(array) = workspaces.as_array() {
            patterns.extend(
                array
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string)),
            );
        } else if let Some(array) = workspaces.get("packages").and_then(|v| v.as_array()) {
            patterns.extend(
                array
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string)),
            );
        }
    }
    if let Ok(text) = fs::read_to_string(root.join("pnpm-workspace.yaml")) {
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("- ") {
                patterns.push(unquote(value.trim()).unwrap_or_else(|| value.trim().to_string()));
            }
        }
    }
    if let Ok(text) = fs::read_to_string(root.join("Cargo.toml")) {
        patterns.extend(cargo_workspace_array_values(&text, "members"));
    }
    if let Ok(text) = fs::read_to_string(root.join("go.work")) {
        patterns.extend(go_work_uses(&text));
    }
    if let Ok(text) = fs::read_to_string(root.join("pyproject.toml")) {
        patterns.extend(pyproject_workspace_patterns(&text));
    }
    patterns
        .into_iter()
        .map(|pattern| normalize_rel_path(pattern.trim().trim_start_matches("./")))
        .filter(|pattern| !pattern.is_empty() && pattern != ".")
        .collect()
}

fn expand_workspace_pattern(
    root: &Path,
    files: &BTreeMap<String, FileInfo>,
    pattern: &str,
    out: &mut BTreeSet<String>,
) {
    if pattern.starts_with('!') || pattern.contains("**") || pattern.contains('{') {
        return;
    }
    if let Some(base) = pattern.strip_suffix("/*") {
        let base = normalize_rel_path(base);
        let Ok(children) = fs::read_dir(root.join(&base)) else {
            return;
        };
        for child in children.flatten() {
            let child_path = child.path();
            if child_path.is_dir() {
                let rel = normalize_rel_path(
                    &child_path
                        .strip_prefix(root)
                        .unwrap_or(&child_path)
                        .to_string_lossy(),
                );
                if workspace_path_has_project(root, files, &rel) {
                    out.insert(rel);
                }
            }
        }
        return;
    }
    if !pattern.contains('*') && workspace_path_has_project(root, files, pattern) {
        out.insert(normalize_rel_path(pattern));
    }
}

fn workspace_path_has_project(root: &Path, files: &BTreeMap<String, FileInfo>, rel: &str) -> bool {
    let rel = normalize_rel_path(rel);
    if !root.join(&rel).is_dir() || should_ignore_rel(&rel) {
        return false;
    }
    let prefix = format!("{}/", rel.trim_end_matches('/'));
    files.keys().any(|file| file.starts_with(&prefix))
        || [
            "package.json",
            "Cargo.toml",
            "go.mod",
            "pyproject.toml",
            "src",
        ]
        .iter()
        .any(|marker| root.join(&rel).join(marker).exists())
}

fn pyproject_workspace_patterns(text: &str) -> Vec<String> {
    let Some(value) = parse_toml_value(text) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for key in ["members", "packages"] {
        if let Some(values) = value.get(key).and_then(toml_string_array) {
            out.extend(values);
        }
        if let Some(values) = value
            .get("project")
            .and_then(|project| project.get(key))
            .and_then(toml_string_array)
        {
            out.extend(values);
        }
    }
    if let Some(values) = value
        .get("tool")
        .and_then(|tool| tool.get("uv"))
        .and_then(|uv| uv.get("workspace"))
        .and_then(|workspace| workspace.get("members"))
        .and_then(toml_string_array)
    {
        out.extend(values);
    }
    unique_strings(out)
}

fn go_work_uses(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_block = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("use (") {
            in_block = true;
            continue;
        }
        if in_block {
            if trimmed.starts_with(')') {
                in_block = false;
            } else if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("use ") {
            out.push(value.trim().to_string());
        }
    }
    out
}

pub fn changed_files(root: &Path, staged: bool, since: Option<&str>) -> Vec<String> {
    if let Some(since) = since
        && let Some(files) = git_name_only(root, &["diff", "--name-only", "--relative", since])
    {
        return files;
    }
    if staged {
        return git_name_only(root, &["diff", "--name-only", "--relative", "--cached"])
            .unwrap_or_default();
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain", "-uall", "--", "."])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let root_prefix = git_status_root_prefix(root);
    let mut files = BTreeSet::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if line.len() < 4 {
            continue;
        }
        let mut path = line[3..].trim().to_string();
        if let Some((_, new_path)) = path.split_once(" -> ") {
            path = new_path.to_string();
        }
        let rel = normalize_rel_path(&path);
        let rel = if let Some(prefix) = root_prefix.as_deref() {
            let Some(stripped) = rel.strip_prefix(prefix) else {
                continue;
            };
            normalize_rel_path(stripped)
        } else {
            rel
        };
        if !rel.is_empty() && !should_ignore_rel(&rel) {
            files.insert(rel);
        }
    }
    files.into_iter().collect()
}

fn git_status_root_prefix(root: &Path) -> Option<String> {
    let git_root = git_root(root)?;
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let git_root = git_root
        .canonicalize()
        .unwrap_or_else(|_| git_root.to_path_buf());
    if root == git_root {
        return None;
    }
    let rel = root.strip_prefix(git_root).ok()?;
    let rel = normalize_rel_path(&rel.to_string_lossy());
    (!rel.is_empty()).then(|| format!("{}/", rel.trim_end_matches('/')))
}

fn git_name_only(root: &Path, args: &[&str]) -> Option<Vec<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
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

pub fn normalize_rel_path(path: &str) -> String {
    let mut out = path.replace('\\', "/");
    while out.contains("//") {
        out = out.replace("//", "/");
    }
    let mut parts = Vec::new();
    for part in out.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}

pub fn tokenize(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .map(str::to_ascii_lowercase)
        .filter(|s| s.len() >= 2)
        .collect()
}

pub fn path_tokens(rel: &str) -> BTreeSet<String> {
    tokenize(&rel.replace(['/', '-', '_'], " "))
}

fn unique_strings(items: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for item in items {
        if seen.insert(item.clone()) {
            out.push(item);
        }
    }
    out
}

fn unique_pairs(items: Vec<(String, String)>) -> Vec<(String, String)> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for item in items {
        if seen.insert(item.clone()) {
            out.push(item);
        }
    }
    out
}

pub fn is_source_ext(ext: &str) -> bool {
    SOURCE_EXTS.iter().any(|x| x == &ext)
}

fn identifier_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"[A-Za-z_$][A-Za-z0-9_$]*"#).expect("valid identifier regex"))
}

fn jsx_tag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"<\s*/?\s*([A-Z][A-Za-z0-9_$]*)\b"#).expect("valid jsx tag regex")
    })
}

fn js_function_params_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?s)\bfunction(?:\s+[A-Za-z_$][A-Za-z0-9_$]*)?\s*\((?P<params>[^)]*)\)"#)
            .expect("valid js function params regex")
    })
}

fn js_arrow_params_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?s)\((?P<params>[^)]*)\)\s*(?::\s*[^=]+?)?=>"#)
            .expect("valid js arrow params regex")
    })
}

fn js_method_params_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"(?s)\b(?P<name>[A-Za-z_$][A-Za-z0-9_$]*)\s*\((?P<params>[^)]*)\)\s*(?::\s*[^={]+?)?\{"#,
        )
        .expect("valid js method params regex")
    })
}

fn js_single_arrow_param_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)(?:^|[=(:,]\s*)(?P<param>[A-Za-z_$][A-Za-z0-9_$]*)\s*=>"#)
            .expect("valid js single arrow param regex")
    })
}

fn js_for_binding_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?s)\bfor\s*(?:await\s*)?\(\s*(?:const|let|var)\s+(?P<binding>[^;)]*?)\s+(?:of|in)\b"#)
            .expect("valid js for binding regex")
    })
}

fn js_catch_param_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?s)\bcatch\s*\(\s*(?P<param>[^)]*?)\s*\)"#)
            .expect("valid js catch param regex")
    })
}

fn js_static_import_statement_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"(?s)^\s*import\s+(?P<type>type\s+)?(?:(?P<clause>.+?)\s+from\s*)?['"](?P<spec>[^'"]+)['"]"#,
        )
        .expect("valid js static import statement regex")
    })
}

fn js_export_from_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?s)^\s*export\s+.+?\s+from\s*['"](?P<spec>[^'"]+)['"]"#)
            .expect("valid js export-from regex")
    })
}

fn js_export_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"\bexport\s+(?:default\s+)?(?:class|function|const|let|var|interface|type|enum)\s+([A-Za-z0-9_]+)"#)
            .expect("valid js export regex")
    })
}

fn js_symbol_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*(?P<export>export\s+)?(?:default\s+)?(?:async\s+)?(?P<kind>function|class|const|let|var|interface|type|enum)\s+(?P<name>[A-Za-z_$][A-Za-z0-9_$]*)"#)
            .expect("valid js symbol regex")
    })
}

fn js_default_symbol_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*export\s+default\s+(?:async\s+)?(?P<kind>function|class)\b(?:\s+(?P<name>[A-Za-z_$][A-Za-z0-9_$]*))?"#)
            .expect("valid js default symbol regex")
    })
}

fn py_import_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s*(?:from\s+([A-Za-z0-9_\.]+)\s+import|import\s+([A-Za-z0-9_\.]+))"#)
            .expect("valid python import regex")
    })
}

fn swift_package_name_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"name:\s*"([^"]+)""#).expect("valid swift package name regex"))
}

fn swift_package_path_dependency_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"\.package\s*\(\s*path:\s*"([^"]+)""#)
            .expect("valid swift package path dependency regex")
    })
}

fn swift_import_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s*(?:@\w+(?:\([^)]*\))?\s+)?import\s+(?:(?:class|struct|enum|protocol|func|var|typealias)\s+)?([A-Za-z_][A-Za-z0-9_]*)"#)
            .expect("valid swift import regex")
    })
}

fn swift_type_symbol_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*(?:@\w+(?:\([^)]*\))?\s+)*(?P<mods>(?:(?:public|open|package|internal|fileprivate|private|final|static|class|indirect)\s+)*)?(?P<kind>class|struct|enum|protocol|actor)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)"#)
            .expect("valid swift type symbol regex")
    })
}

fn swift_func_symbol_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*(?:@\w+(?:\([^)]*\))?\s+)*(?P<mods>(?:(?:public|open|package|internal|fileprivate|private|static|class|mutating|nonmutating|override|final)\s+)*)?func\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*\("#)
            .expect("valid swift function symbol regex")
    })
}

fn swift_property_symbol_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*(?:@\w+(?:\([^)]*\))?\s+)*(?P<mods>(?:(?:public|open|package|internal|fileprivate|private|static|class|weak|unowned|lazy|override|final)\s+)*)?(?P<kind>let|var)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\b"#)
            .expect("valid swift property symbol regex")
    })
}

fn py_def_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^(?:class|def)\s+([A-Za-z0-9_]+)"#).expect("valid py def regex")
    })
}

fn rust_use_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s*(?:use|pub\s+use)\s+([A-Za-z0-9_:]+)"#).expect("valid rust use regex")
    })
}

fn rust_mod_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s*(?:pub\s+)?mod\s+([A-Za-z0-9_]+)\s*;"#).expect("valid rust mod regex")
    })
}

fn rust_symbol_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*(?P<pub>pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?P<kind>fn|struct|enum|trait|mod)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)"#)
            .expect("valid rust symbol regex")
    })
}

fn rust_impl_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*impl(?:<[^>]+>)?\s+(?P<name>[A-Za-z_][A-Za-z0-9_:<>]*(?:\s+for\s+[A-Za-z_][A-Za-z0-9_:<>]*)?)"#)
            .expect("valid rust impl regex")
    })
}

fn python_symbol_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*(?:async\s+)?(?P<kind>def|class)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)"#)
            .expect("valid python symbol regex")
    })
}

fn go_func_symbol_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*func\s+(?P<receiver>\([^)]*\)\s*)?(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*\("#)
            .expect("valid go function symbol regex")
    })
}

fn go_type_symbol_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*type\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s+(?P<kind>struct|interface|[A-Za-z_][A-Za-z0-9_]*)"#)
            .expect("valid go type symbol regex")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_workspace_table_and_dotted_forms_parse() {
        let workspace = r#"[workspace]
members = [
  "crates/app",
  "crates/renderer",
]
exclude = ["crates/ignored"]
dependencies.ctx_fixture_tools = { path = "crates/tools" }
dependencies.ctx_fixture_extra.path = "crates/extra"
dependencies.ctx_fixture_inline = { path = "crates/inline" }
dependencies.ctx_fixture_quoted = { version = "0.1, still a string", path = "crates/quoted,comma" }

[workspace.dependencies.ctx_fixture_replay]
path = "crates/replay"
"#;
        assert_eq!(
            cargo_workspace_array_values(workspace, "members"),
            vec!["crates/app".to_string(), "crates/renderer".to_string()]
        );
        assert_eq!(
            cargo_workspace_array_values(workspace, "exclude"),
            vec!["crates/ignored".to_string()]
        );
        let deps = cargo_workspace_path_dependencies(workspace);
        assert_eq!(
            deps.get("ctx_fixture_replay").map(String::as_str),
            Some("crates/replay")
        );
        assert_eq!(
            deps.get("ctx_fixture_tools").map(String::as_str),
            Some("crates/tools")
        );
        assert_eq!(
            deps.get("ctx_fixture_extra").map(String::as_str),
            Some("crates/extra")
        );
        assert_eq!(
            deps.get("ctx_fixture_inline").map(String::as_str),
            Some("crates/inline")
        );
        assert_eq!(
            deps.get("ctx_fixture_quoted").map(String::as_str),
            Some("crates/quoted,comma")
        );
        let root_dotted = r#"workspace.members = ["crates/app", "crates/replay"]
workspace.dependencies.ctx_fixture_root = { path = "crates/root" }
"#;
        assert_eq!(
            cargo_workspace_array_values(root_dotted, "members"),
            vec!["crates/app".to_string(), "crates/replay".to_string()]
        );
        assert_eq!(
            cargo_workspace_path_dependencies(root_dotted)
                .get("ctx_fixture_root")
                .map(String::as_str),
            Some("crates/root")
        );

        let package = r#"[dependencies]
ctx_fixture_replay.workspace = true
ctx_fixture_tools.workspace = true
ctx_fixture_inline = { version = "0.1, still a string", path = "crates/inline,comma" }

[dependencies.ctx_fixture_table]
workspace = true

[target.'cfg(unix)'.dependencies.ctx_fixture_target]
path = "crates/target"

[package.metadata.fake.dependencies.ctx_fixture_ignored]
path = "crates/ignored"
"#;
        assert_eq!(
            cargo_workspace_dependency_names(package),
            vec![
                "ctx_fixture_replay".to_string(),
                "ctx_fixture_table".to_string(),
                "ctx_fixture_tools".to_string()
            ]
        );
        let path_deps = cargo_path_dependencies(package);
        assert!(
            path_deps.iter().any(|(name, path)| {
                name == "ctx_fixture_inline" && path == "crates/inline,comma"
            })
        );
        assert!(
            path_deps
                .iter()
                .any(|(name, path)| name == "ctx_fixture_target" && path == "crates/target")
        );
        assert!(
            path_deps
                .iter()
                .all(|(name, _)| name != "ctx_fixture_ignored")
        );
        assert!(cargo_workspace_member_pattern_matches(
            "crates/renderer",
            "crates/renderer"
        ));
        assert!(cargo_workspace_member_pattern_matches(
            "crates/group/app",
            "crates/*/app"
        ));
    }

    #[test]
    fn cargo_package_name_uses_structural_toml() {
        assert_eq!(
            cargo_package_name(
                r#"[package]
name = "ctx_fixture_renderer"
version = "0.1.0"
edition = "2024"
"#
            )
            .as_deref(),
            Some("ctx_fixture_renderer")
        );
    }

    #[test]
    fn javascript_local_dependency_specs_parse_only_relative_paths() {
        assert_eq!(
            js_local_dependency_path("file:../renderer").as_deref(),
            Some("../renderer")
        );
        assert_eq!(
            js_local_dependency_path("link:./packages/replay").as_deref(),
            Some("./packages/replay")
        );
        assert_eq!(js_local_dependency_path("portal:..").as_deref(), Some(".."));
        assert_eq!(
            js_local_dependency_path("workspace:../renderer").as_deref(),
            Some("../renderer")
        );
        assert_eq!(js_local_dependency_path("workspace:*"), None);
        assert_eq!(js_local_dependency_path("^1.2.3"), None);
        assert_eq!(js_local_dependency_path("file:/tmp/renderer"), None);
        assert!(js_dependency_spec_is_local_protocol(
            "file:../../../external"
        ));
        assert!(js_dependency_spec_is_local_protocol(
            "workspace:/tmp/renderer"
        ));
        assert!(js_dependency_spec_is_local_protocol(
            "workspace:../../../external"
        ));
        assert!(!js_dependency_spec_is_local_protocol("workspace:"));
        assert!(!js_dependency_spec_is_local_protocol("workspace:*"));
        assert!(!js_dependency_spec_is_local_protocol("workspace:^1.2.3"));
    }

    #[test]
    fn javascript_import_specs_ignore_import_text_inside_strings() {
        let text = r#"const docs = "import { ShellHint } from './shell-hint';";
const tmpl = `require('./shadow')`;
import { Real as LocalReal } from './real';
export { Other } from './other';
const lazy = import('./lazy');
const required = require('./required');
"#;

        let specs = extract_js_import_specs(text);

        assert!(specs.contains("./real"));
        assert!(specs.contains("./other"));
        assert!(specs.contains("./lazy"));
        assert!(specs.contains("./required"));
        assert!(!specs.contains("./shell-hint"));
        assert!(!specs.contains("./shadow"));

        let bindings = extract_js_import_bindings(text);
        assert_eq!(
            bindings
                .get("./real")
                .and_then(|map| map.get("LocalReal"))
                .map(String::as_str),
            Some("Real")
        );
        assert!(!bindings.contains_key("./shell-hint"));
    }

    #[test]
    fn javascript_local_bindings_capture_function_and_destructured_params() {
        let text = r#"import { ShellHint } from './shell-hint';

export function ShellParamShadowView({ ShellHint }: Props) {
  return <ShellHint />;
}

const Arrow = ({ CanvasShellHint }: Props) => <CanvasShellHint />;
const Single = ShellAction => <ShellAction />;
export default function({ DefaultShellHint }: Props) {
  return <DefaultShellHint />;
}
const methods = {
  render({ MethodShellHint }: Props) {
    return <MethodShellHint />;
  },
};
function Destructure(props) {
  const { LocalHint } = props;
  let {
    MultilineHint,
  } = props;
  const { hint: AliasHint = FallbackHint } = props;
  const [ArrayHint = FallbackArrayHint] = props.items;
  return <LocalHint />;
}
function LoopAndCatch() {
  for (const LoopHint of hints) {
    return <LoopHint />;
  }
  for await (const AwaitLoopHint of hints) {
    return <AwaitLoopHint />;
  }
  try {
    run();
  } catch (CatchHint) {
    return <CatchHint />;
  }
}
"#;

        let bindings = extract_local_bindings(text, "tsx");

        assert!(bindings.contains("ShellHint"));
        assert!(bindings.contains("CanvasShellHint"));
        assert!(bindings.contains("ShellAction"));
        assert!(bindings.contains("DefaultShellHint"));
        assert!(bindings.contains("MethodShellHint"));
        assert!(bindings.contains("LocalHint"));
        assert!(bindings.contains("MultilineHint"));
        assert!(bindings.contains("AliasHint"));
        assert!(bindings.contains("ArrayHint"));
        assert!(bindings.contains("LoopHint"));
        assert!(bindings.contains("AwaitLoopHint"));
        assert!(bindings.contains("CatchHint"));
    }

    #[test]
    fn javascript_jsx_tags_ignore_type_generic_arguments() {
        let generic_only = r#"import { GroupCard } from './card';

const value = identity<GroupCard | null>(null);
type Factory = <GroupCard>() => void;
const make = <GroupCard extends object>() => null;
"#;
        assert!(!extract_jsx_tags(generic_only, "tsx").contains("GroupCard"));

        let text = r#"import { GroupCard } from './card';

export function View() {
  return <GroupCard title="real" />;
}
"#;

        let tags = extract_jsx_tags(text, "tsx");

        assert!(tags.contains("GroupCard"));
        assert_eq!(tags.len(), 1);
    }

    #[test]
    fn javascript_regex_keyword_probe_handles_unicode_prefix() {
        assert!(!previous_word_is("навигации", "return"));
        assert!(previous_word_is("навигации return", "return"));
    }

    #[test]
    fn cargo_workspace_edges_use_workspace_dependency_tables() {
        let repo = tempfile::TempDir::new().expect("temp repo");
        write_test_file(
            &repo.path().join("Cargo.toml"),
            r#"[workspace]
members = [
  "crates/renderer",
  "crates/replay",
]

[workspace.dependencies.ctx_fixture_replay]
path = "crates/replay"
"#,
        );
        write_test_file(
            &repo.path().join("crates/renderer/Cargo.toml"),
            r#"[package]
name = "ctx_fixture_renderer"
version = "0.1.0"
edition = "2024"

[dependencies.ctx_fixture_replay]
workspace = true
"#,
        );
        write_test_file(
            &repo.path().join("crates/replay/Cargo.toml"),
            r#"[package]
name = "ctx_fixture_replay"
version = "0.1.0"
edition = "2024"
"#,
        );
        let project = load_project_with_cache(
            RootSelection::Exact(repo.path().to_path_buf()),
            CacheWriteMode::ReadOnly,
        )
        .expect("load project");
        let by_path: BTreeMap<String, &PackageInfo> = project
            .packages
            .iter()
            .map(|package| (package.path.clone(), package))
            .collect();
        let workspaces =
            cargo_workspace_infos(repo.path(), &project.files, &project.packages, &by_path);
        assert!(
            project.package_edges.iter().any(|edge| {
                edge.from == "crates/renderer"
                    && edge.to == "crates/replay"
                    && edge.source == "Cargo.toml workspace dependency"
            }),
            "files: {:#?}; packages: {:#?}; workspaces: {:#?}; package edges: {:#?}",
            project.files.keys().collect::<Vec<_>>(),
            project.packages,
            workspaces,
            project.package_edges
        );
    }

    #[test]
    fn cargo_paths_resolve_inside_repo_without_root_escape() {
        assert_eq!(
            resolve_repo_relative_path(Path::new("crates/app"), "../renderer").as_deref(),
            Some("crates/renderer")
        );
        assert_eq!(
            resolve_repo_relative_path(Path::new("."), "crates/replay").as_deref(),
            Some("crates/replay")
        );
        assert_eq!(
            resolve_repo_relative_path(Path::new("."), "../external"),
            None
        );
        assert_eq!(
            resolve_repo_relative_path(Path::new("nested"), "../../external"),
            None
        );
        assert_eq!(
            resolve_repo_relative_path(Path::new("."), "/tmp/external"),
            None
        );
        assert!(!cargo_workspace_member_pattern_matches(
            "external",
            "../external"
        ));
    }

    #[test]
    fn pyproject_paths_use_structural_toml() {
        let pyproject = r#"[project]
name = "ctx-renderer"

[tool.uv.sources]
ctx-replay = { path = "../replay,with-comma", marker = "platform_system == 'Darwin,macOS'" }

[tool.poetry.dependencies]
ctx-tools = { path = "../tools" }
ctx-version-only = "^1"
"#;
        assert_eq!(
            pyproject_package_name(pyproject).as_deref(),
            Some("ctx-renderer")
        );
        let deps = pyproject_path_dependencies(pyproject);
        assert!(
            deps.iter()
                .any(|(name, path)| name == "ctx-replay" && path == "../replay,with-comma")
        );
        assert!(
            deps.iter()
                .any(|(name, path)| name == "ctx-tools" && path == "../tools")
        );
        assert!(deps.iter().all(|(name, _)| name != "ctx-version-only"));
    }

    #[test]
    fn pyproject_workspace_patterns_ignore_unrelated_tool_metadata() {
        let pyproject = r#"[project]
name = "ctx-python-workspace"
members = ["services/replay"]
packages = ["apps/api"]

[tool.uv.workspace]
members = ["libs/*"]

[tool.unrelated]
members = ["shadow/replay"]
packages = ["shadow/renderer"]

[tool.poetry]
packages = ["not-a-workspace"]
"#;
        let patterns = pyproject_workspace_patterns(pyproject);
        assert!(patterns.iter().any(|item| item == "services/replay"));
        assert!(patterns.iter().any(|item| item == "apps/api"));
        assert!(patterns.iter().any(|item| item == "libs/*"));
        assert!(patterns.iter().all(|item| item != "shadow/replay"));
        assert!(patterns.iter().all(|item| item != "shadow/renderer"));
        assert!(patterns.iter().all(|item| item != "not-a-workspace"));
    }

    #[test]
    fn javascript_symbols_keep_exports_and_line_ranges() {
        let text = r#"import { frameAt } from "./timeline";

export function seekFrame(timeMs: number): number {
  return frameAt(timeMs);
}

export const FeedPage = () => null;
const useReplayClock = () => 1;
export interface ReplayDto {
  frame: number;
}
"#;

        let symbols = extract_symbols(text, "tsx");

        assert_symbol(&symbols, "seekFrame", "function", true, 3, 5);
        assert_symbol(&symbols, "FeedPage", "component", true, 7, 7);
        assert_symbol(&symbols, "useReplayClock", "hook", false, 8, 8);
        assert_symbol(&symbols, "ReplayDto", "interface", true, 9, 11);
    }

    #[test]
    fn javascript_semicolonless_expression_symbol_does_not_swallow_next_block() {
        let text = r#"export const FeedPage = () => <View />

export function renderFeed() {
  return FeedPage
}
"#;

        let symbols = extract_symbols(text, "tsx");

        assert_symbol(&symbols, "FeedPage", "component", true, 1, 1);
        assert_symbol(&symbols, "renderFeed", "function", true, 3, 5);
    }

    #[test]
    fn javascript_local_export_list_does_not_hide_following_symbols() {
        let text = r#"const Foo = 1;
export { Foo };

export type {
  ReplayDto,
};

export function laterSymbol() {
  return Foo;
}
"#;

        let symbols = extract_symbols(text, "ts");

        assert_symbol(&symbols, "Foo", "const", false, 1, 1);
        assert_symbol(&symbols, "laterSymbol", "function", true, 8, 10);
        assert!(
            symbols.iter().all(|symbol| symbol.name != "ReplayDto"),
            "export-list members are not declarations"
        );
    }

    #[test]
    fn javascript_surface_tokens_capture_ui_selectors_without_plain_text_noise() {
        let text = r#"export function Button() {
  return <button data-testid="submit-order-button" aria-label="Submit order">Submit order</button>;
}

test("flow", async ({ page }) => {
  await test.step("submit-order-button string in prose is not evidence", async () => {});
  await page.goto("/orders/new");
  await expect(page.locator(".submit-order-button")).toBeVisible();
});
"#;

        let surfaces = extract_surfaces(text, "tsx");
        let tokens = &surfaces.tokens;

        assert!(tokens.contains("submit"));
        assert!(tokens.contains("order"));
        assert!(tokens.contains("orders"));
        assert!(surfaces.phrases.contains("submit-order-button"));
        assert!(surfaces.phrases.contains("orders-new"));
        assert!(!tokens.contains("button"));
        assert!(!tokens.contains("flow"));
        assert!(!tokens.contains("prose"));
    }

    #[test]
    fn javascript_surface_phrases_skip_import_paths_and_broad_mode_literals() {
        let text = r#"import { useFrameTitleDrag } from './use-frame-title-drag';

export function Title() {
  return <div className="blueprint-frame-node__title-input nodrag" data-mode="frame-title" />;
}
"#;

        let surfaces = extract_surfaces(text, "tsx");

        assert!(
            surfaces
                .phrases
                .contains("blueprint-frame-node-title-input")
        );
        assert!(!surfaces.phrases.contains("use-frame-title-drag"));
        assert!(!surfaces.phrases.contains("frame-title"));
    }

    #[test]
    fn javascript_surface_phrases_capture_labels_and_routes_only_in_ui_context() {
        let text = r#"test("Open settings panel is prose, not a surface", () => {});

export function SettingsLink() {
  return <a href="/orders/new" aria-label="Open settings panel">Orders</a>;
}

export function CartButton() {
  return <button aria-label="Remove from cart">Remove</button>;
}

export function ImportButton() {
  return <button aria-label="Import (CSV)">Import</button>;
}

test("flow", async ({ page }) => {
  await page.goto("/orders/new");
  await expect(page.getByLabel("Open settings panel")).toBeVisible();
  await expect(page.getByLabel("Remove from cart")).toBeVisible();
  await expect(page.getByLabel("Import (CSV)")).toBeVisible();
});
"#;

        let surfaces = extract_surfaces(text, "tsx");

        assert!(surfaces.phrases.contains("open-settings-panel"));
        assert!(surfaces.phrases.contains("remove-from-cart"));
        assert!(surfaces.phrases.contains("import-csv"));
        assert!(surfaces.phrases.contains("orders-new"));
        assert!(surfaces.tokens.contains("settings"));
        assert!(surfaces.tokens.contains("orders"));
        assert!(!surfaces.tokens.contains("prose"));
    }

    #[test]
    fn javascript_surface_phrases_capture_bounded_jsx_visible_text() {
        let source = r#"export function ShellHint() {
  return (
    <div className="blueprint-canvas__hint" aria-live="polite">
      Дважды кликни по канвасу или нажми <kbd className="kbd">F</kbd> — появится новый кадр
    </div>
  );
}
"#;
        let test = r#"test("this prose is not a surface", async ({ page }) => {
  await expect(page.getByText("Дважды кликни по канвасу или нажми")).toBeVisible();
});
"#;
        let prose = r#"export function Plain() {
  return <p>Дважды кликни по канвасу или нажми</p>;
}
"#;

        let source_surfaces = extract_surfaces(source, "tsx");
        let test_surfaces = extract_surfaces(test, "tsx");
        let prose_surfaces = extract_surfaces(prose, "tsx");

        assert!(
            source_surfaces
                .phrases
                .contains("дважды-кликни-по-канвасу-или-нажми-f-—-появится-новый-кадр")
        );
        assert!(
            test_surfaces
                .phrases
                .contains("дважды-кликни-по-канвасу-или-нажми")
        );
        assert!(
            prose_surfaces.phrases.is_empty(),
            "visible text without a UI surface container should fail closed: {prose_surfaces:#?}"
        );
    }

    #[test]
    fn javascript_surface_phrases_ignore_ui_looking_module_specifiers() {
        let text = r#"import widget from '@app/aria-label-open-settings-panel';
export { widget as openSettingsPanel } from '@app/route-orders-new';
const lazy = import ('@app/data-testid-submit-order-button');
const required = require ('@app/class-name-submit-order-button');
const bareLazy = import ('aria-label-open-settings-panel');
const bareRequired = require ('data-testid-submit-order-button');
const commentedLazy = import(/* webpackChunkName: "settings" */ 'aria-label-open-settings-panel');
import {
  multi,
} from '@app/aria-label-open-settings-panel';
const bareSubpath = require ('@scope/data-testid-submit-order-button');
"#;

        let surfaces = extract_surfaces(text, "tsx");

        assert!(surfaces.phrases.is_empty(), "{surfaces:#?}");
        assert!(surfaces.tokens.is_empty(), "{surfaces:#?}");
    }

    #[test]
    fn javascript_surface_phrases_ignore_multiline_comments() {
        let text = r#"/*
  <button aria-label="Open settings panel" data-testid="submit-order-button">Settings</button>
  await page.goto("/orders/new");
*/
export function CommentOnly() {
  return <div />;
}
"#;

        let surfaces = extract_surfaces(text, "tsx");

        assert!(surfaces.phrases.is_empty(), "{surfaces:#?}");
        assert!(surfaces.tokens.is_empty(), "{surfaces:#?}");
    }

    #[test]
    fn rust_symbols_keep_visibility_and_ranges() {
        let text = r#"use crate::timeline::frame_at;

pub struct Session {
    frame: u64,
}

impl Session {
    pub fn seek_frame(&self, time_ms: u64) -> u64 {
        frame_at(time_ms)
    }
}

fn internal_tick() {}
"#;

        let symbols = extract_symbols(text, "rs");

        assert_symbol(&symbols, "Session", "struct", true, 3, 5);
        assert_symbol(&symbols, "Session", "impl", false, 7, 11);
        assert_symbol(&symbols, "seek_frame", "function", true, 8, 10);
        assert_symbol(&symbols, "internal_tick", "function", false, 13, 13);
    }

    #[test]
    fn python_symbols_keep_functions_and_classes_without_export_claims() {
        let text = r#"from .timeline import frame_at


class ReplaySession:
    pass


def seek(frames: list[int], frame: int) -> int:
    return frame_at(frames, frame)


async def refresh() -> None:
    return None
"#;

        let symbols = extract_symbols(text, "py");

        assert_symbol(&symbols, "ReplaySession", "class", false, 4, 5);
        assert_symbol(&symbols, "seek", "function", false, 8, 9);
        assert_symbol(&symbols, "refresh", "function", false, 12, 13);
    }

    #[test]
    fn go_symbols_keep_exports_functions_methods_and_types() {
        let text = r#"package session

type Frame struct {
    Index int
}

func Seek(frames []Frame, frame int) Frame {
    return frames[frame]
}

func (s Session) tick() {}
"#;

        let symbols = extract_symbols(text, "go");

        assert_symbol(&symbols, "Frame", "struct", true, 3, 5);
        assert_symbol(&symbols, "Seek", "function", true, 7, 9);
        assert_symbol(&symbols, "tick", "method", false, 11, 11);
    }

    #[test]
    fn swift_symbols_keep_modifiers_imports_and_ranges() {
        let text = r#"import Foundation
import SwiftUI

@MainActor
public final class ReplayViewModel: ObservableObject {
    @Published public var selectedID: String?

    public struct NavigationFrame {
        let label: String
    }

    public var title: String {
        "Replay"
    }

    private let frames: [NavigationFrame] = []

    public func seekFrame(_ index: Int) -> NavigationFrame? {
        frames.indices.contains(index) ? frames[index] : nil
    }
}

private enum ReplayMode {
    case paused
}
"#;

        let symbols = extract_symbols(text, "swift");

        assert_symbol(&symbols, "ReplayViewModel", "class", true, 5, 21);
        assert_symbol(&symbols, "selectedID", "property", true, 6, 6);
        assert_symbol(&symbols, "NavigationFrame", "struct", true, 8, 10);
        assert_symbol(&symbols, "title", "property", true, 12, 14);
        assert_symbol(&symbols, "frames", "constant", false, 16, 16);
        assert_symbol(&symbols, "seekFrame", "function", true, 18, 20);
        assert_symbol(&symbols, "ReplayMode", "enum", false, 23, 25);

        let imports = swift_import_re()
            .captures_iter(text)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .collect::<Vec<_>>();
        assert_eq!(imports, vec!["Foundation", "SwiftUI"]);

        let qualified_imports = swift_import_re()
            .captures_iter(
                "@testable import SwiftFixture\n@_spi(Internal) import Core\nimport struct Foundation.UUID\n",
            )
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .collect::<Vec<_>>();
        assert_eq!(
            qualified_imports,
            vec!["SwiftFixture", "Core", "Foundation"]
        );
    }

    #[test]
    fn fixture_projects_populate_symbols_for_primary_languages() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");
        let cases = [
            (
                "mixed-monorepo",
                "domains/replay/src/replay-session.ts",
                "seekFrame",
            ),
            (
                "rust-workspace",
                "crates/replay/src/session.rs",
                "seek_frame",
            ),
            (
                "python-workspace",
                "services/replay/replay/session.py",
                "seek",
            ),
            ("go-workspace", "services/replay/session/session.go", "Seek"),
            (
                "swift-package",
                "Sources/SwiftFixture/ViewModel.swift",
                "ReplayViewModel",
            ),
        ];

        for (fixture, rel, symbol) in cases {
            let project = load_project_with_cache(
                RootSelection::Exact(root.join(fixture)),
                CacheWriteMode::ReadOnly,
            )
            .expect("load fixture project");
            let file = project.files.get(rel).unwrap_or_else(|| {
                panic!(
                    "expected file `{rel}` in fixture `{fixture}`; available: {:#?}",
                    project.files.keys().collect::<Vec<_>>()
                )
            });
            assert!(
                file.symbols.iter().any(|item| item.name == symbol),
                "expected `{symbol}` in `{fixture}/{rel}` symbols: {:#?}",
                file.symbols
            );
            assert!(file.line_count > 0, "line_count should be populated");
        }
    }

    fn assert_symbol(
        symbols: &[SymbolInfo],
        name: &str,
        kind: &str,
        exported: bool,
        line_start: usize,
        line_end: usize,
    ) {
        let symbol = symbols
            .iter()
            .find(|item| item.name == name && item.kind == kind && item.line_start == line_start)
            .unwrap_or_else(|| {
                panic!("missing symbol `{name}` kind `{kind}` line `{line_start}` in {symbols:#?}")
            });
        assert_eq!(symbol.exported, exported, "{name} exported mismatch");
        assert_eq!(symbol.line_end, line_end, "{name} line_end mismatch");
    }

    fn write_test_file(path: &Path, body: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(path, body).expect("write test file");
    }
}
