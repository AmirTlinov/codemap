// Responsibility: render-cone-xray
use crate::model::{ConeReport, StructuralEdge};
use crate::render::{
    AnchorPathDisplay, code, edge_location_summary_with_paths, public_evidence_label,
};

pub(crate) fn render_cone_xray(report: &ConeReport) {
    let paths = AnchorPathDisplay::new(&report.anchor.path);
    let xray = &report.xray;
    if xray.roles.is_empty()
        && xray.inputs.is_empty()
        && xray.outputs.is_empty()
        && xray.state.is_empty()
        && xray.side_effects.is_empty()
        && xray.direct_consumers.is_empty()
        && xray.mediated_consumers.is_empty()
        && xray.flow.is_empty()
        && xray.nearby.is_empty()
        && xray.proof_hard.is_empty()
        && xray.proof_direct.is_empty()
        && xray.proof_mediated.is_empty()
        && xray.proof_soft.is_empty()
        && xray.unknowns.is_empty()
    {
        return;
    }
    println!("\n## X-Ray Card\n");
    render_xray_surfaces("Role", &xray.roles, &paths);
    render_xray_edges("Inputs", &xray.inputs, &paths);
    render_xray_output_surfaces(&xray.outputs, &paths);
    render_xray_surfaces("State", &xray.state, &paths);
    render_xray_surfaces("Side Effects", &xray.side_effects, &paths);
    render_observed_xray_edges("Direct Consumers", &xray.direct_consumers, &paths);
    render_observed_xray_edges("Mediated Consumers", &xray.mediated_consumers, &paths);
    render_xray_flow(&xray.flow, &paths);
    render_xray_surfaces("Existing Nearby Surfaces", &xray.nearby, &paths);
    render_xray_proof(xray, &paths);
    render_xray_unknowns(&xray.unknowns, &paths);
}

pub(crate) fn render_cone_xray_proof(report: &ConeReport) {
    render_xray_proof(&report.xray, &AnchorPathDisplay::new(&report.anchor.path));
}

fn render_xray_surfaces(
    title: &str,
    surfaces: &[crate::model::Surface],
    paths: &AnchorPathDisplay<'_>,
) {
    if surfaces.is_empty() {
        return;
    }
    println!("{title}:");
    for surface in surfaces {
        render_xray_surface(surface, paths);
    }
}

fn render_xray_output_surfaces(surfaces: &[crate::model::Surface], paths: &AnchorPathDisplay<'_>) {
    if surfaces.is_empty() {
        return;
    }
    println!("Outputs:");
    for surface in surfaces {
        render_xray_surface(surface, paths);
    }
}

fn render_xray_surface(surface: &crate::model::Surface, paths: &AnchorPathDisplay<'_>) {
    let label = surface
        .path
        .as_ref()
        .map(|path| code(&paths.path(path)))
        .unwrap_or_else(|| code(&surface.kind));
    let distinct_examples = surface
        .examples
        .iter()
        .filter(|example| surface.path.as_deref() != Some(example.as_str()))
        .take(3)
        .collect::<Vec<_>>();
    let examples = if distinct_examples.is_empty() || paths.compact() {
        String::new()
    } else {
        format!(
            " examples={}",
            distinct_examples
                .iter()
                .map(|example| code(&paths.path(example)))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    println!(
        "- [{}] `{}` {} [{}]{}",
        xray_surface_label(surface),
        surface.kind,
        label,
        public_evidence_label(&surface.evidence),
        examples
    );
    if surface.hidden_count > 0 {
        println!("  additional examples: {}", surface.hidden_count);
    }
}

fn render_xray_edges(title: &str, edges: &[StructuralEdge], paths: &AnchorPathDisplay<'_>) {
    if edges.is_empty() {
        return;
    }
    println!("{title}:");
    for edge in edges {
        println!(
            "- [{}] `{}` --{}--> `{}` [{}] {}",
            xray_edge_label(edge),
            paths.path(&edge.from),
            edge.edge_type,
            paths.path(&edge.to),
            public_evidence_label(&edge.evidence),
            edge_location_summary_with_paths(edge, paths)
        );
    }
}

fn render_observed_xray_edges(
    title: &str,
    edges: &[StructuralEdge],
    paths: &AnchorPathDisplay<'_>,
) {
    if edges.is_empty() {
        return;
    }
    println!("{title}:");
    for edge in edges {
        println!(
            "- [{}] `{}` --{}--> `{}` [{}] {}",
            xray_edge_label(edge),
            paths.path(&edge.from),
            edge.edge_type,
            paths.path(&edge.to),
            public_evidence_label(&edge.evidence),
            edge_location_summary_with_paths(edge, paths)
        );
    }
}

fn render_xray_flow(steps: &[crate::model::FlowStep], paths: &AnchorPathDisplay<'_>) {
    if steps.is_empty() {
        return;
    }
    println!("Structural Flow:");
    for step in steps {
        let where_hint = step
            .locations
            .first()
            .map(|location| {
                if let Some(line) = location.line_start {
                    code(&format!("{}:{line}", paths.path(&location.path)))
                } else {
                    code(&paths.path(&location.path))
                }
            })
            .unwrap_or_else(|| "unknown".to_string());
        println!(
            "- [Direct] `{}` [{}; {}] {}",
            paths.path(&step.anchor),
            step.kind,
            public_evidence_label(&step.evidence),
            where_hint
        );
    }
}

fn render_xray_proof(xray: &crate::model::XrayCard, paths: &AnchorPathDisplay<'_>) {
    if xray.proof_hard.is_empty()
        && xray.proof_direct.is_empty()
        && xray.proof_mediated.is_empty()
        && xray.proof_soft.is_empty()
    {
        return;
    }
    println!("Verification Sensors:");
    render_xray_proof_bucket("Runnable", &xray.proof_hard, paths);
    render_xray_proof_bucket("Direct", &xray.proof_direct, paths);
    render_xray_proof_bucket("Mediated", &xray.proof_mediated, paths);
    render_xray_proof_bucket("Soft", &xray.proof_soft, paths);
    if !xray.proof_soft.is_empty() {
        println!(
            "- [Unknown] soft surface matches are name/path/token overlap; they are not runnable command surfaces"
        );
    }
}

fn render_xray_proof_bucket(label: &str, edges: &[StructuralEdge], paths: &AnchorPathDisplay<'_>) {
    for edge in edges {
        println!(
            "- [{label}] `{}` --{}--> `{}` [{}] {}",
            paths.path(&edge.from),
            edge.edge_type,
            paths.path(&edge.to),
            public_evidence_label(&edge.evidence),
            edge_location_summary_with_paths(edge, paths)
        );
    }
}

fn render_xray_unknowns(unknowns: &[crate::model::Unknown], paths: &AnchorPathDisplay<'_>) {
    if unknowns.is_empty() {
        return;
    }
    println!("Unknown:");
    for unknown in unknowns {
        println!(
            "- [Unknown] `{}` at {} - {}",
            unknown.kind,
            compact_unknown_where(unknown, paths),
            unknown.reason
        );
    }
}

fn compact_unknown_where(unknown: &crate::model::Unknown, paths: &AnchorPathDisplay<'_>) -> String {
    unknown
        .path
        .as_ref()
        .map(|path| {
            let path = paths.path(path);
            if let Some(line) = unknown.line_start {
                code(&format!("{path}:{line}"))
            } else {
                code(&path)
            }
        })
        .unwrap_or_else(|| "none".to_string())
}

fn xray_surface_label(surface: &crate::model::Surface) -> &'static str {
    match surface.evidence.as_str() {
        "env_file" | "symbol_definition" => "Hard",
        "exported_symbol" | "static_storage_write" | "static_network_call" | "raw_sql_mutation" => {
            "Direct"
        }
        "same_directory_surface" | "surface_hint" | "file_role_or_extension" | "file_kind" => {
            "Soft"
        }
        _ if surface.strength == crate::model::EvidenceStrength::Hard => "Hard",
        _ if surface.strength >= crate::model::EvidenceStrength::High => "Direct",
        _ => "Soft",
    }
}

pub(crate) fn xray_edge_label(edge: &StructuralEdge) -> &'static str {
    if edge.evidence.ends_with("_via_direct_consumer")
        || edge.evidence.ends_with("_via_direct_dependency")
        || edge.evidence.ends_with("_via_local_symbol_consumer")
        || edge.evidence.ends_with("_via_cone_depth")
        || edge.evidence.contains("reexport")
        || edge.evidence.contains("barrel")
        || edge.evidence.contains("module_aggregator")
    {
        return "Mediated";
    }
    let base = crate::proof_classification::proof_base_evidence(&edge.evidence);
    if matches!(
        base,
        "test_name"
            | "e2e_surface_phrase"
            | "e2e_path_surface"
            | "test_surface_phrase"
            | "test_surface_tokens"
            | "test_role_surface_match"
            | "script_path_token"
            | "script_surface_match"
            | "proof_neighbor_token_match"
    ) {
        return "Soft";
    }
    if edge.strength == crate::model::EvidenceStrength::Hard {
        "Hard"
    } else {
        "Direct"
    }
}
