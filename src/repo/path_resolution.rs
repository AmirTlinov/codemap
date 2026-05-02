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
        "rs" => resolve_rust(from, spec, paths, packages),
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
        "ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "rs", "go", "vue", "svelte", "css",
        "scss", "sass", "less",
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
    for base in [subpath.to_string(), format!("src/{subpath}")] {
        let Some(base) = package_target_path(&package.path, &base) else {
            continue;
        };
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
            .flat_map(|entry| package_public_target_candidates(&package.path, &entry))
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
