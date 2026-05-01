fn anchors_markdown(report: &AnchorValidation) {
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
                    "Forbidden boundaries".to_string(),
                    report.summary.forbidden_boundaries.to_string()
                ],
                vec![
                    "Verification defaults".to_string(),
                    report.summary.verification_defaults.to_string()
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

#[cfg(test)]
mod tests {
    use crate::model::VerificationPlan;

    use super::{planned_run_commands, resolve_run_command};

    #[test]
    fn run_plan_dedupes_minimal_and_supplemental_commands() {
        let plan = VerificationPlan {
            minimal: vec![
                "cargo test".to_string(),
                " cargo test ".to_string(),
                "cargo clippy".to_string(),
            ],
            supplemental: vec![
                "cargo clippy".to_string(),
                "codemap boundaries --changed".to_string(),
            ],
            full_only_if_triggered: vec!["cargo test --all".to_string()],
        };

        let commands = planned_run_commands(&plan, true).expect("commands should be runnable");

        assert_eq!(
            commands,
            vec!["cargo test", "cargo clippy", "codemap boundaries --changed"]
        );
    }

    #[test]
    fn run_plan_rejects_placeholder_before_running_any_command() {
        let plan = VerificationPlan {
            minimal: vec!["run the nearest domain tests for the changed files".to_string()],
            supplemental: vec!["cargo test".to_string()],
            full_only_if_triggered: Vec::new(),
        };

        let error = planned_run_commands(&plan, true).expect_err("placeholder should fail closed");

        assert!(
            error
                .to_string()
                .contains("verification plan contains non-runnable placeholder commands")
        );
    }

    #[test]
    fn run_plan_resolves_self_command_to_current_executable() {
        let command = resolve_run_command("codemap boundaries --changed")
            .expect("self command should resolve");

        assert!(command.ends_with(" boundaries --changed"));
        assert_ne!(command, "codemap boundaries --changed");
    }
}
