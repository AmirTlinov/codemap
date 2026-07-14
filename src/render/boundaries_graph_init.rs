// Responsibility: render-boundaries-graph-init
use crate::model::{BoundaryFinding, GraphEdge, GraphLens};
use crate::render::{bullet, code, escape_mermaid, mermaid_id, root_aware_expand, table};

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
            .map(|e| {
                vec![
                    code(&e.from),
                    e.edge_type.clone(),
                    code(&e.to),
                    e.evidence.clone(),
                    format!("{:?}", e.strength).to_ascii_lowercase(),
                    graph_edge_location_summary(e),
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
    if !graph.hidden.is_empty() {
        println!("\n## Hidden\n");
        let rows = graph
            .hidden
            .iter()
            .map(|hidden| {
                vec![
                    hidden.reason.clone(),
                    hidden.count.to_string(),
                    code(&root_aware_expand(&hidden.expand)),
                ]
            })
            .collect();
        println!("{}", table(&["Reason", "Count", "Expand"], rows));
    }
}

pub fn graph_mermaid(graph: &GraphLens) {
    println!("graph TD");
    if graph.nodes.is_empty() && graph.edges.is_empty() {
        println!("  Empty[\"No graph data for lens\"]");
        return;
    }
    for hidden in &graph.hidden {
        println!(
            "  %% hidden: {} ({}); expand: {}",
            escape_mermaid(&hidden.reason),
            hidden.count,
            escape_mermaid(&root_aware_expand(&hidden.expand))
        );
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

fn graph_edge_location_summary(edge: &GraphEdge) -> String {
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

pub fn init_suggestion(path: Option<&str>) {
    println!("{}", suggested_codemap_yml_for(path));
}

pub fn agents_bootloader() -> &'static str {
    "# Agent Bootstrap\n\nFor coding tasks in this repository, use `codemap` as the deterministic structural map before broad manual scanning:\n\n```bash\ncodemap ls .\ncodemap ls <scope-or-file>\ncodemap cone <scope-or-file> --depth 1\n```\n\nAfter edits, use one overview first:\n\n```bash\ncodemap changed\ncodemap proof changed\n```\n\nFollow exact expand commands from the map when you need focused lenses such as `runtime`, `contract`, `flow`, `boundary-map`, `siblings`, `place`, `delete`, `diff-map`, `impact`, `proof-map`, or `graph`.\n"
}

pub fn global_instruction() -> &'static str {
    "For coding tasks, if `codemap` is available in PATH, begin with the small daily structural map surface:\n\n```bash\ncodemap ls .\ncodemap ls <scope-or-file>\ncodemap cone <scope-or-file> --depth 1\n```\n\nAfter edits, use one changed overview and then proof:\n\n```bash\ncodemap changed\ncodemap proof changed\n```\n\nFollow exact expand commands from the output for focused lenses such as `runtime`, `contract`, `flow`, `boundary-map`, `siblings`, `place`, `delete`, `diff-map`, `impact`, `proof-map`, or `graph`. Read code lines after choosing anchors from the map.\n"
}

pub fn suggested_codemap_yml_for(path: Option<&str>) -> String {
    let domain_id = path
        .and_then(|p| p.trim_end_matches('/').rsplit('/').next())
        .filter(|p| !p.is_empty() && *p != ".")
        .unwrap_or("repo");
    format!(
        "version: 1\n\ndomain:\n  id: {domain_id}\n\nboundaries:\n  forbidden: []\n\nverification:\n  default: []\n"
    )
}
