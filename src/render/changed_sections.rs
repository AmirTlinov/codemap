fn changed_roles_section(report: &ChangedReport, force: bool) {
    let mut paths_with_summaries = std::collections::BTreeSet::new();
    let mut grouped: std::collections::BTreeMap<String, Vec<(String, String)>> =
        std::collections::BTreeMap::new();
    for file in &report.changed {
        paths_with_summaries.insert(file.path.clone());
        for role in canonical_roles(file) {
            grouped.entry(role).or_default().push((
                file.path.clone(),
                format!("[{}; {}]", file.kind, file.language),
            ));
        }
    }
    for change in &report.git_state {
        if paths_with_summaries.contains(&change.path) {
            continue;
        }
        for role in changed_roles_for_path(&change.path) {
            grouped.entry(role).or_default().push((
                change.path.clone(),
                format!("[git; status={}]", change.status),
            ));
        }
    }
    if grouped.is_empty() {
        if force {
            println!("\n## Mutation Roles\n");
            println!("No changed surfaces to classify.");
        }
        return;
    }
    for entries in grouped.values_mut() {
        entries.sort();
        entries.dedup();
    }
    println!("\n## Mutation Roles\n");
    let paths = grouped
        .values()
        .flat_map(|entries| entries.iter().map(|(path, _)| path.as_str()))
        .collect::<Vec<_>>();
    let prefix = changed_common_dir_prefix(&paths);
    if let Some(prefix) = &prefix {
        println!("prefix: `{prefix}`");
    }
    let mut rendered = std::collections::BTreeSet::new();
    for role in CHANGED_ROLE_ORDER {
        if let Some(entries) = grouped.get(*role) {
            changed_role_entries(role, entries, prefix.as_deref(), report.display_limit);
            rendered.insert((*role).to_string());
        }
    }
    for (role, entries) in grouped {
        if rendered.contains(&role) {
            continue;
        }
        changed_role_entries(&role, &entries, prefix.as_deref(), report.display_limit);
    }
}

const CHANGED_ROLE_ORDER: &[&str] = &[
    "source",
    "test",
    "schema",
    "manifest",
    "env",
    "config",
    "lockfile",
    "docs",
    "public_boundary",
    "contract_doc",
    "ci",
    "script",
    "fixture",
    "generated",
    "archive",
    "witness",
    "build_output",
    "unknown",
];

fn changed_role_entries(
    role: &str,
    entries: &[(String, String)],
    prefix: Option<&str>,
    limit: usize,
) {
    println!("- `{role}`: `{}`", entries.len());
    for (path, meta) in entries.iter().take(limit) {
        println!("  - `{}` {}", changed_relative_path(path, prefix), meta);
    }
    let hidden = entries.len().saturating_sub(limit);
    if hidden > 0 {
        println!("  - hidden: `{hidden}` surfaces");
    }
}

fn changed_roles_for_path(path: &str) -> Vec<String> {
    let lower = path.to_ascii_lowercase();
    let mut roles = std::collections::BTreeSet::new();
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
    if changed_path_is_config(&lower) {
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
    if lower.contains("/witness")
        || changed_path_has_segment(&lower, "receipts")
        || changed_path_has_segment(&lower, "proof")
    {
        roles.insert("witness".to_string());
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
    if roles.is_empty() && changed_path_looks_like_source(&lower) {
        roles.insert("source".to_string());
    }
    if roles.is_empty() {
        roles.insert("unknown".to_string());
    }
    roles.into_iter().collect()
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
        || matches!(path.rsplit('.').next().unwrap_or_default(), "json" | "toml" | "yaml" | "yml")
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

fn changed_links_section(report: &ChangedReport, compact: bool, force: bool) {
    if report.impact.is_empty() {
        if force {
            println!("\n## Links\n");
            println!("No deterministic links found.");
        }
        return;
    }
    println!("\n## Links\n");
    if compact {
        changed_link_summary_lines(&report.impact);
        return;
    }
    for cluster in &report.impact {
        println!("\n### `{}`", cluster.id);
        if !cluster.changed.is_empty() {
            println!("changed:");
            println!("{}", bullet(&cluster.changed, true, Some(10)));
        }
        if !cluster.reasons.is_empty() {
            println!("facts:");
            println!("{}", bullet(&cluster.reasons, false, Some(6)));
        }
        grouped_edge_list("direct consumers", &cluster.direct_consumers, 8);
        grouped_edge_list("cross-boundary consumers", &cluster.cross_boundary_consumers, 8);
        grouped_edge_list("contract links", &cluster.contract_links, 8);
        if !cluster.proof.is_empty() {
            println!("proof links: {}", cluster.proof.len());
        }
    }
}

fn changed_link_summary_lines(clusters: &[ImpactCluster]) {
    let paths = clusters
        .iter()
        .filter_map(|cluster| cluster.id.strip_prefix("changed:"))
        .collect::<Vec<_>>();
    let prefix = changed_common_dir_prefix(&paths);
    if let Some(prefix) = &prefix {
        println!("prefix: `{prefix}`");
    }
    for cluster in clusters {
        let label = cluster
            .id
            .strip_prefix("changed:")
            .map(|path| changed_relative_path(path, prefix.as_deref()))
            .unwrap_or_else(|| cluster.id.clone());
        println!(
            "- `{}` [direct={}; cross={}; contract={}; proof={}]",
            label,
            cluster.direct_consumers.len(),
            cluster.cross_boundary_consumers.len(),
            cluster.contract_links.len(),
            cluster.proof.len()
        );
        if !cluster.reasons.is_empty() {
            println!("  facts: {}", cluster.reasons.join("; "));
        }
    }
}

fn changed_unknown_section(values: &[Unknown], force: bool) {
    if values.is_empty() {
        if force {
            println!("\n## Unknown\n");
            println!("No Unknown entries recorded for this selector.");
        }
        return;
    }
    unknown_section(values);
}

fn changed_hidden_section(hidden: &[crate::model::HiddenGroup], force: bool) {
    if hidden.is_empty() {
        if force {
            println!("\n## Hidden\n");
            println!("No hidden material.");
        }
        return;
    }
    hidden_section(hidden);
}
