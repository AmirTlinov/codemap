pub fn diff_map(report: &DiffMapReport) {
    println!("# Diff Map\n");
    render_file_summaries("Changed", &report.changed);
    cone_section("Added Structural Lines", &report.added_edges);
    cone_section("Removed Structural Lines", &report.removed_edges);
    if !report.changed_symbols.is_empty() {
        println!("\n## Changed Symbols\n");
        let rows = report
            .changed_symbols
            .iter()
            .map(|symbol| {
                vec![
                    code(&symbol.path),
                    symbol.name.clone(),
                    symbol.change.clone(),
                    symbol
                        .line_start
                        .map(|line| line.to_string())
                        .unwrap_or_else(|| "unknown".to_string()),
                ]
            })
            .collect();
        println!("{}", table(&["Path", "Name", "Change", "Line"], rows));
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
    println!("Anchor: `{}`", report.anchor.path);
    println!(
        "\n{}",
        table(
            &["Field", "Value"],
            vec![
                vec!["Contract kind".to_string(), report.contract_kind.clone()],
                vec!["Public surface".to_string(), report.public_surface.to_string()],
            ],
        )
    );
    contract_exports_section("Exported Contracts", &report.exported_contracts);
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
    if let Some(scope) = &report.scope {
        println!("Scope: `{scope}`");
    }
    if !report.changed.is_empty() {
        println!("Changed: `{}`", report.changed.join("`, `"));
    }
    proof_surface_section("Direct", &report.direct);
    proof_surface_section("Indirect", &report.indirect);
    proof_surface_section("E2E", &report.e2e);
    proof_surface_section("Contract", &report.contract);
    surface_section("Missing Direct", &report.missing_direct);
    proof_command_summary_section("Commands", &report.commands);
    if !report.fallback.is_empty() {
        println!("\n## Fallback\n");
        println!("{}", code_block("bash", &report.fallback));
    }
    unknown_section(&report.unknowns);
    hidden_section(&report.hidden);
    section("Expand", &report.expand);
}

pub fn delete(report: &DeleteReport) {
    println!("# Delete Lens\n");
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
    println!("Scope: `{}`", report.scope);
    cone_section("Actual Cross Edges", &report.actual_cross_edges);
    render_file_summaries("Public Boundary Files", &report.public_boundary_files);
    cone_section("Test Only Crossings", &report.test_only_crossings);
    if !report.package_edges.is_empty() {
        println!("\n## Package Edges\n");
        let rows = report
            .package_edges
            .iter()
            .map(|edge| {
                vec![
                    code(&edge.from_manifest),
                    edge.dependency.clone(),
                    edge.to_manifest
                        .as_ref()
                        .map(|value| code(value))
                        .unwrap_or_else(|| code(&edge.to)),
                    edge.source.clone(),
                ]
            })
            .collect();
        println!("{}", table(&["From", "Dependency", "To", "Evidence"], rows));
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
    println!("Anchor: `{}`", report.anchor);
    println!("Precision: `{}`", report.precision);
    if !report.steps.is_empty() {
        println!("\n## Steps\n");
        let rows = report
            .steps
            .iter()
            .map(|step| {
                vec![
                    step.index.to_string(),
                    code(&step.anchor),
                    step.kind.clone(),
                    step.evidence.clone(),
                    step
                        .locations
                        .first()
                        .map(|location| {
                            if let Some(line) = location.line_start {
                                code(&format!("{}:{line}", location.path))
                            } else {
                                code(&location.path)
                            }
                        })
                        .unwrap_or_else(|| "unknown".to_string()),
                ]
            })
            .collect();
        println!("{}", table(&["#", "Anchor", "Kind", "Evidence", "Where"], rows));
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
    println!("Scope: `{}`", report.scope);
    surface_section("Same Kind", &report.same_kind);
    surface_section("Route/Service/Test Triplets", &report.route_service_test_triplets);
    cone_section("Shared Helpers", &report.shared_helpers);
    cone_section("Shared Contracts", &report.shared_contracts);
    proof_surface_section("Proof Pattern", &report.proof_pattern);
    unknown_section(&report.unknowns);
    hidden_section(&report.hidden);
    section("Expand", &report.expand);
}

pub fn place(report: &PlaceReport) {
    println!("# Place Lens\n");
    println!("Scope: `{}`", report.scope);
    println!("Kind: `{}`", report.requested_kind);
    surface_section("Existing Surfaces", &report.existing_surfaces);
    section("Local Conventions", &report.local_conventions);
    proof_surface_section("Paired Proof Pattern", &report.paired_proof_pattern);
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
    let rows = surfaces
        .iter()
        .map(|surface| {
            vec![
                surface.kind.clone(),
                surface
                    .role
                    .clone()
                    .unwrap_or_else(|| "none".to_string()),
                surface
                    .path
                    .as_ref()
                    .map(|path| code(path))
                    .unwrap_or_else(|| "aggregate".to_string()),
                surface.evidence.clone(),
                format!("{:?}", surface.strength).to_ascii_lowercase(),
                surface
                    .examples
                    .iter()
                    .map(|example| code(example))
                    .collect::<Vec<_>>()
                    .join(", "),
            ]
        })
        .collect();
    println!(
        "{}",
        table(&["Kind", "Role", "Path", "Evidence", "Strength", "Examples"], rows)
    );
}

fn runtime_routes_section(title: &str, routes: &[RuntimeRoute]) {
    if routes.is_empty() {
        return;
    }
    println!("\n## {title}\n");
    let rows = routes
        .iter()
        .map(|route| {
            vec![
                route.method.clone().unwrap_or_else(|| "ANY".to_string()),
                code(&route.path),
                code(&route.file),
                route.evidence.clone(),
                format!("{:?}", route.strength).to_ascii_lowercase(),
            ]
        })
        .collect();
    println!("{}", table(&["Method", "Path", "File", "Evidence", "Strength"], rows));
}

fn env_section(title: &str, env: &[EnvSurface]) {
    if env.is_empty() {
        return;
    }
    println!("\n## {title}\n");
    let rows = env
        .iter()
        .map(|item| {
            vec![
                item.name.clone(),
                code(&item.used_by),
                item.declaration
                    .as_ref()
                    .map(|path| code(path))
                    .unwrap_or_else(|| "none".to_string()),
                item.evidence.clone(),
                format!("{:?}", item.strength).to_ascii_lowercase(),
                proof_location_summary(&item.locations),
            ]
        })
        .collect();
    println!(
        "{}",
        table(
            &["Name", "Used By", "Declaration", "Evidence", "Strength", "Where"],
            rows,
        )
    );
}

fn render_file_summaries(title: &str, files: &[crate::model::FileSummary]) {
    if files.is_empty() {
        return;
    }
    println!("\n## {title}\n");
    let rows = files
        .iter()
        .map(|file| {
            vec![
                code(&file.path),
                file.kind.clone(),
                file.package.clone().unwrap_or_else(|| "none".to_string()),
                file.language.clone(),
            ]
        })
        .collect();
    println!("{}", table(&["Path", "Kind", "Package", "Language"], rows));
}
