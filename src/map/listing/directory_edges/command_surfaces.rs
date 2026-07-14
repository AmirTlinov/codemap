// Responsibility: map-listing-directory-command-surfaces
use crate::map::{
    command_invokes_script, command_tokens, manifest_dir_for_rel, token_invokes_package_script,
};
use crate::model::{PackageInfo, ScriptInfo};
use crate::repo;

pub(crate) fn script_target_for_path(path: &str, name: &str) -> String {
    let scope = manifest_dir_for_rel(path);
    if scope == "." {
        format!("script:{name}")
    } else {
        format!("script:{scope}:{name}")
    }
}

pub(crate) fn script_target_for_package(package: &PackageInfo, name: &str) -> String {
    if package.path == "." {
        format!("script:{name}")
    } else {
        format!("script:{}:{name}", package.path)
    }
}

pub(crate) fn command_target(command: &str) -> String {
    format!("command:{}", command.trim())
}

pub(crate) fn include_package_script_command_edge(scope: &str) -> bool {
    repo::normalize_rel_path(scope) != "."
}

pub(crate) fn command_invokes_script_surface(command: &str, script: &ScriptInfo) -> bool {
    if command_invokes_script(command, &script.name) {
        return true;
    }
    let command_parts = command_tokens(command);
    let script_tokens = command_tokens(&script.command);
    !script_tokens.is_empty()
        && command_parts
            .windows(script_tokens.len())
            .any(|window| window == script_tokens.as_slice())
}

pub(crate) fn validation_command_like(command: &str) -> bool {
    let tokens = command_tokens(command);
    if tokens.is_empty() {
        return false;
    }
    tokens.windows(2).any(|window| {
        matches!(
            (window[0].as_str(), window[1].as_str()),
            ("cargo", "test")
                | ("cargo", "check")
                | ("cargo", "clippy")
                | ("cargo", "fmt")
                | ("go", "test")
                | ("swift", "test")
                | ("swift", "build")
                | ("python", "-m")
        )
    }) || tokens.iter().any(|token| {
        matches!(
            token.as_str(),
            "pytest"
                | "vitest"
                | "jest"
                | "eslint"
                | "biome"
                | "typecheck"
                | "svelte-check"
                | "doctor"
        )
    }) || command_uses_e2e_runner(&tokens)
        || tokens.iter().enumerate().any(|(index, token)| {
            matches!(
                token.as_str(),
                "test" | "lint" | "check" | "build" | "verify" | "e2e"
            ) && token_invokes_package_script(&tokens, index)
        })
}

fn command_uses_e2e_runner(tokens: &[String]) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        matches!(token.as_str(), "playwright" | "cypress")
            && tokens[index + 1..]
                .iter()
                .any(|later| matches!(later.as_str(), "test" | "e2e" | "run"))
    })
}
