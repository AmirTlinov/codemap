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
}
