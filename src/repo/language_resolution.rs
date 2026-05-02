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

fn resolve_rust(
    from: &str,
    spec: &str,
    paths: &BTreeSet<String>,
    packages: &[PackageInfo],
) -> Option<String> {
    if let Some(target) = resolve_rust_include_path(from, spec, paths) {
        return Some(target);
    }
    let segments = spec
        .split("::")
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if segments.is_empty() {
        return None;
    }
    if let Some(rest) = spec.strip_prefix("crate::") {
        let crate_root = rust_crate_src_dir(from, packages);
        let rest_segments = rest
            .split("::")
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>();
        return resolve_rust_module_segments(&crate_root, &rest_segments, paths);
    }
    if let Some(rest) = spec.strip_prefix("super::") {
        let base = rust_super_base_dir(from);
        let rest_segments = rest
            .split("::")
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>();
        return resolve_rust_module_segments(&base, &rest_segments, paths);
    }
    if let Some(rest) = spec.strip_prefix("self::") {
        let base = rust_module_dir(from);
        let rest_segments = rest
            .split("::")
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>();
        return resolve_rust_module_segments(&base, &rest_segments, paths);
    }
    if let Some(target) = resolve_rust_package_crate_segments(&segments, paths, packages) {
        return Some(target);
    }
    let base_dir = Path::new(from)
        .parent()
        .map(|p| normalize_rel_path(&p.to_string_lossy()))
        .unwrap_or_default();
    let module_dir = rust_module_dir(from);
    resolve_rust_module_segments(&module_dir, &segments, paths)
        .or_else(|| resolve_rust_module_segments(&base_dir, &segments, paths))
}

fn resolve_rust_package_crate_segments(
    segments: &[&str],
    paths: &BTreeSet<String>,
    packages: &[PackageInfo],
) -> Option<String> {
    let crate_name = segments.first()?;
    let package = packages
        .iter()
        .filter(|package| package.ecosystem == "rust")
        .find(|package| rust_package_crate_name(&package.name) == *crate_name)?;
    let crate_root = rust_package_src_dir(&package.path);
    let rest = &segments[1..];
    if rest.is_empty() {
        return rust_package_root_file(&crate_root, paths);
    }
    resolve_rust_module_segments(&crate_root, rest, paths)
        .or_else(|| rust_package_root_file(&crate_root, paths))
}

fn rust_package_crate_name(package_name: &str) -> String {
    package_name.replace('-', "_")
}

fn rust_package_src_dir(package_path: &str) -> String {
    match package_path {
        "." | "" => "src".to_string(),
        path => normalize_rel_path(&format!("{path}/src")),
    }
}

fn rust_package_root_file(crate_root: &str, paths: &BTreeSet<String>) -> Option<String> {
    for candidate in [format!("{crate_root}/lib.rs"), format!("{crate_root}/main.rs")] {
        let candidate = normalize_rel_path(&candidate);
        if paths.contains(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn resolve_rust_module_segments(
    base: &str,
    segments: &[&str],
    paths: &BTreeSet<String>,
) -> Option<String> {
    for len in (1..=segments.len()).rev() {
        let joined = segments[..len].join("/");
        let candidate_base = if base == "." || base.is_empty() {
            joined
        } else {
            format!("{base}/{joined}")
        };
        for candidate in [
            format!("{candidate_base}.rs"),
            format!("{candidate_base}/mod.rs"),
        ] {
            let candidate = normalize_rel_path(&candidate);
            if paths.contains(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

fn rust_crate_src_dir(from: &str, packages: &[PackageInfo]) -> String {
    let package = packages
        .iter()
        .filter(|package| package.ecosystem == "rust")
        .filter(|package| {
            package.path == "."
                || from == package.path
                || from.starts_with(&format!("{}/", package.path.trim_end_matches('/')))
        })
        .max_by_key(|package| {
            if package.path == "." {
                0
            } else {
                package.path.len()
            }
        });
    match package.map(|package| package.path.as_str()) {
        Some(".") | None => "src".to_string(),
        Some(path) => normalize_rel_path(&format!("{path}/src")),
    }
}

fn resolve_rust_include_path(from: &str, spec: &str, paths: &BTreeSet<String>) -> Option<String> {
    if !spec.ends_with(".rs") {
        return None;
    }
    let base_dir = Path::new(from).parent().unwrap_or_else(|| Path::new("."));
    let candidate = normalize_rel_path(&base_dir.join(spec).to_string_lossy());
    paths.contains(&candidate).then_some(candidate)
}

fn rust_module_dir(from: &str) -> String {
    let path = Path::new(from);
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if matches!(
        path.file_stem().and_then(|name| name.to_str()),
        Some("lib" | "main" | "mod")
    ) {
        return normalize_rel_path(&parent.to_string_lossy());
    }
    let stem = path.file_stem().and_then(|name| name.to_str()).unwrap_or("");
    normalize_rel_path(&parent.join(stem).to_string_lossy())
}

fn rust_super_base_dir(from: &str) -> String {
    let path = Path::new(from);
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    if matches!(
        path.file_stem().and_then(|name| name.to_str()),
        Some("lib" | "main" | "mod")
    ) {
        return parent
            .parent()
            .map(|parent| normalize_rel_path(&parent.to_string_lossy()))
            .unwrap_or_else(|| ".".to_string());
    }
    normalize_rel_path(&parent.to_string_lossy())
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
