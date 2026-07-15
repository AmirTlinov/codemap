// Responsibility: lens-report-entries
use crate::model::{
    BoundaryMapReport, ContractReport, DeleteReport, DiffMapReport, FlowReport, PlaceReport,
    RuntimeReport, SiblingsReport,
};
use crate::render::{
    boundaries, code, cone_section, contract_exports_section, env_section, hidden_section,
    lens_proof_sensor_section, map_snapshot_line, plain_section, proof_surface_section,
    public_evidence_label, render_file_summaries, render_runtime_visibility, runtime_paths_section,
    runtime_routes_section, section, surface_section, table, unknown_section,
};

pub fn diff_map(report: &DiffMapReport) {
    println!("# Diff Map\n");
    map_snapshot_line();
    render_file_summaries("Changed", &report.changed);
    cone_section("Added Structural Lines", &report.added_edges);
    cone_section("Removed Structural Lines", &report.removed_edges);
    if !report.changed_symbols.is_empty() {
        println!("\n## Changed Symbols\n");
        for symbol in &report.changed_symbols {
            let where_hint = symbol
                .line_start
                .map(|line| code(&format!("{}:{line}", symbol.path)))
                .unwrap_or_else(|| code(&symbol.path));
            println!("- `{}` in {} [{}]", symbol.name, where_hint, symbol.change);
        }
    }
    surface_section("Added Exports", &report.added_exports);
    surface_section("Removed Exports", &report.removed_exports);
    runtime_routes_section("Added Runtime Routes", &report.added_runtime_routes);
    runtime_routes_section("Removed Runtime Routes", &report.removed_runtime_routes);
    env_section("Added Env", &report.added_env);
    env_section("Removed Env", &report.removed_env);
    proof_surface_section("Added Verification Surfaces", &report.added_proof_surfaces);
    proof_surface_section(
        "Removed Verification Surfaces",
        &report.removed_proof_surfaces,
    );
    unknown_section(&report.new_unknowns);
    hidden_section(&report.hidden);
    section("Expand", &report.expand);
}

pub fn contract(report: &ContractReport) {
    println!("# Contract Lens\n");
    map_snapshot_line();
    println!("Anchor: `{}`", report.anchor.path);
    println!(
        "\n{}",
        table(
            &["Field", "Value"],
            vec![
                vec![
                    "Contract kind".to_string(),
                    public_evidence_label(&report.contract_kind),
                ],
                vec![
                    "Public surface".to_string(),
                    report.public_surface.to_string()
                ],
            ],
        )
    );
    surface_section("Declarations", &report.declarations);
    cone_section("Lineage", &report.lineage);
    contract_exports_section("Exported Contracts", &report.exported_contracts);
    cone_section("Package Exports", &report.package_exports);
    cone_section("Producers", &report.producers);
    cone_section("Consumers", &report.consumers);
    cone_section("Cross Package Consumers", &report.cross_package_consumers);
    cone_section("Verification Surfaces", &report.proof);
    unknown_section(&report.unknowns);
    hidden_section(&report.hidden);
    section("Expand", &report.expand);
}

pub fn runtime(report: &RuntimeReport) {
    println!("# Runtime Lens\n");
    map_snapshot_line();
    println!("Scope: `{}`", report.scope);
    surface_section("Entrypoints", &report.entrypoints);
    runtime_routes_section("Routes", &report.routes);
    runtime_paths_section("Runtime Paths", &report.paths);
    surface_section("Scripts", &report.scripts);
    env_section("Env", &report.env);
    surface_section("Workers", &report.workers);
    surface_section("CI", &report.ci);
    cone_section("Verification Surfaces", &report.proof);
    unknown_section(&report.unknowns);
    render_runtime_visibility(&report.observations);
    hidden_section(&report.hidden);
    section("Expand", &report.expand);
}

pub fn delete(report: &DeleteReport) {
    println!("# Delete Lens\n");
    map_snapshot_line();
    println!("Anchor: `{}`", report.anchor.path);
    cone_section("Direct Users", &report.direct_users);
    cone_section("Symbol Users", &report.symbol_users);
    cone_section("Reexports", &report.reexports);
    cone_section("Package Exports", &report.package_exports);
    cone_section("Tests", &report.tests);
    cone_section("Runtime Refs", &report.runtime_refs);
    section("Checklist", &report.checklist);
    unknown_section(&report.unknowns);
    hidden_section(&report.hidden);
    section("Expand", &report.expand);
}

pub fn boundary_map(report: &BoundaryMapReport) {
    println!("# Boundary Map\n");
    map_snapshot_line();
    println!("Scope: `{}`", report.scope);
    cone_section("Actual Cross Edges", &report.actual_cross_edges);
    render_file_summaries("Public Boundary Files", &report.public_boundary_files);
    cone_section("Test Only Crossings", &report.test_only_crossings);
    if !report.package_edges.is_empty() {
        println!("\n## Package Edges\n");
        for edge in &report.package_edges {
            let target = edge.to_manifest.as_ref().unwrap_or(&edge.to);
            println!(
                "- `{}` --{}--> `{}` [{}; kind={}]",
                edge.from_manifest, edge.dependency, target, edge.source, edge.dependency_kind
            );
            if let Some(workspace_manifest) = &edge.workspace_manifest {
                println!("  workspace: `{workspace_manifest}`");
            }
        }
    }
    if !report.explicit_forbidden_findings.is_empty() {
        boundaries(&report.explicit_forbidden_findings);
    }
    unknown_section(&report.unknowns);
    hidden_section(&report.hidden);
    section("Expand", &report.expand);
}

pub fn flow(report: &FlowReport) {
    println!("# Flow Lens\n");
    map_snapshot_line();
    println!("Anchor: `{}`", report.anchor);
    println!("Precision: `{}`", report.precision);
    if !report.steps.is_empty() {
        println!("\n## Steps\n");
        for step in &report.steps {
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
                "- {}. `{}` [{}; {}] {}",
                step.index,
                step.anchor,
                step.kind,
                public_evidence_label(&step.evidence),
                where_hint
            );
        }
    }
    surface_section("Side Effects", &report.side_effects);
    cone_section("Contracts", &report.contracts);
    cone_section("Verification Surfaces", &report.proof);
    unknown_section(&report.unknown_breaks);
    hidden_section(&report.hidden);
    section("Expand", &report.expand);
}

pub fn siblings(report: &SiblingsReport) {
    println!("# Siblings Lens\n");
    map_snapshot_line();
    println!("Scope: `{}`", report.scope);
    surface_section("Same Kind", &report.same_kind);
    surface_section(
        "Route/Service/Test Triplets",
        &report.route_service_test_triplets,
    );
    cone_section("Shared Helpers", &report.shared_helpers);
    cone_section("Shared Contracts", &report.shared_contracts);
    lens_proof_sensor_section("Verification Sensors", &report.proof_pattern);
    unknown_section(&report.unknowns);
    hidden_section(&report.hidden);
    section("Expand", &report.expand);
}

pub fn place(report: &PlaceReport) {
    println!("# Place Lens\n");
    map_snapshot_line();
    println!("Scope: `{}`", report.scope);
    println!("Kind: `{}`", report.requested_kind);
    surface_section("Existing Surfaces", &report.existing_surfaces);
    plain_section("Local Conventions", &report.local_conventions);
    lens_proof_sensor_section("Paired Verification Sensors", &report.paired_proof_pattern);
    cone_section("Shared Contracts", &report.shared_contracts);
    unknown_section(&report.unknowns);
    hidden_section(&report.hidden);
    section("Expand", &report.expand);
}
