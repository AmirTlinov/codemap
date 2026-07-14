// Responsibility: cli-anchors-render
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

#[cfg(test)]
mod tests {
    use crate::model::{
        EvidenceLocation, EvidenceStrength, ProofReport, ProofSurface, VerificationPlan,
    };

    use crate::cli::{planned_run_commands, proof_plan_commands_for_run, resolve_run_command};

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
    fn proof_run_uses_fallback_when_only_soft_evidence_exists() {
        let report = ProofReport {
            kind: "proof_plan",
            schema_version: "9",
            target: Some("src/routes.ts".to_string()),
            changed: Vec::new(),
            selector: "src/routes.ts".to_string(),
            risk: "medium".to_string(),
            proofs: vec![ProofSurface {
                command: Some("pnpm exec vitest run tests/routes.test.ts".to_string()),
                path: Some("tests/routes.test.ts".to_string()),
                target_anchor: Some("src/routes.ts".to_string()),
                evidence: "test_surface_tokens".to_string(),
                strength: EvidenceStrength::Medium,
                reason: "test path/symbols match anchor surface".to_string(),
                locations: vec![EvidenceLocation::line(
                    "tests/routes.test.ts",
                    1,
                    "test_surface",
                )],
            }],
            coverage: None,
            wiring: Vec::new(),
            fallback: vec!["pnpm test".to_string()],
            unknowns: Vec::new(),
            hidden: Vec::new(),
            expand: Vec::new(),
            run_hint: "codemap proof prints a verification surface plan by default; use --run to execute rendered commands"
                .to_string(),
        };

        assert_eq!(
            proof_plan_commands_for_run(&report),
            vec!["pnpm test"],
            "soft token/path evidence must not become the direct --run command"
        );
    }

    #[test]
    fn proof_run_uses_fallback_when_only_setup_surface_exists() {
        let report = ProofReport {
            kind: "proof_plan",
            schema_version: "9",
            target: Some("pnpm-workspace.yaml".to_string()),
            changed: Vec::new(),
            selector: "pnpm-workspace.yaml".to_string(),
            risk: "medium".to_string(),
            proofs: vec![ProofSurface {
                command: Some("pnpm install --frozen-lockfile".to_string()),
                path: Some(".github/workflows/ci.yml".to_string()),
                target_anchor: Some("pnpm-workspace.yaml".to_string()),
                evidence: "workspace_manifest_ci_reference".to_string(),
                strength: EvidenceStrength::Hard,
                reason: "CI run step references workspace setup".to_string(),
                locations: vec![EvidenceLocation::line(
                    ".github/workflows/ci.yml",
                    7,
                    "ci_step",
                )],
            }],
            coverage: None,
            wiring: Vec::new(),
            fallback: vec!["pnpm test".to_string()],
            unknowns: Vec::new(),
            hidden: Vec::new(),
            expand: Vec::new(),
            run_hint: "codemap proof prints a verification surface plan by default; use --run to execute rendered commands"
                .to_string(),
        };

        assert_eq!(
            proof_plan_commands_for_run(&report),
            vec!["pnpm test"],
            "setup/install surfaces must not become the direct --run command"
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

        assert!(error.to_string().contains("will not run by default"));
    }

    #[test]
    fn run_plan_rejects_unknown_shell_commands_before_running_any_command() {
        let plan = VerificationPlan {
            minimal: vec!["sh -c 'touch proof-ran'".to_string()],
            supplemental: vec!["cargo test".to_string()],
            full_only_if_triggered: Vec::new(),
        };

        let error =
            planned_run_commands(&plan, true).expect_err("unknown shell should fail closed");

        assert!(error.to_string().contains("will not run by default"));
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

        let error =
            planned_run_commands(&plan, false).expect_err("shell control should fail closed");

        assert!(error.to_string().contains("will not run by default"));
    }

    #[test]
    fn run_plan_rejects_unsafe_test_like_script_names() {
        let plan = VerificationPlan {
            minimal: vec![
                "pnpm run test:deploy".to_string(),
                "pnpm run validate-destroy".to_string(),
                "npm run verify:drop".to_string(),
                "yarn proof:delete".to_string(),
                "bun run validate-reset".to_string(),
                "make validate-destroy".to_string(),
                "make verify:drop".to_string(),
                "just proof:delete".to_string(),
            ],
            supplemental: Vec::new(),
            full_only_if_triggered: Vec::new(),
        };

        let error =
            planned_run_commands(&plan, false).expect_err("unsafe scripts should fail closed");

        assert!(error.to_string().contains("will not run by default"));
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

            let error =
                planned_run_commands(&plan, false).expect_err("cd scope escape should fail closed");

            assert!(
                error.to_string().contains("will not run by default"),
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
                "cd apps/api && pnpm run 'db:migrate:status'".to_string(),
                "cd apps/control-center && pnpm run 'smoke:e2e'".to_string(),
                "pnpm --filter @venorus/control-center run smoke:e2e:visual".to_string(),
                "pnpm --filter=@venorus/control-center test".to_string(),
                "cargo fmt --check".to_string(),
                "vitest run".to_string(),
                "playwright test tests/app.spec.ts".to_string(),
                "./scripts/e2e_smoke.sh --headed".to_string(),
                "scripts/verify-local.mjs".to_string(),
                "prisma migrate status --schema prisma/schema.prisma".to_string(),
                "pnpm exec prisma migrate status --schema prisma/schema.prisma".to_string(),
                "cargo test --release".to_string(),
                "pnpm exec jest tests/app.test.ts".to_string(),
                "pnpm exec node --test tests/app.test.ts".to_string(),
                "yarn mocha tests/app.test.ts".to_string(),
                "make test".to_string(),
                "make validate-receipts".to_string(),
                "just doctor".to_string(),
            ],
            supplemental: Vec::new(),
            full_only_if_triggered: Vec::new(),
        };

        let commands = planned_run_commands(&plan, false).expect("safe verification commands");

        assert_eq!(
            commands,
            vec![
                "cd packages/app && pnpm run test:e2e -- tests/app.spec.ts",
                "cd 'packages/app with spaces' && pnpm test tests/app.test.ts",
                "cd apps/api && pnpm run 'db:migrate:status'",
                "cd apps/control-center && pnpm run 'smoke:e2e'",
                "pnpm --filter @venorus/control-center run smoke:e2e:visual",
                "pnpm --filter=@venorus/control-center test",
                "cargo fmt --check",
                "vitest run",
                "playwright test tests/app.spec.ts",
                "./scripts/e2e_smoke.sh --headed",
                "scripts/verify-local.mjs",
                "prisma migrate status --schema prisma/schema.prisma",
                "pnpm exec prisma migrate status --schema prisma/schema.prisma",
                "cargo test --release",
                "pnpm exec jest tests/app.test.ts",
                "pnpm exec node --test tests/app.test.ts",
                "yarn mocha tests/app.test.ts",
                "make test",
                "make validate-receipts",
                "just doctor"
            ]
        );
    }

    #[test]
    fn run_plan_rejects_mutating_and_watch_package_scripts() {
        let plan = VerificationPlan {
            minimal: vec![
                "cd apps/api && pnpm run 'db:push'".to_string(),
                "cd apps/api && pnpm run 'db:normalize-rarity'".to_string(),
                "pnpm run test:watch".to_string(),
                "pnpm run e2e:install".to_string(),
                "pnpm --filter @fixture/api db:migrate:deploy".to_string(),
                "pnpm --filter=@fixture/api db:migrate:deploy".to_string(),
                "pnpm --filter @fixture/api e2e:install".to_string(),
                "cargo fmt".to_string(),
                "./scripts/deploy_test.sh".to_string(),
                "../scripts/e2e_smoke.sh".to_string(),
                "prisma migrate deploy --schema prisma/schema.prisma".to_string(),
            ],
            supplemental: Vec::new(),
            full_only_if_triggered: Vec::new(),
        };

        let error =
            planned_run_commands(&plan, false).expect_err("mutating/watch scripts should fail");

        assert!(error.to_string().contains("will not run by default"));
    }

    #[test]
    fn run_plan_rejects_non_validation_make_targets() {
        let plan = VerificationPlan {
            minimal: vec!["make qwen-sparse-compute-admission-v0-00675".to_string()],
            supplemental: Vec::new(),
            full_only_if_triggered: Vec::new(),
        };

        let error = planned_run_commands(&plan, false)
            .expect_err("non-validation make target should fail closed");

        assert!(error.to_string().contains("will not run by default"));
    }

    #[test]
    fn run_plan_resolves_self_command_to_current_executable() {
        let command = resolve_run_command("codemap boundaries --changed")
            .expect("self command should resolve");

        assert!(command.ends_with(" boundaries --changed"));
        assert_ne!(command, "codemap boundaries --changed");
    }
}
