fn is_receipt_surface(rel: &str, name: &str, ext: &str) -> bool {
    matches!(ext, "json" | "jsonl")
        && (rel.contains("/receipts/")
            || rel.starts_with("receipts/")
            || name.contains("receipt")
            || name.contains("witness"))
}

fn is_witness_surface(rel: &str, name: &str, ext: &str) -> bool {
    matches!(ext, "json" | "jsonl" | "md" | "txt")
        && (rel.contains("/witnesses/")
            || rel.starts_with("witnesses/")
            || name.contains("witness"))
}

fn is_owner_doc_surface(rel: &str, name: &str, ext: &str) -> bool {
    ext == "md"
        && (name.starts_with("qwen-")
            || name.contains("frontier")
            || name.contains("roadmap")
            || name.contains("owner")
            || name.contains("direction")
            || name.contains("manifest")
            || rel.contains("/owner-docs/")
            || rel.contains("/experiments/"))
}

fn is_proof_runner_surface(rel: &str, name: &str, ext: &str, tokens: &BTreeSet<String>) -> bool {
    (is_source_ext(ext) || is_script_ext(ext))
        && (rel.starts_with("tools/")
            || rel.starts_with("scripts/")
            || rel.contains("/tools/")
            || rel.contains("/scripts/")
            || rel.contains("/experiments/"))
        && (name.starts_with("run_")
            || name.starts_with("run-")
            || name.contains("runner")
            || tokens.contains("proof")
            || tokens.contains("validate")
            || tokens.contains("doctor")
            || tokens.contains("qwen")
            || tokens.contains("receipt")
            || tokens.contains("witness"))
}

fn is_migration_surface(rel: &str, name: &str, ext: &str) -> bool {
    rel.contains("/migrations/")
        || rel.starts_with("migrations/")
        || name.contains("migration")
        || (ext == "sql" && rel.contains("/db/"))
}

fn is_deploy_surface(rel: &str, name: &str, ext: &str) -> bool {
    is_runtime_config_surface(rel, name)
        || rel.starts_with("deploy/")
        || rel.starts_with("deployment/")
        || rel.starts_with("infra/")
        || rel.contains("/deploy/")
        || rel.contains("/deployment/")
        || rel.contains("/k8s/")
        || matches!(name, "helmfile.yaml" | "helmfile.yml" | "skaffold.yaml")
        || (matches!(ext, "yaml" | "yml") && rel.contains("/helm/"))
}

fn is_entrypoint_surface(rel: &str, name: &str, ext: &str) -> bool {
    matches!(
        name,
        "main.rs" | "main.go" | "__main__.py" | "main.ts" | "main.js" | "cli.ts" | "cli.js"
    ) || rel.contains("/src/bin/")
        || (is_source_ext(ext) && (name.starts_with("server.") || name.starts_with("worker.")))
}

fn is_runtime_surface(info: &FileInfo) -> bool {
    info.roles.contains("entrypoint")
        || info.roles.contains("runtime_config")
        || info.roles.contains("runtime_state")
        || info.roles.contains("build_ci")
}

fn is_internal_api_surface(rel: &str, name: &str, ext: &str) -> bool {
    is_source_ext(ext)
        && (rel.contains("/internal/")
            || rel.starts_with("internal/")
            || name.contains("internal-api")
            || name.contains("internal_api"))
        && (rel.contains("api") || rel.contains("route") || rel.contains("handler"))
}

fn is_doctor_surface(rel: &str, name: &str, ext: &str) -> bool {
    (is_source_ext(ext) || matches!(ext, "sh" | "bash" | "zsh" | "py"))
        && (name.contains("doctor") || rel.contains("/doctor/"))
}
