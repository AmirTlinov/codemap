// Responsibility: lens-shared-sections
use crate::model::{EnvSurface, ProofSurface, RuntimeRoute, StructuralEdge, Surface};
use crate::render::{
    code, grouped_edge_list, proof_location_summary, proof_surface_section, public_evidence_label,
    public_surface_kind_label,
};

pub(crate) fn surface_section(title: &str, surfaces: &[Surface]) {
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
            public_surface_kind_label(&surface.kind),
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

pub(crate) fn lens_proof_sensor_section(title: &str, proofs: &[ProofSurface]) {
    if proofs.is_empty() {
        return;
    }
    let title = if proofs
        .iter()
        .all(crate::proof_classification::proof_surface_is_soft_evidence)
    {
        "Soft Surface Matches"
    } else {
        title
    };
    proof_surface_section(title, proofs);
    if title == "Soft Surface Matches" {
        println!(
            "\nSoft surface matches are token/name/path overlap. They do not create a direct linked verification surface or remove Unknown entries."
        );
    }
}

pub(crate) fn runtime_routes_section(title: &str, routes: &[RuntimeRoute]) {
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
        if !route.middleware_or_guards.is_empty() {
            let boundaries = route
                .middleware_or_guards
                .iter()
                .map(|item| {
                    format!(
                        "{}:{}",
                        format!("{:?}", item.kind).to_ascii_lowercase(),
                        code(&item.owner)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            println!("  middleware/guards: {boundaries}");
        }
    }
}

pub(crate) fn runtime_paths_section(title: &str, paths: &[StructuralEdge]) {
    if paths.is_empty() {
        return;
    }
    println!("\n## {title}\n");
    grouped_edge_list(&title.to_ascii_lowercase(), paths, 5);
}

pub(crate) fn env_section(title: &str, env: &[EnvSurface]) {
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

pub(crate) fn render_file_summaries(title: &str, files: &[crate::model::FileSummary]) {
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
