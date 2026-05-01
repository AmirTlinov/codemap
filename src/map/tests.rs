#[cfg(test)]
mod tests {
    use super::*;

    fn proof(
        command: &str,
        path: &str,
        evidence: &str,
        strength: EvidenceStrength,
    ) -> ProofSurface {
        ProofSurface {
            command: Some(command.to_string()),
            path: Some(path.to_string()),
            evidence: evidence.to_string(),
            strength,
            reason: format!("{evidence} reason"),
            locations: vec![EvidenceLocation::path(path, evidence)],
        }
    }

    #[test]
    fn symbol_value_scanner_keyword_probe_handles_unicode_prefix() {
        assert!(!previous_word_is("навигации", "return"));
        assert!(previous_word_is("навигации return", "return"));
    }

    #[test]
    fn proof_surfaces_dedupe_by_command_path_and_keep_strongest_evidence() {
        let proofs = unique_proof_surfaces(vec![
            proof(
                "pnpm exec vitest run tests/a.test.ts",
                "tests/a.test.ts",
                "test_surface_tokens",
                EvidenceStrength::Medium,
            ),
            proof(
                "pnpm exec vitest run tests/a.test.ts",
                "tests/a.test.ts",
                "test_name",
                EvidenceStrength::High,
            ),
            proof(
                "pnpm exec vitest run tests/a.test.ts",
                "tests/a.test.ts",
                "test_import",
                EvidenceStrength::High,
            ),
            proof(
                "pnpm exec vitest run tests/b.test.ts",
                "tests/b.test.ts",
                "test_surface_tokens",
                EvidenceStrength::Medium,
            ),
        ]);

        assert_eq!(proofs.len(), 2);
        assert_eq!(proofs[0].path.as_deref(), Some("tests/a.test.ts"));
        assert_eq!(proofs[0].evidence, "test_import");
        assert_eq!(proofs[0].strength, EvidenceStrength::High);
        assert_eq!(proofs[1].path.as_deref(), Some("tests/b.test.ts"));
    }

    #[test]
    fn proof_map_grouping_keeps_strongest_sensor_per_proof_file() {
        let mut proofs = vec![
            proof(
                "pnpm exec vitest run tests/a.test.ts",
                "tests/a.test.ts",
                "test_surface_tokens",
                EvidenceStrength::Medium,
            ),
            proof(
                "pnpm exec vitest run tests/a.test.ts",
                "tests/a.test.ts",
                "test_import",
                EvidenceStrength::High,
            ),
        ];
        let mut hidden = Vec::new();

        group_duplicate_proof_surfaces(
            &mut proofs,
            &mut hidden,
            "duplicate direct proof sensors grouped by structural key",
            "codemap proof-map . --limit <larger-number>",
        );

        assert_eq!(proofs.len(), 1);
        assert_eq!(proofs[0].evidence, "test_import");
        assert_eq!(proofs[0].strength, EvidenceStrength::High);
        assert_eq!(hidden.len(), 1);
        assert_eq!(hidden[0].count, 1);
    }

    #[test]
    fn proof_map_grouping_preserves_distinct_route_sensors_from_same_file() {
        let mut login = proof(
            "pnpm run test:e2e -- tests/e2e/auth.spec.ts",
            "tests/e2e/auth.spec.ts",
            "e2e_visited_route",
            EvidenceStrength::High,
        );
        login.reason = "e2e visits runtime route GET /auth/login".to_string();
        let mut logout = proof(
            "pnpm run test:e2e -- tests/e2e/auth.spec.ts",
            "tests/e2e/auth.spec.ts",
            "e2e_visited_route",
            EvidenceStrength::High,
        );
        logout.reason = "e2e visits runtime route GET /auth/logout".to_string();
        let mut proofs = vec![login, logout];
        let mut hidden = Vec::new();

        group_duplicate_proof_surfaces(
            &mut proofs,
            &mut hidden,
            "duplicate e2e proof sensors grouped by structural key",
            "codemap proof-map . --limit <larger-number>",
        );

        assert_eq!(proofs.len(), 2);
        assert!(hidden.is_empty());
    }
}
