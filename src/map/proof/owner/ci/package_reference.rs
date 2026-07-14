// Responsibility: map-proof-owner-ci-package-reference
use crate::map::command_tokens;
use crate::repo;

pub(crate) fn command_references_package(
    package: &crate::model::PackageInfo,
    command: &str,
) -> bool {
    let name = package.name.to_ascii_lowercase();
    let path = repo::normalize_rel_path(&package.path).to_ascii_lowercase();
    let tokens = command_tokens(command);
    let name_tail = name.rsplit('/').next().unwrap_or(&name);
    tokens.iter().any(|token| {
        package_reference_token_matches(token, &name, name_tail, &path, false)
            || package_selector_token_matches(token, &name, name_tail, &path)
    }) || tokens.windows(2).any(|window| {
        package_selector_flag(&window[0])
            && package_reference_token_matches(&window[1], &name, name_tail, &path, true)
    })
}

fn package_selector_flag(token: &str) -> bool {
    matches!(
        token,
        "--filter" | "-f" | "--workspace" | "-w" | "--package" | "-p" | "workspace"
    )
}

fn package_selector_token_matches(token: &str, name: &str, name_tail: &str, path: &str) -> bool {
    let Some((flag, value)) = token.split_once('=') else {
        return false;
    };
    package_selector_flag(flag)
        && package_reference_token_matches(value, name, name_tail, path, true)
}

fn package_reference_token_matches(
    token: &str,
    name: &str,
    name_tail: &str,
    path: &str,
    selector_value: bool,
) -> bool {
    let normalized = token
        .trim_matches(|ch| matches!(ch, '"' | '\''))
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }
    if !name.is_empty() && normalized == name {
        return true;
    }
    if selector_value && !name_tail.is_empty() && normalized == name_tail {
        return true;
    }
    package_path_token_matches(&normalized, path)
}

fn package_path_token_matches(token: &str, path: &str) -> bool {
    path != "."
        && !path.is_empty()
        && (token == path
            || token.starts_with(&format!("{path}/"))
            || token.ends_with(&format!("/{path}"))
            || token.contains(&format!("/{path}/")))
}

pub(crate) fn command_invokes_script(command: &str, script: &str) -> bool {
    let script = script.to_ascii_lowercase();
    let tokens = command_tokens(command);
    tokens
        .iter()
        .enumerate()
        .any(|(index, token)| token == &script && token_invokes_package_script(&tokens, index))
}

pub(crate) fn token_invokes_package_script(tokens: &[String], script_index: usize) -> bool {
    let Some(package_index) = tokens[..script_index]
        .iter()
        .rposition(|token| package_script_runner(token))
    else {
        return false;
    };
    let between = &tokens[package_index + 1..script_index];
    if between.is_empty() {
        return true;
    }
    if between.iter().any(|token| {
        matches!(
            token.as_str(),
            "exec" | "dlx" | "x" | "create" | "install" | "add" | "remove" | "publish"
        )
    }) {
        return false;
    }
    if between.last().is_some_and(|token| token == "run") {
        return package_script_selector_prefix(&between[..between.len() - 1]);
    }
    package_script_selector_prefix(between)
}

fn package_script_runner(token: &str) -> bool {
    matches!(token, "npm" | "pnpm" | "yarn" | "bun" | "corepack")
}

fn package_script_selector_prefix(tokens: &[String]) -> bool {
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index].as_str();
        if package_selector_with_value(token) {
            if index + 1 >= tokens.len() {
                return false;
            }
            index += 2;
        } else if package_selector_inline(token) || package_selector_without_value(token) {
            index += 1;
        } else {
            return false;
        }
    }
    true
}

fn package_selector_with_value(token: &str) -> bool {
    matches!(
        token,
        "--filter" | "-f" | "--workspace" | "--package" | "-p"
    )
}

fn package_selector_inline(token: &str) -> bool {
    token.starts_with("--filter=")
        || token.starts_with("--workspace=")
        || token.starts_with("--package=")
}

fn package_selector_without_value(token: &str) -> bool {
    matches!(token, "--workspace-root" | "-w" | "--recursive" | "-r")
}
