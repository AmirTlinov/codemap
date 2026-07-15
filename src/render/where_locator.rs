// Responsibility: render-where-locator
use crate::model::{ConeReport, WhereDefinition, WhereReport};
use crate::render::{
    cone_links_empty, disclaimer, edge_location_summary, grouped_edge_list, hidden_section,
    public_evidence_label, render_cone_links, render_cone_xray, root_aware_expand, section,
    unknown_section, xray_edge_label,
};

pub fn where_locator(report: &WhereReport) {
    println!("# Structural Where\n");
    println!("Query: `{}`", report.query);
    println!(
        "Kind filter: `{}`",
        report.kind_filter.as_deref().unwrap_or("none")
    );
    println!("Matches: `{}`", report.total_matches);

    match report.definitions.len() {
        0 => render_where_not_found(report),
        1 => render_where_single(report),
        _ => render_where_multi(report),
    }
}

fn render_where_not_found(report: &WhereReport) {
    println!("\n## Not Found\n");
    println!("No indexed definition matches this exact symbol name.");
    if !report.soft_suggestions.is_empty() {
        println!("\n## Soft Name Matches\n");
        disclaimer(
            "Soft substring matches over symbol names. Deterministic, not ranked, not an answer.",
        );
        for suggestion in &report.soft_suggestions {
            let more = if suggestion.definition_count > 1 {
                format!(" (+{} more definitions)", suggestion.definition_count - 1)
            } else {
                String::new()
            };
            println!(
                "- [Soft] `{}` defined in `{}`{} — `{}`",
                suggestion.name,
                suggestion.defined_in,
                more,
                root_aware_expand(&suggestion.expand)
            );
        }
    }
    unknown_section(&report.unknowns);
    section("Expand", &report.expand);
}

fn render_where_single(report: &WhereReport) {
    let def = &report.definitions[0];
    println!("\n## Definition\n");
    print_where_definition_facts(def, "- ");
    if let Some(detail) = &report.detail {
        render_cone_xray(detail);
        render_where_links(detail);
        hidden_section(&detail.hidden);
        unknown_section(&detail.unknowns);
        section("Expand", &detail.expand);
    } else {
        section("Expand", &def.expand);
    }
}

fn render_where_links(report: &ConeReport) {
    if cone_links_empty(report) {
        render_cone_links(report);
        return;
    }
    if report.outgoing.is_empty() && report.contracts.is_empty() && report.boundary.is_empty() {
        return;
    }
    println!("\n## Links\n");
    grouped_edge_list("outgoing", &report.outgoing, 8);
    grouped_edge_list("contracts", &report.contracts, 8);
    grouped_edge_list("boundary", &report.boundary, 8);
}

fn render_where_multi(report: &WhereReport) {
    println!("\n## Definitions\n");
    for def in &report.definitions {
        println!("### `{}`", def.anchor.path);
        print_where_definition_facts(def, "- ");
        render_where_consumer_preview(def);
        for command in &def.expand {
            println!("- expand: `{}`", root_aware_expand(command));
        }
        println!();
    }
    hidden_section(&report.hidden);
    unknown_section(&report.unknowns);
    section("Expand", &report.expand);
}

fn print_where_definition_facts(def: &WhereDefinition, prefix: &str) {
    let anchor = &def.anchor;
    println!("{prefix}kind: `{}`", anchor.kind);
    println!("{prefix}exported: `{}`", !anchor.exports.is_empty());
    println!(
        "{prefix}package: `{}`",
        anchor.package.as_deref().unwrap_or("none")
    );
    println!("{prefix}lines: `{}`", anchor.lines);
    println!("{prefix}consumers: `{}`", def.consumers_total.display());
}

fn render_where_consumer_preview(def: &WhereDefinition) {
    const PREVIEW: usize = 5;
    for edge in def.consumers.iter().take(PREVIEW) {
        println!(
            "  - [{}] `{}` --{}--> `{}` [{}] {}",
            xray_edge_label(edge),
            edge.from,
            edge.edge_type,
            edge.to,
            public_evidence_label(&edge.evidence),
            edge_location_summary(edge)
        );
    }
    let shown = def.consumers.len().min(PREVIEW);
    let more = def.consumers_total.value.unwrap_or(0).saturating_sub(shown);
    if more > 0 {
        println!(
            "  - {more} more consumers — `codemap cone {} --all`",
            def.anchor.path
        );
    }
}
