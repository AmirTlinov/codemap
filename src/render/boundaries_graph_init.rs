pub fn boundaries(findings: &[BoundaryFinding]) {
    println!("# Boundary Check\n");
    if findings.is_empty() {
        println!("No boundary findings.");
        return;
    }
    let rows = findings
        .iter()
        .map(|f| {
            vec![
                f.status.clone(),
                code(&f.from),
                code(&f.to),
                f.strength.clone(),
                f.reason.clone(),
            ]
        })
        .collect();
    println!(
        "{}",
        table(&["Status", "From", "To", "Strength", "Reason"], rows)
    );
    let recoveries: Vec<_> = findings.iter().filter(|f| !f.recovery.is_empty()).collect();
    if !recoveries.is_empty() {
        println!("\n## Recovery paths");
        for finding in recoveries {
            println!("\n### `{}` -> `{}`\n", finding.from, finding.to);
            println!("{}", bullet(&finding.recovery, false, None));
        }
    }
}

pub fn graph_markdown(graph: &GraphLens) {
    println!("# Graph Lens: {}\n", graph.lens);
    println!("Domain: `{}` (`{}`)\n", graph.domain.id, graph.domain.path);
    println!("## Nodes\n");
    println!("{}", bullet(&graph.nodes, true, Some(30)));
    println!("\n## Edges\n");
    if graph.edges.is_empty() {
        println!("- none");
    } else {
        let rows = graph
            .edges
            .iter()
            .map(|e| vec![code(&e.from), e.edge_type.clone(), code(&e.to)])
            .collect();
        println!("{}", table(&["From", "Type", "To"], rows));
    }
}

pub fn graph_mermaid(graph: &GraphLens) {
    println!("graph TD");
    if graph.nodes.is_empty() && graph.edges.is_empty() {
        println!("  Empty[\"No graph data for lens\"]");
        return;
    }
    for node in &graph.nodes {
        println!("  {}[\"{}\"]", mermaid_id(node), escape_mermaid(node));
    }
    for edge in &graph.edges {
        println!(
            "  {} -->|{}| {}",
            mermaid_id(&edge.from),
            escape_mermaid(&edge.edge_type),
            mermaid_id(&edge.to)
        );
    }
}

pub fn init_suggestion(path: Option<&str>) {
    println!("{}", suggested_ctx_yml_for(path));
}

pub fn agents_bootloader() -> &'static str {
    "# Agent Bootstrap\n\nFor coding tasks in this repository, map the relevant code before broad manual scanning:\n\n```bash\ncodemap ls .\ncodemap ls <scope-or-file>\ncodemap cone <scope-or-file> --depth 1\n```\n\nAfter edits:\n\n```bash\ncodemap impact --changed\ncodemap proof --changed\n```\n\nUse `codemap cone <anchor> --depth 2` only when the first cone is structurally empty, crosses a public/package/schema boundary, or the proof surface is missing.\n"
}

pub fn global_instruction() -> &'static str {
    "For coding tasks, if `codemap` is available in PATH, begin with a bounded structural map:\n\n```bash\ncodemap ls .\ncodemap ls <scope-or-file>\ncodemap cone <scope-or-file> --depth 1\n```\n\nAfter edits:\n\n```bash\ncodemap impact --changed\ncodemap proof --changed\n```\n\nRead code lines after choosing anchors from the map. Widen with `codemap cone <anchor> --depth 2` only when structural edges, public/package/schema boundaries, or proof surfaces require it.\n"
}

pub fn suggested_ctx_yml_for(path: Option<&str>) -> String {
    let domain_id = path
        .and_then(|p| p.trim_end_matches('/').rsplit('/').next())
        .filter(|p| !p.is_empty() && *p != ".")
        .unwrap_or("repo");
    format!(
        "version: 1\n\ndomain:\n  id: {domain_id}\n\nboundaries:\n  forbidden: []\n\nverification:\n  default: []\n"
    )
}
