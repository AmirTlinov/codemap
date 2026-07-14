// Responsibility: map-proof-owner-ci-match-reasons
use crate::map::{
    command_invokes_script, command_references_package, command_tokens,
    manifest_script_is_proof_relevant,
};
use std::path::Path;

pub(crate) fn manifest_ci_run_match_reason(
    package: &crate::model::PackageInfo,
    scripts: &[(String, String, usize)],
    command: &str,
) -> Option<String> {
    match package.ecosystem.as_str() {
        "javascript" => javascript_manifest_ci_run_match_reason(package, scripts, command),
        "rust" => rust_manifest_ci_run_match_reason(package, command),
        "python" => {
            generic_manifest_ci_run_match_reason(package, command, &["pytest", "python -m pytest"])
        }
        "go" => generic_manifest_ci_run_match_reason(package, command, &["go test"]),
        "swift" => generic_manifest_ci_run_match_reason(package, command, &["swift test"]),
        _ => None,
    }
}

fn javascript_manifest_ci_run_match_reason(
    package: &crate::model::PackageInfo,
    scripts: &[(String, String, usize)],
    command: &str,
) -> Option<String> {
    let script = scripts.iter().find(|(name, script_command, _)| {
        manifest_script_is_proof_relevant(name, script_command)
            && command_invokes_script(command, name)
    });
    let package_ref = command_references_package(package, command);
    if package.path == "." {
        if let Some((name, _, _)) = script {
            return Some(format!("CI run step invokes root package script `{name}`"));
        }
        return None;
    }
    if !package_ref {
        return None;
    }
    if let Some((name, _, _)) = script {
        return Some(format!(
            "CI run step references package `{}` and script `{name}`",
            package.name
        ));
    }
    None
}

fn rust_manifest_ci_run_match_reason(
    package: &crate::model::PackageInfo,
    command: &str,
) -> Option<String> {
    if !rust_cargo_ci_command_is_validation(package, command) {
        return None;
    }
    if package.path == "." {
        return Some("CI run step uses root Cargo manifest".to_string());
    }
    if command_references_package(package, command) {
        return Some(format!(
            "CI run step references Cargo package `{}`",
            package.name
        ));
    }
    None
}

fn rust_cargo_ci_command_is_validation(package: &crate::model::PackageInfo, command: &str) -> bool {
    let tokens = command_tokens(command);
    if !tokens
        .iter()
        .any(|token| token == "cargo" || token.ends_with("/cargo"))
    {
        return false;
    }
    if tokens.iter().any(|token| {
        matches!(
            token.as_str(),
            "test" | "check" | "build" | "clippy" | "fmt" | "nextest"
        )
    }) {
        return true;
    }
    if !tokens.iter().any(|token| token == "run") {
        return false;
    }
    if tokens.iter().any(|token| token == "doctor") {
        return true;
    }
    let package_name = package.name.to_ascii_lowercase();
    !package_name.is_empty()
        && tokens
            .windows(2)
            .any(|window| window[0] == "--bin" && window[1] == package_name)
}

fn generic_manifest_ci_run_match_reason(
    package: &crate::model::PackageInfo,
    command: &str,
    tools: &[&str],
) -> Option<String> {
    if !command_uses_any(command, tools) {
        return None;
    }
    if package.path == "." || command_references_package(package, command) {
        return Some(format!("CI run step references package `{}`", package.name));
    }
    None
}

pub(crate) fn schema_ci_run_match_reason(
    package: Option<&crate::model::PackageInfo>,
    rel: &str,
    command: &str,
) -> Option<String> {
    if !schema_ci_command_is_relevant(rel, command) {
        return None;
    }
    let rel_lower = rel.to_ascii_lowercase();
    let command_lower = command.to_ascii_lowercase();
    if command_lower.contains(&rel_lower)
        || Path::new(rel)
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| command_lower.contains(&name.to_ascii_lowercase()))
    {
        return Some("CI run step references schema path/name".to_string());
    }
    if let Some(package) = package
        && (package.path == "." || command_references_package(package, command))
    {
        return Some(format!(
            "CI run step references schema owner package `{}`",
            package.name
        ));
    }
    None
}

fn schema_ci_command_is_relevant(rel: &str, command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    lower.contains("prisma")
        || lower.contains("migrate")
        || lower.contains("migration")
        || lower.contains("db:")
        || lower.contains("schema.prisma")
        || (rel.to_ascii_lowercase().ends_with(".sql") && lower.contains(".sql"))
}

fn command_uses_any(command: &str, needles: &[&str]) -> bool {
    let lower = command.to_ascii_lowercase();
    needles.iter().any(|needle| lower.contains(needle))
}
