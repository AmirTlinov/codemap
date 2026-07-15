// Responsibility: ls-report-rendering
use crate::model::LsReport;
use crate::render::{
    boundary_facts_section, bullet, code, disclaimer, grouped_edge_list, hidden_section,
    map_prelude_line_or_snapshot_line, public_evidence_label, render_roles, root_aware_expand,
    section,
};

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
    if matches!(section_filter, None | Some("links")) && report.mode != "missing" {
        if report.edges.is_empty() {
            render_empty_ls_section(
                "Links",
                "No indexed structural links observed in this scope.",
            );
        } else {
            println!("\n## Links\n");
            let limit = if report.hidden.is_empty() {
                usize::MAX
            } else {
                20
            };
            grouped_edge_list("links", &report.edges, limit);
        }
    }
    if matches!(section_filter, Some("proof")) {
        render_empty_ls_section(
            "Verification Surfaces",
            "Verification surfaces are not computed by ls.",
        );
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

pub(crate) fn render_anchor_summary(title: &str, anchor: &crate::model::FileSummary) {
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
    println!("- imported by: `{}`", anchor.imported_by.display());
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
    disclaimer(
        "Derived from deterministic path/name/extension/manifest patterns. Not intent, correctness, or ownership truth.",
    );
    for (role, count) in roles {
        println!("- `{role}`: `{count}` surfaces");
    }
}

fn render_empty_ls_section(title: &str, detail: &str) {
    println!("\n## {title}\n");
    println!("- {detail}");
}
