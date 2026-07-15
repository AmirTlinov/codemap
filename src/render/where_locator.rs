// Responsibility: render-where-locator
use crate::model::{ConeReport, WhereDefinition, WhereReport};
use crate::render::{
    AnchorPathDisplay, cone_links_empty, disclaimer, edge_location_summary_with_paths,
    grouped_edge_list_with_paths, hidden_section, public_evidence_label, readable_certificate_id,
    render_cone_links, render_cone_xray, render_definition_visibility,
    render_definition_visibility_compact, render_visibility_section, root_aware_expand, section,
    unknown_section, xray_edge_label,
};

pub fn where_locator(report: &WhereReport) {
    if report.total_matches > 1 {
        render_where_multi(report);
        return;
    }
    println!("# Structural Where\n");
    println!("Query: `{}`", report.query);
    println!(
        "Kind filter: `{}`",
        report.kind_filter.as_deref().unwrap_or("none")
    );
    println!("Matches: `{}`", report.total_matches);
    if report.total_matches == 1
        && let Some(definition) = report.definitions.first()
    {
        let paths = AnchorPathDisplay::new(&definition.anchor.path);
        if paths.compact() {
            println!(
                "Anchor: `{}`{}",
                definition.anchor.path,
                paths.header_suffix()
            );
        }
    }
    render_visibility_section(&report.observations);

    match report.total_matches {
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
    let Some(def) = report.definitions.first() else {
        unknown_section(&report.unknowns);
        section("Expand", &report.expand);
        return;
    };
    let paths = AnchorPathDisplay::new(&def.anchor.path);
    println!("\n## Definition\n");
    print_where_definition_facts(def, "- ");
    if paths.compact() {
        let detail_has_expand = report
            .detail
            .as_ref()
            .is_some_and(|detail| detail.hidden.iter().any(|group| !group.expand.is_empty()));
        render_definition_visibility_compact(&def.observations, !detail_has_expand);
    } else {
        render_definition_visibility(&def.observations);
    }
    render_where_consumer_preview(def);
    if let Some(detail) = &report.detail {
        render_cone_xray(detail);
        render_where_links(detail);
        hidden_section(&detail.hidden);
        if !paths.compact() || !has_exact_compact_expand(def, detail) {
            section("Expand", &detail.expand);
        }
    } else {
        section("Expand", &def.expand);
    }
}

fn has_exact_compact_expand(definition: &WhereDefinition, detail: &ConeReport) -> bool {
    detail.hidden.iter().any(|group| !group.expand.is_empty())
        || definition
            .observations
            .horizons
            .iter()
            .any(|horizon| horizon.hidden > 0 && horizon.expand.is_some())
}

fn render_where_links(report: &ConeReport) {
    let paths = AnchorPathDisplay::new(&report.anchor.path);
    if paths.compact() {
        return;
    }
    if cone_links_empty(report) {
        render_cone_links(report, false);
        return;
    }
    if report.outgoing.is_empty() && report.contracts.is_empty() && report.boundary.is_empty() {
        return;
    }
    println!("\n## Links\n");
    grouped_edge_list_with_paths("outgoing", &report.outgoing, 8, &paths);
    grouped_edge_list_with_paths("contracts", &report.contracts, 8, &paths);
    grouped_edge_list_with_paths("boundary", &report.boundary, 8, &paths);
}

fn render_where_multi(report: &WhereReport) {
    println!("# Structural Where\n");
    println!("Query: `{}`", report.query);
    println!(
        "Kind filter: `{}`",
        report.kind_filter.as_deref().unwrap_or("none")
    );
    println!("Matches: `{}`", report.total_matches);
    render_visibility_section(&report.observations);
    println!("\n## Definitions\n");
    for def in &report.definitions {
        render_compact_definition(def);
    }
    hidden_section(&report.hidden);
    unknown_section(&report.unknowns);
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
}

fn render_where_consumer_preview(def: &WhereDefinition) {
    if def.consumers.is_empty() {
        return;
    }
    let paths = AnchorPathDisplay::new(&def.anchor.path);
    println!("Consumers:");
    for edge in &def.consumers {
        println!(
            "  - [{}] `{}` --{}--> `{}` [{}] {}",
            xray_edge_label(edge),
            paths.path(&edge.from),
            edge.edge_type,
            paths.path(&edge.to),
            public_evidence_label(&edge.evidence),
            edge_location_summary_with_paths(edge, &paths)
        );
    }
}

fn render_compact_definition(def: &WhereDefinition) {
    println!("- `{}`", def.anchor.path);
    let mut expands = std::collections::BTreeSet::new();
    for group in ["consumers", "incoming", "verification"] {
        let Some(horizon) = def
            .observations
            .horizons
            .iter()
            .find(|horizon| horizon.group == group)
        else {
            println!("  - {group}: unavailable");
            continue;
        };
        println!(
            "  - {group}: {}; shown={} hidden={}; cert=`{}`",
            horizon.count.display(),
            horizon.shown,
            horizon.hidden,
            readable_certificate_id(&horizon.count.certificate_id)
        );
        if horizon.hidden > 0
            && let Some(expand) = horizon.expand.as_deref()
        {
            expands.insert(root_aware_expand(expand));
        }
    }
    for expand in expands {
        println!("  expand: `{expand}`");
    }
}
