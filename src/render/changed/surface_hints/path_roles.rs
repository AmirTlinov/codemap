// Responsibility: changed-path-role-classifier

pub(crate) fn changed_roles_for_path(path: &str) -> Vec<String> {
    let lower = path.to_ascii_lowercase();
    let mut roles = std::collections::BTreeSet::new();
    let support_artifact = changed_path_is_support_artifact(&lower);
    if lower.contains(".test.")
        || lower.contains(".spec.")
        || changed_path_has_segment(&lower, "tests")
        || changed_path_has_segment(&lower, "__tests__")
        || changed_path_has_segment(&lower, "e2e")
    {
        roles.insert("test".to_string());
    }
    if lower.contains("schema")
        || lower.contains("openapi")
        || lower.ends_with(".prisma")
        || lower.ends_with(".proto")
        || lower.ends_with(".graphql")
        || lower.ends_with(".gql")
        || changed_path_has_segment(&lower, "migrations")
        || changed_path_has_segment(&lower, "prisma")
    {
        roles.insert("schema".to_string());
    }
    if changed_path_is_manifest(&lower) {
        roles.insert("manifest".to_string());
        roles.insert("public_boundary".to_string());
    }
    if changed_path_is_env(&lower) {
        roles.insert("env".to_string());
        roles.insert("config".to_string());
    }
    if changed_path_is_config(&lower) && !support_artifact {
        roles.insert("config".to_string());
    }
    if changed_path_is_lockfile(&lower) {
        roles.insert("lockfile".to_string());
    }
    if lower.starts_with(".github/workflows/")
        || lower.starts_with(".gitlab-ci")
        || changed_path_has_segment(&lower, ".circleci")
        || changed_path_has_segment(&lower, "buildkite")
    {
        roles.insert("ci".to_string());
    }
    if lower.starts_with("scripts/")
        || lower.starts_with("bin/")
        || lower.ends_with(".sh")
        || lower.ends_with(".bash")
        || lower.ends_with(".zsh")
    {
        roles.insert("script".to_string());
    }
    if changed_path_has_segment(&lower, "fixtures") || changed_path_has_segment(&lower, "fixture") {
        roles.insert("fixture".to_string());
    }
    if changed_path_has_segment(&lower, "generated") || lower.contains(".generated.") {
        roles.insert("generated".to_string());
    }
    if changed_path_has_segment(&lower, "archive") || changed_path_has_segment(&lower, "archives") {
        roles.insert("archive".to_string());
    }
    if support_artifact {
        roles.insert("witness".to_string());
    }
    if changed_path_has_segment(&lower, "receipts") || lower.contains("receipt") {
        roles.insert("receipt".to_string());
    }
    if lower.contains("proof")
        || lower.contains("doctor")
        || lower.contains("validate")
        || lower.contains("check")
    {
        roles.insert("proof_runner".to_string());
    }
    if lower.starts_with("makefile")
        || lower.ends_with("justfile")
        || lower.starts_with(".github/workflows/")
        || roles.contains("script")
        || roles.contains("ci")
        || roles.contains("proof_runner")
    {
        roles.insert("proof_rail".to_string());
    }
    if lower.starts_with("dist/")
        || lower.starts_with("build/")
        || changed_path_has_segment(&lower, "dist")
        || changed_path_has_segment(&lower, "build")
        || changed_path_has_segment(&lower, "target")
    {
        roles.insert("build_output".to_string());
    }
    if lower.ends_with(".md") && (lower.contains("/contracts/") || lower.contains("contract")) {
        roles.insert("contract_doc".to_string());
    }
    if lower.ends_with(".md") {
        roles.insert("docs".to_string());
    }
    if changed_path_looks_like_source(&lower) {
        for role in crate::repo::source_path_roles_for_path(&lower) {
            roles.insert(role.to_string());
        }
    }
    if roles.is_empty() && changed_path_looks_like_source(&lower) {
        roles.insert("source".to_string());
    }
    if roles.is_empty() {
        roles.insert("unknown".to_string());
    }
    roles.into_iter().collect()
}

fn changed_path_is_support_artifact(path: &str) -> bool {
    path.contains("/witness")
        || changed_path_has_segment(path, "receipts")
        || changed_path_has_segment(path, "proof")
        || changed_path_has_segment(path, "artifacts")
        || path.contains("-proof/")
}

fn changed_path_has_segment(path: &str, segment: &str) -> bool {
    path.split('/').any(|part| part == segment)
}

fn changed_path_file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn changed_path_is_manifest(path: &str) -> bool {
    matches!(
        changed_path_file_name(path),
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

fn changed_path_is_env(path: &str) -> bool {
    let name = changed_path_file_name(path);
    name == ".env" || name.starts_with(".env.")
}

fn changed_path_is_lockfile(path: &str) -> bool {
    matches!(
        changed_path_file_name(path),
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
    ) || path.ends_with(".lock")
}

fn changed_path_is_config(path: &str) -> bool {
    let name = changed_path_file_name(path);
    changed_path_is_env(path)
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
            path.rsplit('.').next().unwrap_or_default(),
            "json" | "toml" | "yaml" | "yml"
        )
}

fn changed_path_looks_like_source(path: &str) -> bool {
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
