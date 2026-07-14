// Responsibility: repo-roles-structural-surfaces
use crate::repo::is_snapshot_ext;

pub(crate) fn is_package_manifest_name(name: &str) -> bool {
    matches!(
        name,
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

pub(crate) fn is_env_surface_name(name: &str) -> bool {
    name == ".env" || name.starts_with(".env.")
}

pub(crate) fn is_lockfile_name(name: &str) -> bool {
    matches!(
        name,
        "package-lock.json"
            | "npm-shrinkwrap.json"
            | "pnpm-lock.yaml"
            | "pnpm-lock.yml"
            | "yarn.lock"
            | "bun.lock"
            | "bun.lockb"
            | "cargo.lock"
            | "poetry.lock"
            | "pdm.lock"
            | "uv.lock"
            | "gemfile.lock"
            | "composer.lock"
    ) || name.ends_with(".lock")
}

pub(crate) fn is_docs_surface(rel: &str, name: &str, ext: &str) -> bool {
    ext == "md"
        && (name == "readme.md"
            || name == "agents.md"
            || rel.starts_with("docs/")
            || rel.contains("/docs/")
            || rel.starts_with("contracts/")
            || rel.contains("/contracts/"))
}

pub(crate) fn is_runtime_config_surface(rel: &str, name: &str) -> bool {
    is_env_surface_name(name)
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
        || rel.starts_with("deploy/")
        || rel.starts_with("deployment/")
        || rel.starts_with("infra/")
        || rel.contains("/deploy/")
        || rel.contains("/deployment/")
        || rel.contains("/k8s/")
}

pub(crate) fn is_snapshot_surface(rel: &str, name: &str, ext: &str) -> bool {
    is_snapshot_ext(ext)
        || rel.contains("/__snapshots__/")
        || rel.starts_with("__snapshots__/")
        || name.ends_with(".snap")
        || name.ends_with(".snapshot")
}

pub(crate) fn is_golden_surface(rel: &str) -> bool {
    rel.starts_with("golden/")
        || rel.contains("/golden/")
        || rel.starts_with("goldens/")
        || rel.contains("/goldens/")
        || rel.starts_with("baselines/")
        || rel.contains("/baselines/")
}
