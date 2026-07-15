// Responsibility: cone-report-rendering
use crate::model::{ConeReport, StructuralEdge};
use crate::render::{
    AnchorPathDisplay, code, cone_section, grouped_edge_list_with_paths, hidden_section,
    map_prelude_line_or_snapshot_line, render_anchor_summary, render_cone_xray,
    render_cone_xray_proof, render_roles, render_visibility_section_for_groups, section,
    unknown_section,
};

pub fn cone(report: &ConeReport, section_filter: Option<&str>) {
    println!("# Structural Cone\n");
    map_prelude_line_or_snapshot_line();
    let paths = AnchorPathDisplay::new(&report.anchor.path);
    println!("Anchor: `{}`{}", report.anchor.path, paths.header_suffix());
    println!("Depth: `{}`", report.depth);
    let is_symbol = report.anchor.kind.starts_with("symbol");
    let visibility_groups: &[&str] = match section_filter {
        None | Some("observed") => &["incoming", "verification"],
        Some("links") => &["incoming"],
        Some("proof") => &["verification"],
        _ => &[],
    };
    if !visibility_groups.is_empty() {
        render_visibility_section_for_groups(&report.observations, visibility_groups);
    }
    if matches!(section_filter, None | Some("observed")) {
        render_cone_xray(report);
        render_cone_observed(report);
    }
    if matches!(section_filter, None | Some("roles")) {
        render_roles(&report.anchor);
    }
    if matches!(section_filter, None | Some("links")) {
        let compact_default_symbol_links = paths.compact() && is_symbol && section_filter.is_none();
        if !compact_default_symbol_links {
            render_cone_links(report, section_filter == Some("links"));
        }
    }
    if matches!(section_filter, None | Some("proof")) {
        if is_symbol {
            if section_filter == Some("proof") {
                render_cone_xray_proof(report);
            }
        } else {
            render_cone_proof(&report.proof);
        }
    }
    if matches!(section_filter, None | Some("hidden")) {
        hidden_section(&report.hidden);
    }
    if matches!(section_filter, None | Some("unknown")) {
        unknown_section(&report.unknowns);
    }
    if section_filter.is_none() && (!paths.compact() || !has_exact_compact_expand(report)) {
        section("Expand", &report.expand);
    }
}

fn has_exact_compact_expand(report: &ConeReport) -> bool {
    report.hidden.iter().any(|group| !group.expand.is_empty())
        || report
            .observations
            .horizons
            .iter()
            .any(|horizon| horizon.hidden > 0 && horizon.expand.is_some())
}

pub(crate) fn render_cone_links(report: &ConeReport, include_symbol_incoming: bool) {
    let paths = AnchorPathDisplay::new(&report.anchor.path);
    let is_symbol = report.anchor.kind.starts_with("symbol");
    let show_incoming = !is_symbol || include_symbol_incoming;
    if cone_links_empty(report) {
        if !matches!(report.anchor.kind.as_str(), "missing" | "missing_symbol") {
            println!("\n## Links\n");
            println!("No indexed structural links observed in this scope.");
        }
        return;
    }
    if report.outgoing.is_empty()
        && (!show_incoming || report.incoming.is_empty())
        && report.contracts.is_empty()
        && report.boundary.is_empty()
    {
        return;
    }
    println!("\n## Links\n");
    let outgoing_limit = if is_symbol { report.outgoing.len() } else { 12 };
    grouped_edge_list_with_paths("outgoing", &report.outgoing, outgoing_limit, &paths);
    if show_incoming {
        let incoming_limit = if is_symbol { report.incoming.len() } else { 12 };
        grouped_edge_list_with_paths("incoming", &report.incoming, incoming_limit, &paths);
    }
    grouped_edge_list_with_paths("contracts", &report.contracts, 12, &paths);
    grouped_edge_list_with_paths("boundary", &report.boundary, 12, &paths);
}

pub(crate) fn cone_links_empty(report: &ConeReport) -> bool {
    report.outgoing.is_empty()
        && report.incoming.is_empty()
        && report.proof.is_empty()
        && report.contracts.is_empty()
        && report.boundary.is_empty()
}

pub(crate) fn render_cone_proof(edges: &[StructuralEdge]) {
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
    cone_section("Verification Surfaces", &proof);
    cone_section("Linked Surfaces", &evidence);
    cone_section("Setup / Support Surfaces", &setup);
    cone_section("Soft Surface Matches", &soft);
    if !setup.is_empty() {
        println!(
            "\nSetup/support surfaces are connected rails such as install, codegen, migration, seed, deploy, release, watch, or dev-server steps. They are not treated as verification command surfaces."
        );
    }
    if !soft.is_empty() {
        println!(
            "\nSoft surface matches are token/name/path overlap. They do not create a direct linked verification surface or remove Unknown entries."
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
                | "test_role_surface_match"
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

pub(crate) fn edge_location_summary_with_paths(
    edge: &StructuralEdge,
    paths: &AnchorPathDisplay<'_>,
) -> String {
    let Some(first) = edge.locations.first() else {
        return "unknown".to_string();
    };
    let suffix = if edge.locations.len() > 1 {
        format!(" +{}", edge.locations.len() - 1)
    } else {
        String::new()
    };
    let display_path = paths.edge_location_path(edge, &first.path);
    let base = if first.path == "aggregate" {
        "aggregate".to_string()
    } else if let Some(line) = first.line_start {
        format!("{display_path}:{line}")
    } else {
        // A path without a line is a weaker fact; say so instead of
        // rendering it in the same shape as a located edge.
        return format!("{} (line unknown){}", code(&display_path), suffix);
    };
    format!("{}{}", code(&base), suffix)
}

pub(crate) fn render_cone_observed(report: &ConeReport) {
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
