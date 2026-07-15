// Responsibility: readable-project-ecosystem-support-matrix
use crate::map::StatusReport;
use crate::model::EcosystemSupportCells;
use crate::render::{code, table};

pub(crate) fn ecosystem_support(report: &StatusReport) {
    if report.ecosystem_support.is_empty() {
        return;
    }
    println!(
        "\n## Ecosystem Support Matrix (v{})\n",
        report.ecosystem_support_version
    );
    let rows = report
        .ecosystem_support
        .iter()
        .map(|support| {
            vec![
                support.declaration.ecosystem.clone(),
                support.declaration.tier.clone(),
                support.detected_files.to_string(),
                support.generated_files.to_string(),
                supported_cells(&support.declaration.cells),
                unsupported_cells(&support.declaration.cells),
                support
                    .examples
                    .iter()
                    .map(|path| code(path))
                    .collect::<Vec<_>>()
                    .join(", "),
            ]
        })
        .collect();
    println!(
        "{}",
        table(
            &[
                "Ecosystem",
                "Tier",
                "Files",
                "Generated",
                "Observed cells",
                "Open cells",
                "Examples"
            ],
            rows,
        )
    );
    println!(
        "\nTier is the release promise ceiling; a specialized observed cell does not promote the ecosystem. Unsupported cells remain explicit."
    );
}

fn supported_cells(cells: &EcosystemSupportCells) -> String {
    cell_pairs(cells)
        .into_iter()
        .filter(|(_, state)| matches!(*state, "verified" | "structural" | "inventory"))
        .map(|(cell, state)| format!("{cell}={state}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn unsupported_cells(cells: &EcosystemSupportCells) -> String {
    cell_pairs(cells)
        .into_iter()
        .filter(|(_, state)| matches!(*state, "unsupported" | "not_applicable"))
        .map(|(cell, state)| format!("{cell}={state}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn cell_pairs(cells: &EcosystemSupportCells) -> [(&'static str, &str); 9] {
    [
        ("inventory", &cells.inventory),
        ("symbols", &cells.symbols),
        ("imports", &cells.imports),
        ("packages", &cells.packages),
        ("runtime", &cells.runtime),
        ("contracts", &cells.contracts),
        ("data", &cells.data),
        ("verification", &cells.verification),
        ("dynamic_unknowns", &cells.dynamic_unknowns),
    ]
}
