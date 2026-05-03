fn ci_owner_validation_step_reason(command: &str) -> Option<String> {
    let command = strip_inline_shell_comment(command);
    let command = command.trim();
    if command.is_empty()
        || ci_owner_command_has_unsupported_shell_control(command)
        || ci_owner_command_has_unsupported_shell_composition(command)
        || ci_owner_command_is_non_validation(command)
    {
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

#[derive(Debug, Clone, Copy)]
enum CiOwnerStepKind {
    Validation,
    Release,
    Setup,
    Control,
}

impl CiOwnerStepKind {
    fn edge_type(self) -> &'static str {
        match self {
            Self::Validation => "ci_validation_step",
            Self::Release => "ci_release_step",
            Self::Setup => "ci_setup_step",
            Self::Control => "ci_control_step",
        }
    }

    fn evidence(self) -> &'static str {
        match self {
            Self::Validation => "ci_run_validation",
            Self::Release => "ci_run_release",
            Self::Setup => "ci_run_setup",
            Self::Control => "ci_run_control",
        }
    }
}

fn ci_owner_step_kind(command: &str) -> Option<CiOwnerStepKind> {
    let command = strip_inline_shell_comment(command);
    let command = command.trim();
    if command.is_empty() {
        return None;
    }
    if ci_owner_command_is_shell_syntax_only(command) {
        return None;
    }
    if ci_owner_validation_step_reason(command).is_some() {
        return Some(CiOwnerStepKind::Validation);
    }
    if ci_owner_command_is_control(command) {
        return Some(CiOwnerStepKind::Control);
    }
    if ci_owner_command_is_release_or_mutation(command) {
        return Some(CiOwnerStepKind::Release);
    }
    if ci_owner_command_is_setup(command) {
        return Some(CiOwnerStepKind::Setup);
    }
    Some(CiOwnerStepKind::Control)
}

fn ci_owner_command_is_shell_syntax_only(command: &str) -> bool {
    let command = command.trim();
    matches!(
        command,
        ")" | ";;" | ";&" | ";;&" | "then" | "do" | "else" | "fi" | "done" | "esac"
    ) || command.starts_with("--")
        || (command.ends_with(')') && !command.chars().any(char::is_whitespace))
}

fn ci_owner_command_has_unsupported_shell_control(command: &str) -> bool {
    let mut chars = command.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;
    let mut previous_was_escape = false;
    while let Some(ch) = chars.next() {
        if previous_was_escape {
            previous_was_escape = false;
            continue;
        }
        if ch == '\\' && !in_single {
            previous_was_escape = true;
            continue;
        }
        if ch == '\'' && !in_double {
            in_single = !in_single;
            continue;
        }
        if ch == '"' && !in_single {
            in_double = !in_double;
            continue;
        }
        if in_single || in_double {
            continue;
        }
        match ch {
            '\n' | '\r' | ';' | '|' | '`' | '$' | '>' | '<' => return true,
            '&' if chars.peek() == Some(&'&') => {
                chars.next();
            }
            '&' => return true,
            _ => {}
        }
    }
    false
}

fn ci_owner_command_has_unsupported_shell_composition(command: &str) -> bool {
    let and_count = command.match_indices("&&").count();
    if and_count == 0 {
        return false;
    }
    and_count != 1 || !ci_owner_safe_scoped_cd_composition(command)
}

fn ci_owner_safe_scoped_cd_composition(command: &str) -> bool {
    let Some((prefix, tail)) = command.split_once("&&") else {
        return false;
    };
    !tail.trim().is_empty() && ci_owner_safe_cd_prefix(prefix)
}

fn ci_owner_safe_cd_prefix(prefix: &str) -> bool {
    let prefix = prefix.trim();
    let Some(rest) = prefix.strip_prefix("cd ") else {
        return false;
    };
    let rest = rest.trim();
    let Some(path) = ci_owner_cd_path(rest) else {
        return false;
    };
    !path.is_empty()
        && path != "~"
        && !path.starts_with('/')
        && !path.starts_with("~/")
        && !path.starts_with('-')
        && !path.split('/').any(|part| part == "..")
}

fn ci_owner_cd_path(rest: &str) -> Option<String> {
    if let Some(quote) = rest.chars().next().filter(|ch| *ch == '\'' || *ch == '"') {
        return Some(rest.strip_prefix(quote)?.strip_suffix(quote)?.to_string());
    }
    if rest.chars().any(char::is_whitespace) {
        return None;
    }
    Some(rest.to_string())
}

fn ci_owner_command_is_non_validation(command: &str) -> bool {
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

fn ci_owner_command_is_control(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    let first = lower.split_whitespace().next().unwrap_or_default();
    matches!(
        first,
        "if" | "fi" | "then" | "else" | "elif" | "for" | "while" | "do" | "done" | "case"
            | "esac" | "set" | "echo" | "exit" | "test"
    )
}

fn ci_owner_command_is_release_or_mutation(command: &str) -> bool {
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

fn ci_owner_command_is_setup(command: &str) -> bool {
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

fn ci_owner_cargo_validation(tokens: &[String]) -> bool {
    let Some(subcommand) = ci_owner_cargo_subcommand(tokens) else {
        return false;
    };
    if subcommand == "fmt" { return tokens.iter().any(|token| token == "--check"); }
    if ci_owner_cargo_subcommand_is_validation(subcommand) { return true; }
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
        || ci_owner_command_is_readonly_migration_status(command)
        || ci_owner_direct_script_validation(command)
}

fn ci_owner_command_is_readonly_migration_status(command: &str) -> bool {
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
                extension.to_str().unwrap_or_default().to_ascii_lowercase().as_str(),
                "sh"
                    | "bash"
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
    let Some(extension) = Path::new(token).extension().and_then(|extension| extension.to_str())
    else {
        return true;
    };
    matches!(
        extension.to_ascii_lowercase().as_str(),
        "sh"
            | "bash"
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
