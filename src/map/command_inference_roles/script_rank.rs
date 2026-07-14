// Responsibility: map-command-inference-roles-script-rank
use crate::map::ProofRoleContext;
use std::collections::BTreeSet;

pub(crate) fn role_aware_script_rank(
    script: &crate::model::ScriptInfo,
    context: &ProofRoleContext,
) -> Option<usize> {
    let text = script_search_text(script);
    if script_is_disallowed_role_proof_text(&text) || script_is_mutating_without_validation(&text) {
        return None;
    }
    let token_hits = context
        .tokens
        .iter()
        .filter(|token| script_text_has_token(&text, token))
        .count();
    if token_hits >= 3 {
        return Some(0);
    }
    if token_hits >= 2
        && script_text_has_any(
            &text,
            &[
                "audit",
                "check",
                "doctor",
                "falsifier",
                "hardening",
                "proof",
                "qwen",
                "test",
                "validate",
                "verify",
            ],
        )
    {
        return Some(1);
    }
    if !script_is_validation_surface_text(&text) {
        return None;
    }
    if !context.has_role_surface {
        return None;
    }
    role_specific_script_rank(&context.roles, &text)
}

pub(crate) fn script_search_text(script: &crate::model::ScriptInfo) -> String {
    format!(
        "{} {} {}",
        script.name.to_ascii_lowercase(),
        script.command.to_ascii_lowercase(),
        script.reason.to_ascii_lowercase()
    )
}

fn role_specific_script_rank(roles: &BTreeSet<String>, text: &str) -> Option<usize> {
    let mut ranks = Vec::new();
    if roles.contains("receipt") || roles.contains("witness") {
        ranks.extend(role_keyword_ranks(
            text,
            &[
                "validate-receipts",
                "receipt",
                "witness",
                "validate",
                "proof",
                "doctor",
                "next",
                "test",
            ],
            10,
        ));
    }
    if roles.contains("proof_runner") {
        ranks.extend(role_keyword_ranks(
            text,
            &["doctor", "validate", "next", "test"],
            20,
        ));
    }
    if roles.contains("owner_doc") {
        ranks.extend(role_keyword_ranks(
            text,
            &["doctor", "next", "validate", "test"],
            30,
        ));
    }
    if roles.contains("migration") || roles.contains("schema") || roles.contains("schema_contract")
    {
        ranks.extend(role_keyword_ranks(
            text,
            &[
                "validate",
                "schema",
                "migration",
                "migrate",
                "db",
                "doctor",
                "test",
            ],
            40,
        ));
    }
    if roles.contains("deploy") {
        ranks.extend(role_keyword_ranks(
            text,
            &["check", "lint", "validate", "doctor", "test"],
            50,
        ));
    }
    if roles.contains("entrypoint") || roles.contains("runtime_surface") {
        ranks.extend(role_keyword_ranks(
            text,
            &["test", "check", "doctor", "validate", "build"],
            60,
        ));
    }
    if roles.contains("doctor") {
        ranks.extend(role_keyword_ranks(text, &["doctor", "check", "test"], 70));
    }
    ranks.into_iter().min()
}

fn role_keyword_ranks(text: &str, keywords: &[&str], offset: usize) -> Vec<usize> {
    keywords
        .iter()
        .enumerate()
        .filter_map(|(index, keyword)| {
            script_text_has_keyword(text, keyword).then_some(offset + index)
        })
        .collect()
}

fn script_is_validation_surface_text(text: &str) -> bool {
    if script_is_disallowed_role_proof_text(text) || script_is_mutating_without_validation(text) {
        return false;
    }
    script_text_has_any(
        text,
        &[
            "test",
            "check",
            "lint",
            "type",
            "doctor",
            "verify",
            "validate",
            "proof",
            "receipt",
            "witness",
            "qwen",
            "next",
            "schema",
            "migration",
            "migrate",
            "db",
            "build",
        ],
    )
}

fn script_is_disallowed_role_proof_text(text: &str) -> bool {
    script_text_has_any(
        text,
        &[
            "deploy",
            "release",
            "publish",
            "migrate",
            "codegen",
            "generate",
            "setup",
            "install",
            "db:push",
            "db:normalize",
            "watch",
            "reset",
            "destroy",
            "delete",
            "drop",
            "prune",
        ],
    )
}

fn script_is_mutating_without_validation(text: &str) -> bool {
    script_text_has_any(
        text,
        &[
            "deploy",
            "release",
            "publish",
            "apply",
            "destroy",
            "delete",
            "drop",
            "migrate deploy",
        ],
    ) && !script_text_has_any(
        text,
        &["validate", "verify", "check", "lint", "test", "doctor"],
    )
}

pub(crate) fn script_text_has_any(text: &str, needles: &[&str]) -> bool {
    needles
        .iter()
        .any(|needle| script_text_has_keyword(text, needle))
}

fn script_text_has_keyword(text: &str, keyword: &str) -> bool {
    if keyword
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return script_text_has_token(text, keyword);
    }
    text.contains(keyword)
}

pub(crate) fn script_text_has_token(text: &str, token: &str) -> bool {
    text.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .any(|part| part == token)
}
