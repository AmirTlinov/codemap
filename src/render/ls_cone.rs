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
        cone_section("Edges", &report.edges);
    }
    hidden_section(&report.hidden);
    if !report.next.is_empty() {
        println!("\n## Next\n");
        println!("{}", bullet(&report.next, true, Some(5)));
    }
}

pub fn cone(report: &ConeReport) {
    println!("# Structural Cone\n");
    println!("Anchor: `{}`", report.anchor.path);
    println!("Depth: `{}`", report.depth);
    render_anchor_summary("Anchor Summary", &report.anchor);
    cone_section("Outgoing", &report.outgoing);
    cone_section("Incoming", &report.incoming);
    cone_section("Proof", &report.proof);
    cone_section("Contracts", &report.contracts);
    cone_section("Boundary", &report.boundary);
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
    render_anchor_summary("File", anchor);
    if !anchor.symbols.is_empty() {
        println!("\n## Symbols\n");
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
        println!("- roles: {}", anchor.roles.join(", "));
    }
    println!("- symbols: `{}`", anchor.symbols.len());
    println!("- imported by: `{}`", anchor.imported_by_count);
}

fn render_ls_directory(report: &LsReport) {
    if report.directory.is_empty() {
        println!("\nNo indexed files under this directory.");
        return;
    }
    println!("\n## Surfaces\n");
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
