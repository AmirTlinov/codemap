use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use ignore::WalkBuilder;
use regex::Regex;

use crate::cache;
use crate::model::{
    AnchorDomain, ConfigLoadError, CtxConfig, Domain, FileInfo, PackageDependency, PackageInfo,
    Project, ScriptInfo,
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

pub fn load_project(root_override: Option<PathBuf>) -> Result<Project> {
    let cwd = if let Some(path) = &root_override {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    } else {
        env::current_dir().context("failed to read current directory")?
    };
    let root = resolve_root(root_override.as_deref(), &cwd)?;
    let remote = git_remote(&root);
    let (anchors, config_path, config_errors) = load_ctx_configs(&root);
    let nearest_agents = nearest_agents(&cwd, &root);
    let mut files = scan_files(&root)?;
    resolve_imports(&mut files);
    let reverse_imports = build_reverse_imports(&files);
    let packages = detect_packages(&root, &files);
    let package_edges = detect_package_edges(&root, &packages);
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

    let project = Project {
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
    };
    cache::write_status(&project, VERSION)?;
    Ok(project)
}

fn resolve_root(root_override: Option<&Path>, cwd: &Path) -> Result<PathBuf> {
    if let Some(path) = root_override {
        let base = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if let Some(git_root) = git_root(&base) {
            return Ok(git_root);
        }
        return Ok(marker_root(&base).unwrap_or(base));
    }
    if let Some(git_root) = git_root(cwd) {
        return Ok(git_root);
    }
    Ok(marker_root(cwd).unwrap_or_else(|| cwd.to_path_buf()))
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
        Ok(serde_yml::from_str(&text)?)
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
    for route in config.task_routes.values_mut() {
        route.read_first = route
            .read_first
            .iter()
            .map(|file| prefix_config_path(base, file))
            .collect();
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
    merged.task_routes.extend(config.task_routes);
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
            language,
            roles: BTreeSet::new(),
            imports: BTreeSet::new(),
            resolved_imports: BTreeSet::new(),
            exports: BTreeSet::new(),
            tokens: path_tokens(&rel),
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
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if matches!(
        name,
        "package.json"
            | "pyproject.toml"
            | "Cargo.toml"
            | "go.mod"
            | "go.work"
            | "AGENTS.md"
            | "README.md"
            | "Makefile"
            | "justfile"
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
        "source_of_truth",
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
        &[
            "route", "router", "locate", "impact", "verify", "widen", "capsule",
        ],
        "routing",
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
    if name == "agents.md" {
        info.roles.insert("agent_bootstrap".to_string());
    }
    if matches!(name.as_str(), ".ctx.yml" | ".ctx.yaml" | ".ctx.json") {
        info.roles.insert("semantic_anchor".to_string());
    }
    if info.roles.contains("test") {
        for role in [
            "source_of_truth",
            "runtime_state",
            "public_boundary",
            "adapter",
            "schema_contract",
            "parser",
            "renderer_ui",
            "persistence",
            "routing",
            "repo_discovery",
            "cache",
            "cli_surface",
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

fn extract_imports_exports(root: &Path, info: &mut FileInfo) {
    if !is_source_ext(&info.ext) {
        return;
    }
    let path = root.join(&info.rel);
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    match info.ext.as_str() {
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte" => {
            let import_re = js_import_re();
            for cap in import_re.captures_iter(&text) {
                if let Some(m) = cap.get(1) {
                    info.imports.insert(m.as_str().trim().to_string());
                }
            }
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
            let import_re = go_import_re();
            for cap in import_re.captures_iter(&text) {
                if let Some(m) = cap.get(1) {
                    info.imports.insert(m.as_str().trim().to_string());
                }
            }
        }
        _ => {}
    }
}

fn resolve_imports(files: &mut BTreeMap<String, FileInfo>) {
    let paths: BTreeSet<String> = files.keys().cloned().collect();
    let snapshot: Vec<(String, String, Vec<String>)> = files
        .values()
        .map(|f| {
            (
                f.rel.clone(),
                f.ext.clone(),
                f.imports.iter().cloned().collect(),
            )
        })
        .collect();
    for (rel, ext, imports) in snapshot {
        let mut resolved = BTreeSet::new();
        for spec in imports {
            if let Some(target) = resolve_import(&rel, &ext, &spec, &paths) {
                resolved.insert(target);
            }
        }
        if let Some(info) = files.get_mut(&rel) {
            info.resolved_imports = resolved;
        }
    }
}

fn resolve_import(from: &str, ext: &str, spec: &str, paths: &BTreeSet<String>) -> Option<String> {
    if spec.starts_with('.') {
        return resolve_relative(from, spec, paths);
    }
    match ext {
        "py" => resolve_python(spec, paths),
        "rs" => resolve_rust(from, spec, paths),
        _ => None,
    }
}

fn resolve_relative(from: &str, spec: &str, paths: &BTreeSet<String>) -> Option<String> {
    let base_dir = Path::new(from)
        .parent()
        .map(|p| normalize_rel_path(&p.to_string_lossy()))
        .unwrap_or_default();
    let base = normalize_rel_path(&format!("{base_dir}/{spec}"));
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

fn resolve_python(spec: &str, paths: &BTreeSet<String>) -> Option<String> {
    let base = spec.replace('.', "/");
    [format!("{base}.py"), format!("{base}/__init__.py")]
        .into_iter()
        .chain([
            format!("src/{base}.py"),
            format!("src/{base}/__init__.py"),
            format!("app/{base}.py"),
            format!("app/{base}/__init__.py"),
        ])
        .find(|c| paths.contains(c))
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

fn build_reverse_imports(files: &BTreeMap<String, FileInfo>) -> BTreeMap<String, BTreeSet<String>> {
    let mut reverse: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for file in files.values() {
        for target in &file.resolved_imports {
            reverse
                .entry(target.clone())
                .or_default()
                .insert(file.rel.clone());
        }
    }
    reverse
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

fn detect_package_edges(root: &Path, packages: &[PackageInfo]) -> Vec<PackageDependency> {
    let mut edges = Vec::new();
    let by_name: BTreeMap<String, &PackageInfo> = packages
        .iter()
        .map(|package| (package.name.clone(), package))
        .collect();
    let by_path: BTreeMap<String, &PackageInfo> = packages
        .iter()
        .map(|package| (package.path.clone(), package))
        .collect();

    for package in packages {
        match package.ecosystem.as_str() {
            "javascript" => {
                edges.extend(js_package_edges(root, package, &by_name));
            }
            "rust" => {
                edges.extend(cargo_package_edges(root, package, &by_path));
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
) -> Vec<PackageDependency> {
    let Ok(text) = fs::read_to_string(root.join(&package.manifest)) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
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
        for dep in map.keys() {
            if let Some(target) = by_name.get(dep) {
                edges.push(PackageDependency {
                    from: package.path.clone(),
                    from_manifest: package.manifest.clone(),
                    to: target.path.clone(),
                    to_manifest: Some(target.manifest.clone()),
                    dependency: dep.clone(),
                    source: format!("package.json {section}"),
                });
            }
        }
    }
    edges
}

fn cargo_package_edges(
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
    cargo_path_dependencies(&text)
        .into_iter()
        .filter_map(|(name, path)| {
            let target_path = normalize_rel_path(&base.join(path).to_string_lossy());
            let target = by_path.get(&target_path)?;
            Some(PackageDependency {
                from: package.path.clone(),
                from_manifest: package.manifest.clone(),
                to: target.path.clone(),
                to_manifest: Some(target.manifest.clone()),
                dependency: name,
                source: "Cargo.toml path dependency".to_string(),
            })
        })
        .collect()
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
    let mut in_package = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_package = trimmed == "[package]";
            continue;
        }
        if in_package
            && let Some(raw) = trimmed.strip_prefix("name")
            && let Some(value) = raw.split_once('=').map(|(_, value)| value.trim())
        {
            return unquote(value).filter(|s| !s.is_empty());
        }
    }
    None
}

fn cargo_path_dependencies(text: &str) -> Vec<(String, String)> {
    let mut deps = Vec::new();
    let mut section: Option<String> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let name = trimmed.trim_matches(&['[', ']'][..]).to_string();
            section = (name.contains("dependencies")).then_some(name);
            continue;
        }
        if section.is_none() || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((name, value)) = trimmed.split_once('=') else {
            continue;
        };
        let Some(path) = cargo_inline_path(value) else {
            continue;
        };
        deps.push((name.trim().to_string(), path));
    }
    deps
}

fn cargo_inline_path(value: &str) -> Option<String> {
    let value = value.trim();
    if !(value.starts_with('{') && value.ends_with('}')) {
        return None;
    }
    for part in value.trim_matches(&['{', '}'][..]).split(',') {
        let (key, raw_value) = part.split_once('=')?;
        if key.trim() == "path" {
            return unquote(raw_value.trim()).filter(|s| !s.is_empty());
        }
    }
    None
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
    } else if root.join("go.mod").exists() {
        "go"
    } else if root.join("pyproject.toml").exists() || root.join("requirements.txt").exists() {
        "python"
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
        patterns.extend(toml_array_values(&text, &["members"]));
    }
    if let Ok(text) = fs::read_to_string(root.join("go.work")) {
        patterns.extend(go_work_uses(&text));
    }
    if let Ok(text) = fs::read_to_string(root.join("pyproject.toml")) {
        patterns.extend(toml_array_values(&text, &["members", "packages"]));
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

fn toml_array_values(text: &str, keys: &[&str]) -> Vec<String> {
    let mut values = Vec::new();
    let mut collecting = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }
        if collecting {
            values.extend(quoted_values(trimmed));
            if trimmed.contains(']') {
                collecting = false;
            }
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        if !keys.iter().any(|wanted| key.trim() == *wanted) {
            continue;
        }
        values.extend(quoted_values(value));
        collecting = value.contains('[') && !value.contains(']');
    }
    values
}

fn quoted_values(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for ch in text.chars() {
        if let Some(active) = quote {
            if ch == active {
                if !current.is_empty() {
                    out.push(current.clone());
                }
                current.clear();
                quote = None;
            } else {
                current.push(ch);
            }
        } else if ch == '"' || ch == '\'' {
            quote = Some(ch);
        }
    }
    out
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
        && let Some(files) = git_name_only(root, &["diff", "--name-only", since])
    {
        return files;
    }
    if staged {
        return git_name_only(root, &["diff", "--name-only", "--cached"]).unwrap_or_default();
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain", "-uall"])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
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
        if !rel.is_empty() && !should_ignore_rel(&rel) {
            files.insert(rel);
        }
    }
    files.into_iter().collect()
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

pub fn is_source_ext(ext: &str) -> bool {
    SOURCE_EXTS.iter().any(|x| x == &ext)
}

fn js_import_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?:import\s+(?:[^'"]+?\s+from\s+)?|export\s+[^'"]*?from\s+|require\s*\(|import\s*\()\s*['"]([^'"]+)['"]"#)
            .expect("valid js import regex")
    })
}

fn js_export_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"\bexport\s+(?:default\s+)?(?:class|function|const|let|var|interface|type|enum)\s+([A-Za-z0-9_]+)"#)
            .expect("valid js export regex")
    })
}

fn py_import_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s*(?:from\s+([A-Za-z0-9_\.]+)\s+import|import\s+([A-Za-z0-9_\.]+))"#)
            .expect("valid python import regex")
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

fn go_import_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#""([^"]+)""#).expect("valid go import regex"))
}
