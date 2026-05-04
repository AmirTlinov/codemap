fn render_cone_xray(report: &ConeReport) {
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
    render_xray_surfaces("Role", &xray.roles);
    render_xray_edges("Inputs", &xray.inputs);
    render_xray_surfaces("Outputs", &xray.outputs);
    render_xray_surfaces("State", &xray.state);
    render_xray_surfaces("Side Effects", &xray.side_effects);
    render_xray_edges("Direct Consumers", &xray.direct_consumers);
    render_xray_edges("Mediated Consumers", &xray.mediated_consumers);
    render_xray_flow(&xray.flow);
    render_xray_surfaces("Existing Nearby Surfaces", &xray.nearby);
    render_xray_proof(xray);
    render_xray_unknowns(&xray.unknowns);
}

fn render_xray_surfaces(title: &str, surfaces: &[crate::model::Surface]) {
    if surfaces.is_empty() {
        return;
    }
    println!("{title}:");
    const XRAY_SURFACE_LIMIT: usize = 5;
    for surface in surfaces.iter().take(XRAY_SURFACE_LIMIT) {
        let label = surface
            .path
            .as_ref()
            .map(|path| code(path))
            .unwrap_or_else(|| code(&surface.kind));
        let examples = if surface.examples.is_empty() {
            String::new()
        } else {
            format!(
                " examples={}",
                surface
                    .examples
                    .iter()
                    .take(3)
                    .map(|example| code(example))
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
    let hidden = surfaces.len().saturating_sub(XRAY_SURFACE_LIMIT);
    if hidden > 0 {
        println!("- [Unknown] {hidden} more {title} entries hidden by compact x-ray limit");
    }
}

fn render_xray_edges(title: &str, edges: &[StructuralEdge]) {
    if edges.is_empty() {
        return;
    }
    println!("{title}:");
    const XRAY_EDGE_LIMIT: usize = 5;
    for edge in edges.iter().take(XRAY_EDGE_LIMIT) {
        println!(
            "- [{}] `{}` --{}--> `{}` [{}] {}",
            xray_edge_label(edge),
            edge.from,
            edge.edge_type,
            edge.to,
            public_evidence_label(&edge.evidence),
            edge_location_summary(edge)
        );
    }
    let hidden = edges.len().saturating_sub(XRAY_EDGE_LIMIT);
    if hidden > 0 {
        println!("- [Unknown] {hidden} more {title} edges hidden by compact x-ray limit");
    }
}

fn render_xray_flow(steps: &[crate::model::FlowStep]) {
    if steps.is_empty() {
        return;
    }
    println!("Structural Flow:");
    const XRAY_FLOW_LIMIT: usize = 5;
    for step in steps.iter().take(XRAY_FLOW_LIMIT) {
        let where_hint = step
            .locations
            .first()
            .map(|location| {
                if let Some(line) = location.line_start {
                    code(&format!("{}:{line}", location.path))
                } else {
                    code(&location.path)
                }
            })
            .unwrap_or_else(|| "unknown".to_string());
        println!(
            "- [Direct] `{}` [{}; {}] {}",
            step.anchor,
            step.kind,
            public_evidence_label(&step.evidence),
            where_hint
        );
    }
    let hidden = steps.len().saturating_sub(XRAY_FLOW_LIMIT);
    if hidden > 0 {
        println!("- [Unknown] {hidden} more structural flow steps hidden by compact x-ray limit");
    }
}

fn render_xray_proof(xray: &crate::model::XrayCard) {
    if xray.proof_hard.is_empty()
        && xray.proof_direct.is_empty()
        && xray.proof_mediated.is_empty()
        && xray.proof_soft.is_empty()
    {
        return;
    }
    println!("Proof Sensors:");
    render_xray_proof_bucket("Hard", &xray.proof_hard);
    render_xray_proof_bucket("Direct", &xray.proof_direct);
    render_xray_proof_bucket("Mediated", &xray.proof_mediated);
    render_xray_proof_bucket("Soft", &xray.proof_soft);
    if !xray.proof_soft.is_empty() {
        println!(
            "- [Unknown] soft proof evidence is name/path/token overlap; it is not runnable proof"
        );
    }
}

fn render_xray_proof_bucket(label: &str, edges: &[StructuralEdge]) {
    const XRAY_PROOF_LIMIT: usize = 3;
    for edge in edges.iter().take(XRAY_PROOF_LIMIT) {
        println!(
            "- [{label}] `{}` --{}--> `{}` [{}] {}",
            edge.from,
            edge.edge_type,
            edge.to,
            public_evidence_label(&edge.evidence),
            edge_location_summary(edge)
        );
    }
    let hidden = edges.len().saturating_sub(XRAY_PROOF_LIMIT);
    if hidden > 0 {
        println!("- [Unknown] {hidden} more {label} proof sensors hidden by compact x-ray limit");
    }
}

fn render_xray_unknowns(unknowns: &[crate::model::Unknown]) {
    if unknowns.is_empty() {
        return;
    }
    println!("Unknown:");
    for unknown in unknowns.iter().take(5) {
        println!(
            "- [Unknown] `{}` at {} - {}",
            unknown.kind,
            unknown_where(unknown),
            unknown.reason
        );
    }
    let hidden = unknowns.len().saturating_sub(5);
    if hidden > 0 {
        println!("- [Unknown] {hidden} more Unknown entries hidden by compact x-ray limit");
    }
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

fn xray_edge_label(edge: &StructuralEdge) -> &'static str {
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
