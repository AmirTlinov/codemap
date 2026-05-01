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

