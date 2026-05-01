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
        println!("\n## Edges\n");
        let rows = report
            .edges
            .iter()
            .map(|edge| {
                vec![
                    code(&edge.from),
                    edge.edge_type.clone(),
                    code(&edge.to),
                    edge.evidence.clone(),
                    format!("{:?}", edge.strength).to_ascii_lowercase(),
                    edge_location_summary(edge),
                ]
            })
            .collect();
        println!(
            "{}",
            table(
                &["From", "Type", "To", "Evidence", "Strength", "Where"],
                rows
            )
        );
    }
    if !report.hidden.is_empty() {
        println!("\n## Hidden\n");
        let rows = report
            .hidden
            .iter()
            .map(|hidden| {
                vec![
                    hidden.reason.clone(),
                    hidden.count.to_string(),
                    code(&hidden.expand),
                ]
            })
            .collect();
        println!("{}", table(&["Reason", "Count", "Expand"], rows));
    }
    if !report.next.is_empty() {
        println!("\n## Next\n");
        println!("{}", bullet(&report.next, true, Some(5)));
    }
}

pub fn cone(report: &ConeReport) {
    println!("# Structural Cone\n");
    println!("Anchor: `{}`", report.anchor.path);
    println!("Depth: `{}`", report.depth);
    println!(
        "\n{}",
        table(
            &["Field", "Value"],
            vec![
                vec!["Kind".to_string(), report.anchor.kind.clone()],
                vec![
                    "Package".to_string(),
                    report
                        .anchor
                        .package
                        .clone()
                        .unwrap_or_else(|| "none".to_string()),
                ],
                vec!["Language".to_string(), report.anchor.language.clone()],
                vec!["Lines".to_string(), report.anchor.lines.to_string()],
                vec![
                    "Symbols".to_string(),
                    report.anchor.symbols.len().to_string(),
                ],
                vec![
                    "Imported by".to_string(),
                    report.anchor.imported_by_count.to_string(),
                ],
            ],
        )
    );
    cone_section("Outgoing", &report.outgoing);
    cone_section("Incoming", &report.incoming);
    cone_section("Proof", &report.proof);
    cone_section("Contracts", &report.contracts);
    cone_section("Boundary", &report.boundary);
    if !report.hidden.is_empty() {
        println!("\n## Hidden\n");
        let rows = report
            .hidden
            .iter()
            .map(|hidden| {
                vec![
                    hidden.reason.clone(),
                    hidden.count.to_string(),
                    code(&hidden.expand),
                ]
            })
            .collect();
        println!("{}", table(&["Reason", "Count", "Expand"], rows));
    }
    unknown_section(&report.unknowns);
    section("Expand", &report.expand);
}

fn cone_section(title: &str, edges: &[StructuralEdge]) {
    if edges.is_empty() {
        return;
    }
    println!("\n## {title}\n");
    let rows = edges
        .iter()
        .map(|edge| {
            vec![
                code(&edge.from),
                edge.edge_type.clone(),
                code(&edge.to),
                edge.evidence.clone(),
                format!("{:?}", edge.strength).to_ascii_lowercase(),
                edge_location_summary(edge),
            ]
        })
        .collect();
    println!(
        "{}",
        table(
            &["From", "Type", "To", "Evidence", "Strength", "Where"],
            rows
        )
    );
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
    println!(
        "\n{}",
        table(
            &["Field", "Value"],
            vec![
                vec!["Kind".to_string(), anchor.kind.clone()],
                vec![
                    "Package".to_string(),
                    anchor.package.clone().unwrap_or_else(|| "none".to_string()),
                ],
                vec!["Language".to_string(), anchor.language.clone()],
                vec!["Lines".to_string(), anchor.lines.to_string()],
                vec![
                    "Roles".to_string(),
                    if anchor.roles.is_empty() {
                        "none".to_string()
                    } else {
                        anchor.roles.join(", ")
                    },
                ],
                vec![
                    "Imported by".to_string(),
                    anchor.imported_by_count.to_string(),
                ],
            ],
        )
    );
    if !anchor.symbols.is_empty() {
        println!("\n## Symbols\n");
        let rows = anchor
            .symbols
            .iter()
            .take(30)
            .map(|symbol| {
                vec![
                    symbol.name.clone(),
                    symbol.kind.clone(),
                    symbol.exported.to_string(),
                    format!("{}-{}", symbol.line_start, symbol.line_end),
                ]
            })
            .collect();
        println!("{}", table(&["Name", "Kind", "Exported", "Lines"], rows));
    }
    section("Exports", &anchor.exports);
    section("Imports", &anchor.imports);
}

fn render_ls_directory(report: &LsReport) {
    if report.directory.is_empty() {
        println!("\nNo indexed files under this directory.");
        return;
    }
    println!("\n## Surfaces\n");
    let rows = report
        .directory
        .iter()
        .map(|surface| {
            vec![
                surface.kind.clone(),
                surface
                    .role
                    .clone()
                    .unwrap_or_else(|| "none".to_string()),
                surface.count.to_string(),
                surface.evidence.clone(),
                format!("{:?}", surface.strength).to_ascii_lowercase(),
                surface
                    .examples
                    .iter()
                    .map(|example| code(example))
                    .collect::<Vec<_>>()
                    .join(", "),
            ]
        })
        .collect();
    println!(
        "{}",
        table(
            &["Kind", "Role", "Count", "Evidence", "Strength", "Examples"],
            rows
        )
    );
}
