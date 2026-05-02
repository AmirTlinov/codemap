fn changed_map_path_file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn changed_map_path_is_manifest(path: &str) -> bool {
    matches!(
        changed_map_path_file_name(path).to_ascii_lowercase().as_str(),
        "package.json"
            | "cargo.toml"
            | "go.mod"
            | "go.work"
            | "pyproject.toml"
            | "requirements.txt"
            | "package.swift"
            | "pnpm-workspace.yaml"
            | "pnpm-workspace.yml"
    )
}

fn changed_map_path_is_env(path: &str) -> bool {
    let name = changed_map_path_file_name(path).to_ascii_lowercase();
    name == ".env" || name.starts_with(".env.")
}

fn changed_map_path_is_config(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let name = changed_map_path_file_name(&lower);
    changed_map_path_is_env(&lower)
        || matches!(
            name,
            "dockerfile"
                | "docker-compose.yml"
                | "docker-compose.yaml"
                | "compose.yml"
                | "compose.yaml"
                | "kustomization.yaml"
                | "kustomization.yml"
        )
        || matches!(
            lower.rsplit('.').next().unwrap_or_default(),
            "json" | "toml" | "yaml" | "yml"
        )
}
