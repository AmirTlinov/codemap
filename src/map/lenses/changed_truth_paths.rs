fn changed_path_is_generated(project: &Project, path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("/generated/")
        || lower.starts_with("generated/")
        || lower.contains(".generated.")
        || project
            .files
            .get(path)
            .is_some_and(|file| file.roles.contains("generated"))
}

fn changed_path_is_large_binary(project: &Project, path: &str) -> bool {
    const LARGE_BINARY_BYTES: u64 = 10 * 1024 * 1024;
    let size = project
        .files
        .get(path)
        .map(|file| file.size)
        .or_else(|| std::fs::metadata(project.root.join(path)).ok().map(|meta| meta.len()));
    size.is_some_and(|size| size >= LARGE_BINARY_BYTES) && changed_path_is_binary_like(path)
}

fn changed_path_is_binary_like(path: &str) -> bool {
    matches!(
        path.to_ascii_lowercase().rsplit('.').next().unwrap_or_default(),
        "png" | "jpg"
            | "jpeg"
            | "gif"
            | "webp"
            | "pdf"
            | "zip"
            | "gz"
            | "tgz"
            | "xz"
            | "dmg"
            | "mp4"
            | "mov"
            | "sqlite"
            | "db"
            | "bin"
    )
}

fn changed_path_is_model_weight_like(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    changed_lens_path_has_segment(&lower, "models")
        || changed_lens_path_has_segment(&lower, "checkpoints")
        || changed_lens_path_has_segment(&lower, "weights")
        || matches!(
            lower.rsplit('.').next().unwrap_or_default(),
            "safetensors" | "gguf" | "pt" | "pth" | "onnx" | "ckpt" | "bin" | "pb"
        )
}

fn changed_path_is_protected_looking(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.starts_with("vendor/")
        || lower.starts_with("generated/")
        || lower.starts_with("dist/")
        || lower.starts_with("build/")
        || lower.starts_with("target/")
        || lower.starts_with("node_modules/")
        || changed_lens_path_has_segment(&lower, "vendor")
        || changed_lens_path_has_segment(&lower, "generated")
        || changed_lens_path_has_segment(&lower, "dist")
        || changed_lens_path_has_segment(&lower, "build")
        || changed_lens_path_has_segment(&lower, "target")
        || changed_lens_path_has_segment(&lower, "node_modules")
        || changed_path_is_model_weight_like(&lower)
}

fn changed_path_is_instruction_file(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let name = changed_map_path_file_name(&lower);
    lower == "agents.md"
        || lower.starts_with(".agents/")
        || matches!(
            name,
            "agents.md" | "contributing.md" | "code_of_conduct.md" | "security.md"
        )
}

fn changed_manifest_for_lockfile(path: &str) -> Option<String> {
    let lower = path.to_ascii_lowercase();
    let dir = changed_path_dir(path);
    let manifest = match changed_map_path_file_name(&lower) {
        "package-lock.json" | "npm-shrinkwrap.json" | "pnpm-lock.yaml" | "pnpm-lock.yml"
        | "yarn.lock" | "bun.lock" | "bun.lockb" => "package.json",
        "cargo.lock" => "Cargo.toml",
        "uv.lock" | "poetry.lock" | "pdm.lock" => "pyproject.toml",
        "go.sum" => "go.mod",
        "gemfile.lock" => "Gemfile",
        "package.resolved" => "Package.swift",
        _ => return None,
    };
    Some(changed_join_dir(&dir, manifest))
}

fn changed_lockfiles_for_manifest(project: &Project, path: &str) -> Vec<String> {
    let lower = path.to_ascii_lowercase();
    let dir = changed_path_dir(path);
    let candidates = match changed_map_path_file_name(&lower) {
        "package.json" => vec![
            "package-lock.json",
            "npm-shrinkwrap.json",
            "pnpm-lock.yaml",
            "pnpm-lock.yml",
            "yarn.lock",
            "bun.lock",
            "bun.lockb",
        ],
        "cargo.toml" => vec!["Cargo.lock"],
        "pyproject.toml" => vec!["uv.lock", "poetry.lock", "pdm.lock"],
        "go.mod" => vec!["go.sum"],
        "gemfile" => vec!["Gemfile.lock"],
        "package.swift" => vec!["Package.resolved"],
        _ => Vec::new(),
    };
    candidates
        .into_iter()
        .map(|candidate| changed_join_dir(&dir, candidate))
        .filter(|candidate| project.files.contains_key(candidate))
        .collect()
}

fn changed_path_dir(path: &str) -> String {
    Path::new(path)
        .parent()
        .map(|parent| repo::normalize_rel_path(&parent.to_string_lossy()))
        .filter(|parent| parent != ".")
        .unwrap_or_default()
}

fn changed_join_dir(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_string()
    } else {
        format!("{dir}/{name}")
    }
}

fn changed_path_is_runner_like(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let name = changed_map_path_file_name(&lower);
    lower.starts_with("scripts/")
        || lower.starts_with("tools/")
        || lower.starts_with("bin/")
        || changed_lens_path_has_segment(&lower, "scripts")
        || changed_lens_path_has_segment(&lower, "tools")
        || changed_lens_path_has_segment(&lower, "bin")
        || changed_lens_path_has_segment(&lower, "proof")
        || changed_lens_path_has_segment(&lower, "proofs")
        || changed_lens_path_has_segment(&lower, "receipts")
        || changed_lens_path_has_segment(&lower, "doctor")
        || changed_lens_path_has_segment(&lower, "validate")
        || name.starts_with("doctor.")
        || name.starts_with("validate.")
        || name.starts_with("check.")
        || name.starts_with("verify.")
        || lower.ends_with(".sh")
}

fn changed_lens_path_has_segment(path: &str, segment: &str) -> bool {
    path.split('/').any(|part| part == segment)
}

fn changed_lens_path_looks_like_source(path: &str) -> bool {
    matches!(
        path.rsplit('.').next().unwrap_or_default(),
        "rs" | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "py"
            | "go"
            | "swift"
            | "kt"
            | "java"
            | "c"
            | "cc"
            | "cpp"
            | "h"
            | "hpp"
    )
}

fn changed_runner_has_package_script(project: &Project, path: &str) -> bool {
    let stem = Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(path);
    project.scripts.iter().any(|script| {
        script.command.contains(path)
            || script.command.contains(stem)
            || script.name == stem
            || script.path.as_deref() == Some(path)
    })
}
