pub fn ls(report: &LsReport) {
    println!("# Structural LS\n");
    println!("Path: `{}`", report.path);
    println!("Mode: `{}`", report.mode);
    match report.mode.as_str() {
        "file" => render_ls_file(report),
        "directory" => render_ls_directory(report),
        "missing" => {
            println!("\nNo indexed file or directory anchor found.");
        }
        _ => {}
    }
    if !report.edges.is_empty() {
        cone_section("Links", &report.edges);
    }
    hidden_section(&report.hidden);
    if !report.next.is_empty() {
        println!("\n## Expand\n");
        let next = report
            .next
            .iter()
            .map(|command| root_aware_expand(command))
            .collect::<Vec<_>>();
        println!("{}", bullet(&next, true, Some(5)));
    }
}

pub fn cone(report: &ConeReport) {
    println!("# Structural Cone\n");
    println!("Anchor: `{}`", report.anchor.path);
    println!("Depth: `{}`", report.depth);
    render_anchor_summary("Observed", &report.anchor);
    render_roles(&report.anchor);
    if !report.outgoing.is_empty() || !report.incoming.is_empty() || !report.contracts.is_empty() || !report.boundary.is_empty() {
        println!("\n## Links\n");
        grouped_edge_list("outgoing", &report.outgoing, 20);
        grouped_edge_list("incoming", &report.incoming, 20);
        grouped_edge_list("contracts", &report.contracts, 20);
        grouped_edge_list("boundary", &report.boundary, 20);
    }
    cone_section("Proof", &report.proof);
    hidden_section(&report.hidden);
    unknown_section(&report.unknowns);
    section("Expand", &report.expand);
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

fn render_ls_file(report: &LsReport) {
    let Some(anchor) = &report.anchor else {
        return;
    };
    render_anchor_summary("Observed", anchor);
    render_roles(anchor);
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
            println!("- hidden: {hidden_count} symbols");
        }
    }
    section("Exports", &anchor.exports);
    section("Imports", &anchor.imports);
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
        println!("- local tags: {}", anchor.roles.join(", "));
    }
    println!("- symbols: `{}`", anchor.symbols.len());
    println!("- imported by: `{}`", anchor.imported_by_count);
}

fn render_ls_directory(report: &LsReport) {
    if report.directory.is_empty() {
        println!("\nNo indexed files under this directory.");
        return;
    }
    println!("\n## Observed\n");
    for surface in &report.directory {
        let role = surface.role.as_deref().unwrap_or("none");
        let strength = format!("{:?}", surface.strength).to_ascii_lowercase();
        println!(
            "- `{}` [role={}; count={}; {}; {}]",
            surface.kind, role, surface.count, surface.evidence, strength
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
            println!("  hidden: {} examples", surface.hidden_count);
        }
    }
}

fn render_roles(anchor: &crate::model::FileSummary) {
    let roles = canonical_roles(anchor);
    if roles.is_empty() {
        return;
    }
    println!("\n## Roles\n");
    println!("{}", bullet(&roles, true, None));
}

fn canonical_roles(anchor: &crate::model::FileSummary) -> Vec<String> {
    let mut roles = std::collections::BTreeSet::new();
    let local = anchor.roles.iter().map(String::as_str).collect::<Vec<_>>();
    let path = anchor.path.to_ascii_lowercase();
    if local.iter().any(|role| matches!(*role, "test" | "e2e_test" | "test_support")) {
        roles.insert("test".to_string());
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
    if local.contains(&"fixture") || path.contains("/fixtures/") || path.starts_with("fixtures/") {
        roles.insert("fixture".to_string());
    }
    if local.contains(&"generated") {
        roles.insert("generated".to_string());
    }
    if path.contains("/archive/") || path.starts_with("archive/") || path.contains("/archives/") {
        roles.insert("archive".to_string());
    }
    if path.contains("/witness") || path.contains("/receipts/") || path.contains("/proof/") {
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
