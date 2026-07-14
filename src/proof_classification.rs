use crate::model::{EvidenceStrength, ProofSurface};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofSurfaceClass {
    Hard,
    DirectEvidence,
    MediatedEvidence,
    SoftEvidence,
    SetupSupport,
}

pub fn proof_surface_class(proof: &ProofSurface) -> ProofSurfaceClass {
    if proof_surface_is_setup_or_support(proof) {
        return ProofSurfaceClass::SetupSupport;
    }
    if proof_surface_is_mediated_evidence(proof) {
        return ProofSurfaceClass::MediatedEvidence;
    }
    if proof_surface_is_soft_evidence(proof) {
        return ProofSurfaceClass::SoftEvidence;
    }
    if proof_surface_is_runnable_validation(proof) {
        return ProofSurfaceClass::Hard;
    }
    ProofSurfaceClass::DirectEvidence
}

pub fn proof_base_evidence(evidence: &str) -> &str {
    evidence
        .strip_suffix("_owning_file")
        .or_else(|| evidence.strip_suffix("_via_direct_consumer"))
        .or_else(|| evidence.strip_suffix("_via_direct_dependency"))
        .or_else(|| evidence.strip_suffix("_via_local_symbol_consumer"))
        .unwrap_or(evidence)
}

pub fn proof_surface_is_runnable_validation(proof: &ProofSurface) -> bool {
    proof.command.is_some()
        && !proof_surface_is_setup_or_support(proof)
        && !proof_evidence_is_soft_match(&proof.evidence)
        && (proof.strength >= EvidenceStrength::High
            || proof_evidence_is_direct_validation(&proof.evidence))
}

pub fn proof_surface_is_evidence_only(proof: &ProofSurface) -> bool {
    proof.command.is_none() && !proof_surface_is_soft_evidence(proof)
}

pub fn proof_surface_is_setup_or_support(proof: &ProofSurface) -> bool {
    if matches!(
        proof_base_evidence(&proof.evidence),
        "ci_run_setup"
            | "ci_setup_step"
            | "ci_run_release"
            | "ci_release_step"
            | "manifest_script_setup"
    ) {
        return true;
    }
    let command = proof.command.as_deref();
    let command_is_setup = command.is_some_and(proof_text_is_setup_or_support);
    if !command_is_setup {
        return false;
    }
    if command.is_some_and(proof_command_has_lifecycle_support_name) {
        return true;
    }
    true
}

pub fn proof_surface_is_soft_evidence(proof: &ProofSurface) -> bool {
    if proof_surface_is_setup_or_support(proof) {
        return false;
    }
    if proof_evidence_is_soft_match(&proof.evidence) {
        return true;
    }
    if proof.strength < EvidenceStrength::High
        && !proof_evidence_is_direct_validation(&proof.evidence)
    {
        return true;
    }
    false
}

pub fn proof_surface_is_mediated_evidence(proof: &ProofSurface) -> bool {
    proof_evidence_is_mediated(&proof.evidence) || proof.evidence.ends_with("_owning_file")
}

pub fn proof_evidence_is_direct_validation(evidence: &str) -> bool {
    if proof_evidence_is_mediated(evidence) {
        return false;
    }
    matches!(
        proof_base_evidence(evidence),
        "test_import"
            | "test_imported_symbol_reference"
            | "test_reexported_symbol_reference"
            | "test_support_import"
            | "test_symbol_reference"
            | "e2e_route"
            | "current_level_proof_container"
    )
}

pub fn proof_text_is_readonly_migration_status(text: &str) -> bool {
    let normalized = normalize_proof_text(text);
    normalized.contains("migrate status")
        || normalized.contains("migrate:status")
        || normalized.contains("db:migrate:status")
}

pub fn proof_command_is_readonly_migration_status(command: &str) -> bool {
    if !proof_text_is_readonly_migration_status(command) {
        return false;
    }
    let tokens = proof_command_tokens(command);
    let Some(prisma_index) = tokens.iter().position(|token| {
        token == "prisma" || token.ends_with("/prisma") || token.ends_with("/.bin/prisma")
    }) else {
        return false;
    };
    tokens
        .get(prisma_index + 1)
        .is_some_and(|token| token == "migrate")
        && tokens
            .get(prisma_index + 2)
            .is_some_and(|token| token == "status")
        && readonly_prisma_prefix_is_safe(&tokens[..prisma_index])
}

pub fn proof_evidence_is_soft_match(evidence: &str) -> bool {
    if proof_evidence_is_mediated(evidence) {
        return true;
    }
    matches!(
        proof_base_evidence(evidence),
        "script_path_token"
            | "script_surface_match"
            | "test_name"
            | "e2e_surface_phrase"
            | "e2e_path_surface"
            | "test_surface_phrase"
            | "test_surface_tokens"
            | "test_role_surface_match"
    )
}

fn proof_evidence_is_mediated(evidence: &str) -> bool {
    evidence.ends_with("_via_direct_consumer")
        || evidence.ends_with("_via_direct_dependency")
        || evidence.ends_with("_via_local_symbol_consumer")
}

pub fn proof_text_is_setup_or_support(text: &str) -> bool {
    let normalized = normalize_proof_text(text);
    let migration_status = proof_text_is_readonly_migration_status(&normalized);
    if setup_or_support_needles()
        .iter()
        .any(|needle| normalized.contains(needle))
    {
        return true;
    }
    if !migration_status
        && (normalized.contains("db:migrate")
            || normalized.contains(" migrate")
            || normalized.contains(":migrate")
            || normalized.contains("migration run"))
    {
        return true;
    }
    false
}

fn normalize_proof_text(text: &str) -> String {
    text.to_ascii_lowercase()
        .replace(['\'', '"', '`'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn proof_command_tokens(command: &str) -> Vec<String> {
    command
        .to_ascii_lowercase()
        .replace(['\'', '"', '`'], " ")
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

fn readonly_prisma_prefix_is_safe(prefix: &[String]) -> bool {
    prefix.is_empty()
        || matches!(prefix, [runner] if matches!(runner.as_str(), "npx" | "bunx" | "yarn"))
        || matches!(
            prefix,
            [runner, subcommand]
                if matches!(runner.as_str(), "pnpm" | "npm" | "yarn" | "bun")
                    && matches!(subcommand.as_str(), "exec" | "dlx")
        )
}

fn setup_or_support_needles() -> &'static [&'static str] {
    &[
        " install",
        ":install",
        " setup",
        " watch",
        ":watch",
        "-watch",
        " run dev",
        " run start",
        " run serve",
        " run preview",
        " codegen",
        ":codegen",
        " generate",
        ":generate",
        " seed",
        ":seed",
        " studio",
        ":studio",
        " db push",
        " db:push",
        ":db:push",
        "db:push",
        "db:normalize",
        "prisma/normalize",
        " deploy",
        ":deploy",
        " release",
        ":release",
        " publish",
        ":publish",
        " destroy",
        ":destroy",
        " delete",
        ":delete",
        " drop",
        ":drop",
        " prune",
        ":prune",
        " reset",
        ":reset",
    ]
}

fn proof_command_has_lifecycle_support_name(text: &str) -> bool {
    let normalized = text
        .to_ascii_lowercase()
        .replace(['\'', '"', '`'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    normalized.contains(":watch")
        || normalized.contains("-watch")
        || normalized.contains(" run dev")
        || normalized.contains(" run start")
        || normalized.contains(" run serve")
        || normalized.contains(" run preview")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EvidenceLocation;

    fn proof(command: Option<&str>, evidence: &str, strength: EvidenceStrength) -> ProofSurface {
        ProofSurface {
            command: command.map(str::to_string),
            path: Some("package.json".to_string()),
            target_anchor: Some("package.json".to_string()),
            evidence: evidence.to_string(),
            strength,
            reason: "fixture".to_string(),
            locations: vec![EvidenceLocation::line("package.json", 1, "script")],
        }
    }

    #[test]
    fn setup_support_surfaces_do_not_also_render_as_soft_evidence() {
        let surface = proof(
            Some("pnpm run dev"),
            "script_surface_match",
            EvidenceStrength::Medium,
        );

        assert!(proof_surface_is_setup_or_support(&surface));
        assert!(!proof_surface_is_soft_evidence(&surface));
        assert!(!proof_surface_is_runnable_validation(&surface));
    }

    #[test]
    fn token_matches_without_setup_command_remain_soft_evidence() {
        let surface = proof(
            Some("pnpm test"),
            "script_surface_match",
            EvidenceStrength::Medium,
        );

        assert!(!proof_surface_is_setup_or_support(&surface));
        assert!(proof_surface_is_soft_evidence(&surface));
        assert!(!proof_surface_is_runnable_validation(&surface));
    }
}
