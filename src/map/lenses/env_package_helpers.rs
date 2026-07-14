// Responsibility: env-package-helpers-lens
use crate::map::{code_shape_without_literal_content, quoted_literal_at};
use crate::model::Project;
use crate::repo;
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) fn static_env_names(line: &str) -> Vec<String> {
    let code = code_shape_without_literal_content(line);
    let mut names = BTreeSet::new();
    for prefix in ["process.env.", "import.meta.env."] {
        for start in find_all(&code, prefix) {
            let tail = &line[start + prefix.len()..];
            let name = tail
                .chars()
                .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
                .collect::<String>();
            if !name.is_empty() {
                names.insert(name);
            }
        }
    }
    for call in ["Deno.env.get(", "std::env::var(", "env::var(", "os.getenv("] {
        for start in find_all(&code, call) {
            if let Some(name) = quoted_literal_at(&line[start + call.len()..]) {
                names.insert(name);
            }
        }
    }
    for start in find_all(&code, "os.environ[") {
        if let Some(name) = quoted_literal_at(&line[start + "os.environ[".len()..]) {
            names.insert(name);
        }
    }
    names.into_iter().collect()
}

pub(crate) fn line_may_contain_static_env_reference(line: &str) -> bool {
    [
        "process.env.",
        "import.meta.env.",
        "Deno.env.get(",
        "std::env::var(",
        "env::var(",
        "os.getenv(",
        "os.environ[",
        "env(",
    ]
    .iter()
    .any(|needle| line.contains(needle))
}

pub(crate) fn find_all(value: &str, needle: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let mut offset = 0;
    while let Some(start) = value[offset..].find(needle) {
        let absolute = offset + start;
        out.push(absolute);
        offset = absolute + needle.len();
    }
    out
}

pub(crate) fn env_declaration(project: &Project, rel: &str) -> Option<String> {
    let mut dirs = Vec::new();
    let mut current = Path::new(rel).parent().unwrap_or_else(|| Path::new("."));
    loop {
        dirs.push(current.to_path_buf());
        let Some(parent) = current.parent() else {
            break;
        };
        current = parent;
    }
    if !dirs.iter().any(|dir| dir.as_os_str().is_empty()) {
        dirs.push(Path::new(".").to_path_buf());
    }
    dirs.into_iter()
        .flat_map(|dir| {
            [".env.example", ".env.sample"]
                .into_iter()
                .map(move |name| {
                    let candidate = if dir.as_os_str().is_empty() || dir == Path::new(".") {
                        name.to_string()
                    } else {
                        dir.join(name).to_string_lossy().to_string()
                    };
                    repo::normalize_rel_path(&candidate)
                })
        })
        .find(|candidate| project.files.contains_key(candidate))
}

pub(crate) fn package_public_targets(
    project: &Project,
    package: &crate::model::PackageInfo,
) -> Vec<String> {
    if package.ecosystem != "javascript" {
        return Vec::new();
    }
    let Ok(text) = std::fs::read_to_string(project.root.join(&package.manifest)) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let mut targets = Vec::new();
    if let Some(exports) = value.get("exports") {
        collect_public_manifest_targets(exports, &mut targets);
    }
    for key in ["main", "module", "types", "typings"] {
        if let Some(target) = value.get(key).and_then(|value| value.as_str()) {
            targets.push(target.to_string());
        }
    }
    if let Some(bin) = value.get("bin") {
        collect_public_manifest_targets(bin, &mut targets);
    }
    targets
        .into_iter()
        .flat_map(|target| normalize_package_public_target(package, &target))
        .filter(|target| project.files.contains_key(target))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn collect_public_manifest_targets(value: &serde_json::Value, out: &mut Vec<String>) {
    if let Some(target) = value.as_str() {
        out.push(target.to_string());
        return;
    }
    if let Some(array) = value.as_array() {
        for item in array {
            collect_public_manifest_targets(item, out);
        }
        return;
    }
    if let Some(map) = value.as_object() {
        for value in map.values() {
            collect_public_manifest_targets(value, out);
        }
    }
}

fn normalize_package_public_target(
    package: &crate::model::PackageInfo,
    target: &str,
) -> Vec<String> {
    repo::package_public_target_candidates(&package.path, target)
}
