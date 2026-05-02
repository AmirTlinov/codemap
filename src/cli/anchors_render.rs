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
    fn run_plan_rejects_deploy_commands_before_running_any_command() {
        let plan = VerificationPlan {
            minimal: vec!["npm run deploy".to_string()],
            supplemental: vec!["cargo test".to_string()],
            full_only_if_triggered: Vec::new(),
        };

        let error = planned_run_commands(&plan, true).expect_err("deploy should fail closed");

        assert!(
            error
                .to_string()
                .contains("will not run by default")
        );
    }

    #[test]
    fn run_plan_rejects_unknown_shell_commands_before_running_any_command() {
        let plan = VerificationPlan {
            minimal: vec!["sh -c 'touch proof-ran'".to_string()],
            supplemental: vec!["cargo test".to_string()],
            full_only_if_triggered: Vec::new(),
        };

        let error = planned_run_commands(&plan, true).expect_err("unknown shell should fail closed");

        assert!(
            error
                .to_string()
                .contains("will not run by default")
        );
    }

    #[test]
    fn run_plan_rejects_shell_control_after_safe_prefix() {
        let plan = VerificationPlan {
            minimal: vec![
                "cargo test ; rm -rf target/proof-owned".to_string(),
                "pnpm test $(touch proof-owned)".to_string(),
                "pnpm run test:e2e -- tests/app.spec.ts ; rm proof-owned".to_string(),
            ],
            supplemental: Vec::new(),
            full_only_if_triggered: Vec::new(),
        };

        let error = planned_run_commands(&plan, false).expect_err("shell control should fail closed");

        assert!(
            error
                .to_string()
                .contains("will not run by default")
        );
    }

    #[test]
    fn run_plan_rejects_unsafe_test_like_script_names() {
        let plan = VerificationPlan {
            minimal: vec!["pnpm run test:deploy".to_string()],
            supplemental: Vec::new(),
            full_only_if_triggered: Vec::new(),
        };

        let error = planned_run_commands(&plan, false).expect_err("deploy script should fail closed");

        assert!(
            error
                .to_string()
                .contains("will not run by default")
        );
    }

    #[test]
    fn run_plan_rejects_cd_scope_escape_before_running_any_command() {
        for command in [
            "cd / && cargo test",
            "cd .. && cargo test",
            "cd ../pkg && cargo test",
            "cd ~ && cargo test",
            "cd packages/app extra && cargo test",
        ] {
            let plan = VerificationPlan {
                minimal: vec![command.to_string()],
                supplemental: Vec::new(),
                full_only_if_triggered: Vec::new(),
            };

            let error = planned_run_commands(&plan, false)
                .expect_err("cd scope escape should fail closed");

            assert!(
                error
                    .to_string()
                    .contains("will not run by default"),
                "{command} should be rejected before execution"
            );
        }
    }

    #[test]
    fn run_plan_allows_scoped_safe_test_commands() {
        let plan = VerificationPlan {
            minimal: vec![
                "cd packages/app && pnpm run test:e2e -- tests/app.spec.ts".to_string(),
                "cd 'packages/app with spaces' && pnpm test tests/app.test.ts".to_string(),
                "cargo test --release".to_string(),
                "pnpm exec jest tests/app.test.ts".to_string(),
                "pnpm exec node --test tests/app.test.ts".to_string(),
                "yarn mocha tests/app.test.ts".to_string(),
                "make test".to_string(),
            ],
            supplemental: Vec::new(),
            full_only_if_triggered: Vec::new(),
        };

        let commands = planned_run_commands(&plan, false).expect("safe proof commands");

        assert_eq!(
            commands,
            vec![
                "cd packages/app && pnpm run test:e2e -- tests/app.spec.ts",
                "cd 'packages/app with spaces' && pnpm test tests/app.test.ts",
                "cargo test --release",
                "pnpm exec jest tests/app.test.ts",
                "pnpm exec node --test tests/app.test.ts",
                "yarn mocha tests/app.test.ts",
                "make test"
            ]
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
