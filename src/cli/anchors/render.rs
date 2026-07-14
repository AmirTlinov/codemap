// Responsibility: cli-anchors-render
#[cfg(test)]
mod tests;

use crate::cli::AnchorValidation;
use crate::render;

pub(crate) fn anchors_markdown(report: &AnchorValidation) {
    println!("# Anchor Validation\n");
    println!(
        "{}",
        render::table(
            &["Field", "Value"],
            vec![
                vec![
                    "Config".to_string(),
                    report.config.clone().unwrap_or_else(|| "none".to_string())
                ],
                vec!["OK".to_string(), report.ok.to_string()],
                vec!["Domains".to_string(), report.summary.domains.to_string()],
                vec!["Concepts".to_string(), report.summary.concepts.to_string()],
                vec![
                    "Surface hint patterns".to_string(),
                    report.summary.role_patterns.to_string()
                ],
                vec![
                    "Forbidden boundaries".to_string(),
                    report.summary.forbidden_boundaries.to_string()
                ],
                vec![
                    "Verification defaults".to_string(),
                    report.summary.verification_defaults.to_string()
                ],
                vec![
                    "Verification surfaces for changed paths".to_string(),
                    report.summary.proof_changed_commands.to_string()
                ],
            ],
        )
    );
    if report.ok {
        println!("\nNo anchor problems found.");
    } else {
        println!("\n## Problems\n");
        for problem in &report.problems {
            println!("- {problem}");
        }
    }
    if !report.warnings.is_empty() {
        println!("\n## Warnings\n");
        for warning in &report.warnings {
            println!("- {warning}");
        }
    }
    if !report.details.is_empty() {
        println!("\n## Details\n");
        let rows = report
            .details
            .iter()
            .map(|detail| {
                vec![
                    detail.kind.clone(),
                    detail.id.clone(),
                    detail.status.clone(),
                    detail.message.clone(),
                    if detail.next.is_empty() {
                        "none".to_string()
                    } else {
                        detail.next.join("<br>")
                    },
                ]
            })
            .collect();
        println!(
            "{}",
            render::table(&["Kind", "ID", "Status", "Message", "Next"], rows)
        );
    }
}
