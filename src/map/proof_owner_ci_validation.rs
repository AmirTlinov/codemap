fn ci_owner_validation_step_reason(command: &str) -> Option<String> {
    let command = strip_inline_shell_comment(command);
    let command = command.trim();
    if command.is_empty() || ci_owner_command_is_non_validation(command) {
        return None;
    }
    let tokens = command_tokens(command);
    if ci_owner_cargo_validation(&tokens) {
        return Some("CI workflow run step invokes Cargo validation".to_string());
    }
    if ci_owner_package_script_validation(&tokens) {
        return Some("CI workflow run step invokes package validation script".to_string());
    }
    if ci_owner_direct_tool_validation(command) {
        return Some("CI workflow run step invokes test or validation tool".to_string());
    }
    if ci_owner_make_or_just_validation(&tokens) {
        return Some("CI workflow run step invokes make/just validation target".to_string());
    }
    if command_has_codemap_validation(&tokens) {
        return Some("CI workflow run step invokes codemap validation".to_string());
    }
    None
}

fn ci_owner_command_is_non_validation(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    let first = lower.split_whitespace().next().unwrap_or_default();
    if matches!(
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
            | "export"
            | "source"
            | "."
            | "mkdir"
            | "chmod"
            | "printf"
            | "git"
            | "rustup"
    ) {
        return true;
    }
    if lower.contains(" deploy")
        || lower.contains(":deploy")
        || lower.contains(" release")
        || lower.contains(":release")
        || lower.contains(" publish")
        || lower.contains(":publish")
        || lower.contains(" migrate")
        || lower.contains(":migrate")
    {
        return true;
    }
    first.contains('=')
}

fn ci_owner_cargo_validation(tokens: &[String]) -> bool {
    let Some(subcommand) = ci_owner_cargo_subcommand(tokens) else {
        return false;
    };
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

fn ci_owner_package_script_validation(tokens: &[String]) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        ci_owner_script_name_is_validation(token) && token_invokes_package_script(tokens, index)
    })
}

fn ci_owner_script_name_is_validation(script: &str) -> bool {
    let lower = script
        .trim_matches(|ch| matches!(ch, '"' | '\''))
        .to_ascii_lowercase();
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
        || lower.contains(":test")
        || lower == "lint"
        || lower.starts_with("lint:")
        || lower == "build"
        || lower.starts_with("build:")
        || lower == "check"
        || lower.starts_with("check:")
        || lower.starts_with("typecheck")
        || lower.starts_with("type-check")
        || lower.contains("verify")
        || lower.contains("validate")
        || lower == "doctor"
        || lower.starts_with("doctor:")
        || lower.contains("proof")
}

fn ci_owner_direct_tool_validation(command: &str) -> bool {
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
}

fn ci_owner_make_or_just_validation(tokens: &[String]) -> bool {
    matches!(
        tokens,
        [runner, target, ..]
            if matches!(runner.as_str(), "make" | "just")
                && ci_owner_script_name_is_validation(target)
    )
}

fn command_has_codemap_validation(tokens: &[String]) -> bool {
    tokens.windows(2).any(|window| {
        window[0] == "codemap"
            && matches!(
                window[1].as_str(),
                "changed" | "proof-map" | "doctor" | "boundaries"
            )
    })
}
