// Responsibility: repo-roles-source
use crate::model::FileInfo;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub(crate) fn classify_source_roles(root: &Path, info: &mut FileInfo, rel: &str, name: &str) {
    if source_is_test_support_module(root, info, rel, name) {
        info.roles.insert("test_support".to_string());
    }
    if source_is_module_surface(root, rel, name, &info.ext) {
        info.roles.insert("module".to_string());
    }

    for role in source_path_roles_for_path(rel) {
        info.roles.insert(role.to_string());
    }
}

pub(crate) fn source_path_roles_for_path(rel: &str) -> Vec<&'static str> {
    let tokens = source_path_tokens(rel);
    SOURCE_PATH_ROLE_RULES
        .iter()
        .filter_map(|(role, needles)| {
            needles
                .iter()
                .any(|needle| tokens.contains(*needle))
                .then_some(*role)
        })
        .collect()
}

const SOURCE_PATH_ROLE_RULES: &[(&str, &[&str])] = &[
    (
        "application",
        &["app", "apps", "application", "applications"],
    ),
    ("service", &["service", "services"]),
    (
        "controller",
        &["controller", "controllers", "handler", "handlers", "router"],
    ),
    ("domain", &["domain", "domains"]),
    ("repository", &["repository", "repositories"]),
    (
        "package_graph",
        &[
            "package",
            "packages",
            "workspace",
            "dependency",
            "dependencies",
            "target",
            "targets",
            "manifest",
        ],
    ),
    (
        "role_classifier",
        &["role", "roles", "classifier", "classify", "classification"],
    ),
    (
        "script_catalog",
        &["script", "scripts", "make", "makefile", "justfile"],
    ),
    (
        "map_surface",
        &[
            "map",
            "maps",
            "cone",
            "lens",
            "lenses",
            "graph",
            "boundary",
            "boundaries",
            "impact",
            "diff",
            "unknown",
            "unknowns",
            "symbol",
            "symbols",
            "resolve",
            "resolver",
            "resolution",
            "directory",
            "directories",
            "entry",
        ],
    ),
    (
        "render_surface",
        &["render", "renderer", "markdown", "output", "outputs"],
    ),
    ("helper_surface", &["helper", "helpers", "prelude"]),
    ("proof_surface", &["proof", "coverage", "wiring"]),
    ("contract_surface", &["contract", "contracts"]),
    ("analysis_surface", &["analysis", "analyzer", "analyser"]),
    ("teach_surface", &["teach", "teaching"]),
    (
        "extractor",
        &[
            "scan",
            "scanner",
            "extract",
            "extractor",
            "surfaces",
            "surface",
            "literal",
            "literals",
            "metadata",
            "strip",
            "range",
            "ranges",
            "identifier",
            "identifiers",
            "import",
            "imports",
            "js",
            "alias",
            "aliases",
            "language",
            "languages",
            "object",
            "objects",
            "param",
            "params",
            "regex",
            "string",
            "strings",
            "jsx",
            "playwright",
            "accessibility",
        ],
    ),
    (
        "config_loader",
        &["config", "configs", "constant", "constants", "preamble"],
    ),
    ("evidence_surface", &["evidence", "provenance"]),
    ("cli_surface", &["args", "argument", "arguments"]),
];

fn source_is_test_support_module(root: &Path, info: &FileInfo, rel: &str, name: &str) -> bool {
    if name.starts_with("tests.")
        || name.starts_with("tests_")
        || name.starts_with("test_support")
        || rel.contains("/tests_")
    {
        return true;
    }
    let Ok(text) = fs::read_to_string(root.join(&info.rel)) else {
        return false;
    };
    text.lines()
        .take(8)
        .any(|line| line.trim_start().starts_with("#[cfg(test)]"))
}

fn source_is_module_surface(root: &Path, rel: &str, name: &str, ext: &str) -> bool {
    if !matches!(
        ext,
        "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "swift"
    ) {
        return false;
    }
    if matches!(name, "mod.rs" | "__init__.py") || name.contains(".module.") {
        return true;
    }
    if ext != "rs" {
        return false;
    }
    let path = Path::new(rel);
    let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
        return false;
    };
    let Some(parent) = path.parent() else {
        return false;
    };
    root.join(parent).join(stem).is_dir()
}

fn source_path_tokens(rel: &str) -> BTreeSet<String> {
    let path = Path::new(rel);
    let mut token_source = path
        .parent()
        .map(|parent| parent.to_string_lossy().to_string())
        .unwrap_or_default();
    if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
        if !token_source.is_empty() {
            token_source.push('/');
        }
        token_source.push_str(stem);
    } else if token_source.is_empty() {
        token_source = rel.to_string();
    }
    token_source
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}
