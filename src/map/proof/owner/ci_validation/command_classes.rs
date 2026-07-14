// Responsibility: map-proof-owner-ci-validation-command-classes
use crate::map::{command_tokens, token_invokes_package_script};

pub(crate) fn ci_owner_command_is_non_validation(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    let first = lower.split_whitespace().next().unwrap_or_default();
    if ci_owner_command_is_control(command)
        || matches!(
            first,
            "export" | "source" | "." | "mkdir" | "chmod" | "printf" | "git" | "rustup"
        )
    {
        return true;
    }
    if ci_owner_command_is_release_or_mutation(command) {
        return true;
    }
    false
}

pub(crate) fn ci_owner_command_is_control(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    let first = lower.split_whitespace().next().unwrap_or_default();
    matches!(
        first,
        "if" | "fi"
            | "then"
            | "else"
            | "elif"
            | "for"
            | "while"
            | "do"
            | "done"
            | "case"
            | "esac"
            | "set"
            | "echo"
            | "exit"
            | "test"
    )
}

pub(crate) fn ci_owner_command_is_release_or_mutation(command: &str) -> bool {
    if ci_owner_command_is_readonly_migration_status(command) {
        return false;
    }
    let lower = command
        .to_ascii_lowercase()
        .replace(['\'', '"', '`'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    lower.contains(" deploy")
        || lower.contains(":deploy")
        || lower.contains(" release")
        || lower.contains(":release")
        || lower.contains(" publish")
        || lower.contains(":publish")
        || lower.contains(" migrate")
        || lower.contains(":migrate")
        || lower.contains(" db:push")
        || lower.contains(":db:push")
        || lower.contains(" watch")
        || lower.contains(":watch")
        || lower.contains(" run dev")
        || lower.contains(" run start")
        || lower.contains(" run serve")
        || lower.contains(" run preview")
        || lower.contains(" destroy")
        || lower.contains(":destroy")
        || lower.contains(" delete")
        || lower.contains(":delete")
        || lower.contains(" drop")
        || lower.contains(":drop")
        || lower.contains(" prune")
        || lower.contains(":prune")
        || lower.contains(" reset")
        || lower.contains(":reset")
}

pub(crate) fn ci_owner_command_is_setup(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    let first = lower.split_whitespace().next().unwrap_or_default();
    matches!(
        first,
        "rustup" | "chmod" | "mkdir" | "printf" | "git" | "export" | "source" | "."
    ) || first.contains('=')
        || lower.contains(" install")
        || lower.contains(":install")
        || lower.contains(" setup")
        || lower.contains(":setup")
        || lower.contains(" codegen")
        || lower.contains(":codegen")
        || lower.contains(" generate")
        || lower.contains(":generate")
}

pub(crate) fn ci_owner_command_is_readonly_migration_status(command: &str) -> bool {
    ci_owner_readonly_migration_status(command)
        || ci_owner_invokes_readonly_migration_status_script(command)
}

fn ci_owner_invokes_readonly_migration_status_script(command: &str) -> bool {
    let tokens = command_tokens(command);
    tokens.iter().enumerate().any(|(index, token)| {
        crate::proof_classification::proof_text_is_readonly_migration_status(token)
            && token_invokes_package_script(&tokens, index)
    })
}

fn ci_owner_readonly_migration_status(command: &str) -> bool {
    crate::proof_classification::proof_command_is_readonly_migration_status(command)
}
