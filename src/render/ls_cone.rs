pub fn ls(report: &LsReport, section_filter: Option<&str>) {
    println!("# Structural LS\n");
    map_prelude_line_or_snapshot_line();
    println!("Path: `{}`", report.path);
    println!("Mode: `{}`", report.mode);
    match report.mode.as_str() {
        "file" => render_ls_file(report, section_filter),
        "directory" => render_ls_directory(report, section_filter),
        "missing" => {
            if matches!(section_filter, None | Some("observed")) {
                println!("\nNo indexed file or directory anchor found.");
            }
        }
        _ => {}
    }
    if matches!(section_filter, None | Some("links")) && !report.edges.is_empty() {
        println!("\n## Links\n");
        let limit = if report.hidden.is_empty() {
            usize::MAX
        } else {
            20
        };
        grouped_edge_list("links", &report.edges, limit);
    }
    if matches!(section_filter, Some("proof")) {
        render_empty_ls_section("Proof", "Proof surfaces are not computed by ls.");
    }
    if matches!(section_filter, Some("unknown")) {
        let detail = if report.mode == "missing" {
            "No indexed anchor was found for this ls path."
        } else {
            "Typed unknowns are not computed by ls."
        };
        render_empty_ls_section("Unknown", detail);
    }
    if matches!(section_filter, None | Some("hidden")) {
        if report.hidden.is_empty() && section_filter == Some("hidden") {
            render_empty_ls_section("Hidden", "No hidden material in this ls report.");
        } else {
            hidden_section(&report.hidden);
        }
    }
    if section_filter.is_none() && !report.next.is_empty() {
        println!("\n## Expand\n");
        let next = report
            .next
            .iter()
            .map(|command| root_aware_expand(command))
            .collect::<Vec<_>>();
        println!("{}", bullet(&next, true, Some(5)));
    }
}

pub fn cone(report: &ConeReport, section_filter: Option<&str>) {
    println!("# Structural Cone\n");
    map_prelude_line_or_snapshot_line();
    println!("Anchor: `{}`", report.anchor.path);
    println!("Depth: `{}`", report.depth);
    if matches!(section_filter, None | Some("observed")) {
        render_cone_xray(report);
        render_cone_observed(report);
    }
    if matches!(section_filter, None | Some("roles")) {
        render_roles(&report.anchor);
    }
    if matches!(section_filter, None | Some("links"))
        && (!report.outgoing.is_empty()
            || !report.incoming.is_empty()
            || !report.contracts.is_empty()
            || !report.boundary.is_empty())
    {
        println!("\n## Links\n");
        grouped_edge_list("outgoing", &report.outgoing, 12);
        grouped_edge_list("incoming", &report.incoming, 12);
        grouped_edge_list("contracts", &report.contracts, 12);
        grouped_edge_list("boundary", &report.boundary, 12);
    }
    if matches!(section_filter, None | Some("proof")) {
        render_cone_proof(&report.proof);
    }
    if matches!(section_filter, None | Some("hidden")) {
        hidden_section(&report.hidden);
    }
    if matches!(section_filter, None | Some("unknown")) {
        unknown_section(&report.unknowns);
    }
    if section_filter.is_none() {
        section("Expand", &report.expand);
    }
}

fn render_cone_proof(edges: &[StructuralEdge]) {
    if edges.is_empty() {
        return;
    }
    let mut proof = Vec::new();
    let mut evidence = Vec::new();
    let mut setup = Vec::new();
    let mut soft = Vec::new();
    for edge in edges {
        match edge.edge_type.as_str() {
            "evidence_surface" => evidence.push(edge.clone()),
            "setup_support_surface" => setup.push(edge.clone()),
            "soft_evidence_surface" => soft.push(edge.clone()),
            "tests" if cone_edge_is_soft_proof(edge) => soft.push(edge.clone()),
            _ => proof.push(edge.clone()),
        }
    }
    cone_section("Proof", &proof);
    cone_section("Evidence Surfaces", &evidence);
    cone_section("Setup / Support Surfaces", &setup);
    cone_section("Soft Evidence", &soft);
    if !setup.is_empty() {
        println!(
            "\nSetup/support surfaces are connected rails such as install, codegen, migration, seed, deploy, release, watch, or dev-server steps. They are not treated as validation proof."
        );
    }
    if !soft.is_empty() {
        println!(
            "\nSoft evidence is token/name/path surface overlap. It does not replace deterministic proof or remove Unknown entries."
        );
    }
}

fn cone_edge_is_soft_proof(edge: &StructuralEdge) -> bool {
    let mediated = edge.evidence.ends_with("_via_direct_consumer")
        || edge.evidence.ends_with("_via_direct_dependency")
        || edge.evidence.ends_with("_via_local_symbol_consumer");
    let base = crate::proof_classification::proof_base_evidence(&edge.evidence);
    mediated
        || matches!(
            base,
            "test_name"
                | "e2e_surface_phrase"
                | "e2e_path_surface"
                | "test_surface_phrase"
                | "test_surface_tokens"
                | "script_path_token"
                | "script_surface_match"
        )
        || (edge.strength < crate::model::EvidenceStrength::High
            && !matches!(
                base,
                "test_import"
                    | "test_imported_symbol_reference"
                    | "test_reexported_symbol_reference"
                    | "test_support_import"
                    | "test_symbol_reference"
                    | "e2e_route"
            ))
}

fn edge_location_summary(edge: &StructuralEdge) -> String {
    let Some(first) = edge.locations.first() else {
        return "unknown".to_string();
    };
    let suffix = if edge.locations.len() > 1 {
        format!(" +{}", edge.locations.len() - 1)
    } else {
        String::new()
    };
    let base = if first.path == "aggregate" {
        "aggregate".to_string()
    } else if let Some(line) = first.line_start {
        format!("{}:{line}", first.path)
    } else {
        first.path.clone()
    };
    format!("{}{}", code(&base), suffix)
}

fn render_ls_file(report: &LsReport, section_filter: Option<&str>) {
    let Some(anchor) = &report.anchor else {
        return;
    };
    if matches!(section_filter, None | Some("observed")) {
        render_anchor_summary("Observed", anchor);
        if !anchor.symbols.is_empty() {
            println!("\n## Observed Symbols\n");
            for symbol in anchor.symbols.iter().take(30) {
                println!(
                    "- `{}` [{}; exported={}; lines={}-{}]",
                    symbol.name, symbol.kind, symbol.exported, symbol.line_start, symbol.line_end
                );
            }
            let hidden_count = anchor.symbols.len().saturating_sub(30);
            if hidden_count > 0 {
                println!("- additional symbols: {hidden_count}");
            }
        }
    }
    if matches!(section_filter, None | Some("roles")) {
        render_roles(anchor);
    }
    if matches!(section_filter, None | Some("links")) {
        section("Exports", &anchor.exports);
        section("Imports", &anchor.imports);
    }
}

fn render_anchor_summary(title: &str, anchor: &crate::model::FileSummary) {
    println!("\n## {title}\n");
    println!("- kind: `{}`", anchor.kind);
    println!(
        "- package: `{}`",
        anchor.package.as_deref().unwrap_or("none")
    );
    println!("- language: `{}`", anchor.language);
    println!("- lines: `{}`", anchor.lines);
    if !anchor.roles.is_empty() {
        println!("- surface hints: {}", anchor.roles.join(", "));
    }
    println!("- symbols: `{}`", anchor.symbols.len());
    println!("- imported by: `{}`", anchor.imported_by_count);
}

fn render_cone_observed(report: &ConeReport) {
    render_anchor_summary("Observed", &report.anchor);
    render_declared_env_keys(&report.declared_env);
}

fn render_declared_env_keys(keys: &[crate::model::EnvDeclaration]) {
    if keys.is_empty() {
        return;
    }
    println!("- declared env keys: `{}`", keys.len());
    const DECLARED_ENV_RENDER_LIMIT: usize = 12;
    for declaration in keys.iter().take(DECLARED_ENV_RENDER_LIMIT) {
        println!(
            "  - `{}` {}",
            declaration.key,
            code(&format!("{}:{}", declaration.path, declaration.line_start))
        );
    }
    let hidden = keys.len().saturating_sub(DECLARED_ENV_RENDER_LIMIT);
    if hidden > 0 {
        println!("  - additional env keys: {hidden}");
    }
}

fn render_ls_directory(report: &LsReport, section_filter: Option<&str>) {
    if matches!(section_filter, Some("roles")) {
        render_ls_directory_roles(report);
        return;
    }
    if !matches!(section_filter, None | Some("observed")) {
        return;
    }
    if report.directory.is_empty() {
        println!("\nNo indexed files under this directory.");
        return;
    }
    println!("\n## Observed\n");
    for surface in &report.directory {
        let role = surface.role.as_deref().unwrap_or("none");
        let strength = format!("{:?}", surface.strength).to_ascii_lowercase();
        println!(
            "- `{}` [hint={}; count={}; {}; {}]",
            surface.kind,
            role,
            surface.count,
            public_evidence_label(&surface.evidence),
            strength
        );
        if let Some(path) = &surface.path {
            println!("  path: `{path}`");
        }
        if !surface.examples.is_empty() {
            let examples = surface
                .examples
                .iter()
                .map(|example| code(example))
                .collect::<Vec<_>>()
                .join(", ");
            println!("  examples: {examples}");
        }
        if surface.hidden_count > 0 {
            println!("  additional examples: {}", surface.hidden_count);
        }
    }
    if report.path == "." {
        boundary_facts_section(&report.boundary_facts, false, false);
    }
}

fn render_ls_directory_roles(report: &LsReport) {
    let mut roles: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for surface in &report.directory {
        let role = surface.role.as_deref().unwrap_or("none");
        *roles.entry(role.to_string()).or_default() += surface.count;
    }
    if roles.is_empty() {
        render_empty_ls_section("Surface Hints", "No surface hints found in this ls report.");
        return;
    }
    println!("\n## Surface Hints\n");
    println!("Derived from deterministic path/name/extension/manifest patterns. Not intent, correctness, or ownership truth.\n");
    for (role, count) in roles {
        println!("- `{role}`: `{count}` surfaces");
    }
}

fn render_empty_ls_section(title: &str, detail: &str) {
    println!("\n## {title}\n");
    println!("- {detail}");
}

fn render_roles(anchor: &crate::model::FileSummary) {
    let roles = canonical_roles(anchor);
    if roles.is_empty() {
        return;
    }
    println!("\n## Surface Hints\n");
    println!("Derived from deterministic path/name/extension/manifest patterns. Not intent, correctness, or ownership truth.\n");
    println!("{}", bullet(&roles, true, None));
}

fn canonical_roles(anchor: &crate::model::FileSummary) -> Vec<String> {
    let mut roles = std::collections::BTreeSet::new();
    let local = anchor.roles.iter().map(String::as_str).collect::<Vec<_>>();
    let path = anchor.path.to_ascii_lowercase();
    let support_artifact = support_artifact_hint_path(&path)
        || local.iter().any(|role| {
            matches!(
                *role,
                "receipt" | "witness" | "fixture" | "generated" | "archive" | "build_output"
            )
        });
    if local.iter().any(|role| matches!(*role, "test" | "e2e_test" | "test_support")) {
        roles.insert("test".to_string());
    }
    if local.contains(&"public_boundary") || anchor.kind == "public_boundary" {
        roles.insert("public_boundary".to_string());
    }
    if local.contains(&"manifest") || matches!(path.as_str(), "package.json" | "cargo.toml") {
        roles.insert("manifest".to_string());
    }
    if local.contains(&"env_config") || anchor.kind == "env_config" || path.contains(".env") {
        roles.insert("env".to_string());
    }
    if local.contains(&"runtime_config")
        || anchor.kind == "runtime_config"
        || (!support_artifact && (anchor.kind == "config" || anchor.language == "config"))
    {
        roles.insert("config".to_string());
    }
    if local.contains(&"lockfile") || anchor.kind == "lockfile" {
        roles.insert("lockfile".to_string());
    }
    if local.contains(&"docs") || anchor.kind == "docs" || path.ends_with(".md") {
        roles.insert("docs".to_string());
    }
    if local.contains(&"schema_contract") || anchor.kind == "schema_contract" {
        roles.insert("schema".to_string());
    }
    if local.contains(&"build_ci") || anchor.kind == "build_ci" {
        roles.insert("ci".to_string());
    }
    if anchor.kind == "script" || anchor.path.starts_with("test: ") {
        roles.insert("script".to_string());
    }
    if local.contains(&"proof_runner") {
        roles.insert("proof_runner".to_string());
    }
    if local.contains(&"owner_doc") {
        roles.insert("owner_doc".to_string());
    }
    if local.contains(&"doctor") {
        roles.insert("doctor".to_string());
    }
    if local.contains(&"receipt") {
        roles.insert("receipt".to_string());
    }
    if local.contains(&"witness") {
        roles.insert("witness".to_string());
    }
    for role in [
        "application",
        "service",
        "domain",
        "controller",
        "module",
        "repository",
        "package_graph",
        "role_classifier",
        "script_catalog",
        "cli_surface",
        "map_surface",
        "extractor",
        "config_loader",
        "evidence_surface",
    ] {
        if local.contains(&role) || anchor.kind == role {
            roles.insert(role.to_string());
        }
    }
    if local.contains(&"fixture") || path.contains("/fixtures/") || path.starts_with("fixtures/") {
        roles.insert("fixture".to_string());
    }
    if local.contains(&"generated") {
        roles.insert("generated".to_string());
    }
    if path.contains("/archive/") || path.starts_with("archive/") || path.contains("/archives/") {
        roles.insert("archive".to_string());
    }
    if support_artifact_hint_path(&path) {
        roles.insert("witness".to_string());
    }
    if path.contains("/dist/") || path.starts_with("dist/") || path.contains("/build/") || path.starts_with("build/") {
        roles.insert("build_output".to_string());
    }
    if path.ends_with(".md") && (path.contains("/contracts/") || path.contains("contract")) {
        roles.insert("contract_doc".to_string());
    }
    if roles.is_empty() && looks_like_source_anchor(anchor) {
        roles.insert("source".to_string());
    }
    if roles.is_empty() {
        roles.insert("unknown".to_string());
    }
    roles.into_iter().collect()
}

fn support_artifact_hint_path(path: &str) -> bool {
    path.contains("/witness")
        || path.contains("/receipts/")
        || path.starts_with("receipts/")
        || path.contains("/witnesses/")
        || path.starts_with("witnesses/")
        || path.contains("/artifacts/")
        || path.starts_with("artifacts/")
        || path.contains("-proof/")
        || path.contains("/proof/")
}

fn looks_like_source_anchor(anchor: &crate::model::FileSummary) -> bool {
    anchor.kind == "source"
        || !anchor.symbols.is_empty()
        || !anchor.imports.is_empty()
        || !anchor.exports.is_empty()
        || matches!(
            anchor.language.as_str(),
            "rust"
                | "typescript"
                | "tsx"
                | "javascript"
                | "jsx"
                | "python"
                | "go"
                | "swift"
                | "kotlin"
                | "java"
                | "c"
                | "cpp"
        )
}
