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
    proof_surface_section("Added Proof Surfaces", &report.added_proof_surfaces);
    proof_surface_section("Removed Proof Surfaces", &report.removed_proof_surfaces);
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
                vec!["Public surface".to_string(), report.public_surface.to_string()],
            ],
        )
    );
    contract_exports_section("Exported Contracts", &report.exported_contracts);
    cone_section("Package Exports", &report.package_exports);
    cone_section("Producers", &report.producers);
    cone_section("Consumers", &report.consumers);
    cone_section("Cross Package Consumers", &report.cross_package_consumers);
    cone_section("Proof", &report.proof);
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
    surface_section("Scripts", &report.scripts);
    env_section("Env", &report.env);
    surface_section("Workers", &report.workers);
    surface_section("CI", &report.ci);
    cone_section("Proof", &report.proof);
    unknown_section(&report.unknowns);
    hidden_section(&report.hidden);
    section("Expand", &report.expand);
}

pub fn proof_map(report: &ProofMapReport) {
    println!("# Proof Map\n");
    map_snapshot_line();
    if let Some(scope) = &report.scope {
        println!("Scope: `{scope}`");
    }
    if !report.changed.is_empty() {
        println!("Changed: `{}`", report.changed.join("`, `"));
    }
    proof_map_surface_sections(report);
    surface_section("Missing Direct", &report.missing_direct);
    let runnable_commands = report
        .commands
        .iter()
        .filter(|proof| crate::proof_classification::proof_surface_is_runnable_validation(proof))
        .cloned()
        .collect::<Vec<_>>();
    proof_command_summary_section("Runnable Commands", &runnable_commands);
    if !report.fallback.is_empty() {
        println!("\n## Fallback\n");
        println!("{}", code_block("bash", &report.fallback));
    }
    unknown_section(&report.unknowns);
    hidden_section(&report.hidden);
    section("Expand", &report.expand);
}

fn proof_map_surface_sections(report: &ProofMapReport) {
    proof_surface_section("Hard Proof", &report.hard);
    proof_surface_section("Direct Evidence", &report.direct_evidence);
    proof_surface_section("Mediated Evidence", &report.mediated_evidence);
    proof_surface_section("Soft Token Evidence", &report.soft_evidence);
    proof_surface_section("Setup / Support Surfaces", &report.setup_support);
    if !report.mediated_evidence.is_empty() {
        println!(
            "\nMediated evidence is connected through a direct consumer, dependency, symbol consumer, barrel, or runtime bridge. It does not replace direct proof or remove Unknown entries."
        );
    }
    if !report.soft_evidence.is_empty() {
        println!(
            "\nSoft token evidence is token/name/path surface overlap. It does not replace deterministic proof or remove Unknown entries."
        );
    }
    if !report.setup_support.is_empty() {
        println!(
            "\nSetup/support surfaces are connected rails such as install, codegen, migration, seed, deploy, release, watch, or dev-server steps. They are not validation proof."
        );
    }
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
    cone_section("Proof", &report.proof);
    unknown_section(&report.unknown_breaks);
    hidden_section(&report.hidden);
    section("Expand", &report.expand);
}

pub fn siblings(report: &SiblingsReport) {
    println!("# Siblings Lens\n");
    map_snapshot_line();
    println!("Scope: `{}`", report.scope);
    surface_section("Same Kind", &report.same_kind);
    surface_section("Route/Service/Test Triplets", &report.route_service_test_triplets);
    cone_section("Shared Helpers", &report.shared_helpers);
    cone_section("Shared Contracts", &report.shared_contracts);
    lens_proof_sensor_section("Proof Sensors", &report.proof_pattern);
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
    lens_proof_sensor_section("Paired Proof Sensors", &report.paired_proof_pattern);
    cone_section("Shared Contracts", &report.shared_contracts);
    unknown_section(&report.unknowns);
    hidden_section(&report.hidden);
    section("Expand", &report.expand);
}

fn surface_section(title: &str, surfaces: &[Surface]) {
    if surfaces.is_empty() {
        return;
    }
    println!("\n## {title}\n");
    for surface in surfaces {
        let label = surface
            .path
            .as_ref()
            .map(|path| code(path))
            .unwrap_or_else(|| "aggregate".to_string());
        println!(
            "- {label} [{}; {}; {}]",
            surface.kind,
            public_evidence_label(&surface.evidence),
            format!("{:?}", surface.strength).to_ascii_lowercase()
        );
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
}

fn lens_proof_sensor_section(title: &str, proofs: &[ProofSurface]) {
    if proofs.is_empty() {
        return;
    }
    let title = if proofs
        .iter()
        .all(crate::proof_classification::proof_surface_is_soft_evidence)
    {
        "Soft Evidence"
    } else {
        title
    };
    proof_surface_section(title, proofs);
    if title == "Soft Evidence" {
        println!(
            "\nSoft evidence is token/name/path surface overlap. It does not replace deterministic proof or remove Unknown entries."
        );
    }
}

fn runtime_routes_section(title: &str, routes: &[RuntimeRoute]) {
    if routes.is_empty() {
        return;
    }
    println!("\n## {title}\n");
    for route in routes {
        let method = route.method.as_deref().unwrap_or("ANY");
        let handler = route
            .handler_symbol
            .as_ref()
            .map(|symbol| format!(" handler `{symbol}`"))
            .unwrap_or_default();
        println!(
            "- `{method} {}` -> `{}`{} [{}; {}] {}",
            route.path,
            route.file,
            handler,
            public_evidence_label(&route.evidence),
            format!("{:?}", route.strength).to_ascii_lowercase(),
            proof_location_summary(&route.locations)
        );
    }
}

fn env_section(title: &str, env: &[EnvSurface]) {
    if env.is_empty() {
        return;
    }
    println!("\n## {title}\n");
    for item in env {
        println!(
            "- `{}` used by `{}` [{}; {}] {}",
            item.name,
            item.used_by,
            public_evidence_label(&item.evidence),
            format!("{:?}", item.strength).to_ascii_lowercase(),
            proof_location_summary(&item.locations)
        );
        if let Some(declaration) = &item.declaration {
            println!("  declaration: `{declaration}`");
        }
    }
}

fn render_file_summaries(title: &str, files: &[crate::model::FileSummary]) {
    if files.is_empty() {
        return;
    }
    println!("\n## {title}\n");
    for file in files {
        let package = file.package.as_deref().unwrap_or("none");
        println!(
            "- `{}` [{}; {}; package={}; lines={}]",
            file.path, file.kind, file.language, package, file.lines
        );
        if !file.roles.is_empty() {
            println!("  surface hints: {}", file.roles.join(", "));
        }
        if !file.exports.is_empty() {
            println!("  exports: {}", file.exports.join(", "));
        }
    }
}
