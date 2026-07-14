// Responsibility: map-proof-owner-ci-validation-matchers
use crate::map::{
    ci_owner_command_is_readonly_migration_status, command_tokens, token_invokes_package_script,
};
use std::path::Path;

pub(crate) fn ci_owner_cargo_validation(tokens: &[String]) -> bool {
    let Some(subcommand) = ci_owner_cargo_subcommand(tokens) else {
        return false;
    };
    if subcommand == "fmt" {
        return tokens.iter().any(|token| token == "--check");
    }
    if ci_owner_cargo_subcommand_is_validation(subcommand) {
        return true;
    }
    subcommand == "run" && tokens.iter().any(|token| token == "doctor")
}

fn ci_owner_cargo_subcommand(tokens: &[String]) -> Option<&str> {
    let cargo_index = tokens
        .iter()
        .position(|token| token == "cargo" || token.ends_with("/cargo"))?;
    let mut index = cargo_index + 1;
    while index < tokens.len() {
        let token = tokens[index].as_str();
        if token.starts_with('+') || cargo_global_flag(token) {
            index += 1;
            continue;
        }
        return Some(token);
    }
    None
}

fn cargo_global_flag(token: &str) -> bool {
    matches!(
        token,
        "--locked" | "--offline" | "--frozen" | "--quiet" | "-q" | "--verbose" | "-v"
    )
}

fn ci_owner_cargo_subcommand_is_validation(token: &str) -> bool {
    matches!(
        token,
        "test" | "check" | "build" | "clippy" | "fmt" | "nextest"
    )
}

pub(crate) fn ci_owner_package_script_validation(tokens: &[String]) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        ci_owner_script_name_is_validation(token) && token_invokes_package_script(tokens, index)
    })
}

pub(crate) fn ci_owner_script_name_is_validation(script: &str) -> bool {
    let lower = script
        .trim_matches(|ch| matches!(ch, '"' | '\''))
        .to_ascii_lowercase();
    if crate::proof_classification::proof_text_is_readonly_migration_status(&lower) {
        return true;
    }
    if lower.is_empty()
        || lower.contains("deploy")
        || lower.contains("release")
        || lower.contains("publish")
        || lower.contains("migrate")
        || lower.contains("codegen")
        || lower.contains("generate")
        || lower.contains("setup")
        || lower.contains("install")
        || lower.contains("db:push")
        || lower.contains("db:normalize")
        || lower.contains("watch")
        || ci_owner_lifecycle_script_name(&lower)
        || lower.contains("reset")
        || lower.contains("destroy")
        || lower.contains("delete")
        || lower.contains("drop")
        || lower.contains("prune")
    {
        return false;
    }
    lower == "test"
        || lower.starts_with("test:")
        || lower.starts_with("test-")
        || lower.contains(":test")
        || lower == "lint"
        || lower.starts_with("lint:")
        || lower.starts_with("lint-")
        || lower == "build"
        || lower.starts_with("build:")
        || lower == "check"
        || lower.starts_with("check:")
        || lower.starts_with("check-")
        || lower.starts_with("typecheck")
        || lower.starts_with("type-check")
        || lower.starts_with("verify")
        || lower.starts_with("validate")
        || lower == "doctor"
        || lower.starts_with("doctor:")
        || lower.starts_with("doctor-")
        || lower.starts_with("proof")
        || lower.starts_with("e2e")
        || lower.starts_with("smoke")
        || lower.starts_with("integration")
        || lower.starts_with("contract")
}

fn ci_owner_lifecycle_script_name(lower: &str) -> bool {
    lower == "dev"
        || lower.starts_with("dev:")
        || lower.starts_with("dev-")
        || lower == "start"
        || lower.starts_with("start:")
        || lower.starts_with("start-")
        || lower == "serve"
        || lower.starts_with("serve:")
        || lower.starts_with("serve-")
        || lower == "preview"
        || lower.starts_with("preview:")
        || lower.starts_with("preview-")
}

pub(crate) fn ci_owner_direct_tool_validation(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    lower == "go test"
        || lower.starts_with("go test ")
        || lower == "swift test"
        || lower.starts_with("swift test ")
        || lower == "pytest"
        || lower.starts_with("pytest ")
        || lower == "python -m pytest"
        || lower.starts_with("python -m pytest ")
        || lower.contains("playwright test")
        || lower.contains("vitest")
        || lower.contains(" jest")
        || lower.contains(" mocha")
        || lower.contains(" node --test")
        || ci_owner_command_is_readonly_migration_status(command)
        || ci_owner_direct_script_validation(command)
}

fn ci_owner_direct_script_validation(command: &str) -> bool {
    command_tokens(command).iter().any(|token| {
        let token = token.trim_matches(|ch| matches!(ch, '"' | '\''));
        if !ci_owner_token_looks_like_script_path(token) {
            return false;
        }
        if !ci_owner_script_path_has_executable_shape(token) {
            return false;
        }
        let name = Path::new(token)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(token);
        let stem = name
            .strip_suffix(".sh")
            .or_else(|| name.strip_suffix(".bash"))
            .or_else(|| name.strip_suffix(".zsh"))
            .or_else(|| name.strip_suffix(".py"))
            .or_else(|| name.strip_suffix(".js"))
            .or_else(|| name.strip_suffix(".mjs"))
            .or_else(|| name.strip_suffix(".cjs"))
            .or_else(|| name.strip_suffix(".ts"))
            .or_else(|| name.strip_suffix(".mts"))
            .or_else(|| name.strip_suffix(".cts"))
            .or_else(|| name.strip_suffix(".ps1"))
            .or_else(|| name.strip_suffix(".rb"))
            .or_else(|| name.strip_suffix(".pl"))
            .or_else(|| name.strip_suffix(".php"))
            .unwrap_or(name);
        ci_owner_script_name_is_validation(stem)
    })
}

fn ci_owner_token_looks_like_script_path(token: &str) -> bool {
    token.contains('/')
        || Path::new(token).extension().is_some_and(|extension| {
            matches!(
                extension
                    .to_str()
                    .unwrap_or_default()
                    .to_ascii_lowercase()
                    .as_str(),
                "sh" | "bash"
                    | "zsh"
                    | "py"
                    | "js"
                    | "mjs"
                    | "cjs"
                    | "ts"
                    | "mts"
                    | "cts"
                    | "ps1"
                    | "rb"
                    | "pl"
                    | "php"
            )
        })
}

fn ci_owner_script_path_has_executable_shape(token: &str) -> bool {
    let Some(extension) = Path::new(token)
        .extension()
        .and_then(|extension| extension.to_str())
    else {
        return true;
    };
    matches!(
        extension.to_ascii_lowercase().as_str(),
        "sh" | "bash"
            | "zsh"
            | "py"
            | "js"
            | "mjs"
            | "cjs"
            | "ts"
            | "mts"
            | "cts"
            | "ps1"
            | "rb"
            | "pl"
            | "php"
    )
}

pub(crate) fn ci_owner_make_or_just_validation(tokens: &[String]) -> bool {
    matches!(
        tokens,
        [runner, target, ..]
            if matches!(runner.as_str(), "make" | "just")
                && ci_owner_script_name_is_validation(target)
    )
}

pub(crate) fn command_has_codemap_validation(tokens: &[String]) -> bool {
    tokens.windows(2).any(|window| {
        window[0] == "codemap"
            && matches!(
                window[1].as_str(),
                "changed" | "proof-map" | "doctor" | "boundaries"
            )
    })
}
