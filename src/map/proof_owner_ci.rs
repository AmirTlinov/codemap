#[derive(Debug, Clone)]
struct CiRunStep {
    command: String,
    line: usize,
}

fn ci_run_steps(text: &str) -> Vec<CiRunStep> {
    let lines = text.lines().collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut index = 0usize;
    while index < lines.len() {
        let Some(spec) = ci_run_spec(lines[index]) else {
            index += 1;
            continue;
        };
        if ci_value_is_block_scalar(&spec.value) {
            index += 1;
            while index < lines.len() {
                let line = lines[index];
                if line.trim().is_empty() {
                    index += 1;
                    continue;
                }
                let indent = leading_whitespace_count(line);
                if indent <= spec.indent {
                    break;
                }
                let command = trim_yaml_scalar(line.trim());
                if !command.is_empty() && !command.starts_with('#') {
                    out.push(CiRunStep {
                        command,
                        line: index + 1,
                    });
                }
                index += 1;
            }
            continue;
        }
        let command = trim_yaml_scalar(&spec.value);
        if !command.is_empty() {
            out.push(CiRunStep {
                command,
                line: index + 1,
            });
        }
        index += 1;
    }
    out
}

#[derive(Debug, Clone)]
struct CiRunSpec {
    value: String,
    indent: usize,
}

fn ci_run_spec(line: &str) -> Option<CiRunSpec> {
    let indent = leading_whitespace_count(line);
    let trimmed = line.trim_start();
    let value = trimmed
        .strip_prefix("- run:")
        .or_else(|| trimmed.strip_prefix("run:"))?
        .trim()
        .to_string();
    Some(CiRunSpec { value, indent })
}

fn ci_inline_run_command(line: &str) -> Option<String> {
    let spec = ci_run_spec(line)?;
    if ci_value_is_block_scalar(&spec.value) {
        return None;
    }
    let command = trim_yaml_scalar(&spec.value);
    (!command.is_empty()).then_some(command)
}

fn ci_value_is_block_scalar(value: &str) -> bool {
    value
        .split_whitespace()
        .next()
        .is_some_and(|token| token.starts_with('|') || token.starts_with('>'))
}

fn trim_yaml_scalar(value: &str) -> String {
    let mut value = value.trim().to_string();
    if value.len() >= 2 {
        let first = value.as_bytes()[0] as char;
        let last = value.as_bytes()[value.len() - 1] as char;
        if matches!(first, '"' | '\'') && first == last {
            value = value[1..value.len() - 1].to_string();
        }
    }
    value.trim().to_string()
}

fn leading_whitespace_count(line: &str) -> usize {
    line.chars().take_while(|ch| ch.is_whitespace()).count()
}

fn manifest_ci_run_match_reason(
    package: &crate::model::PackageInfo,
    scripts: &[(String, String, usize)],
    command: &str,
) -> Option<String> {
    match package.ecosystem.as_str() {
        "javascript" => javascript_manifest_ci_run_match_reason(package, scripts, command),
        "rust" => rust_manifest_ci_run_match_reason(package, command),
        "python" => generic_manifest_ci_run_match_reason(package, command, &["pytest", "python"]),
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
    let script = scripts
        .iter()
        .find(|(name, _, _)| command_invokes_script(command, name));
    let package_ref = command_references_package(package, command);
    if package.path == "." {
        if let Some((name, _, _)) = script {
            return Some(format!("CI run step invokes root package script `{name}`"));
        }
        if command_uses_any(command, &["pnpm", "npm", "yarn", "bun"]) {
            return Some("CI run step uses root package manager".to_string());
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
    if command_uses_any(command, &["pnpm", "npm", "yarn", "bun"]) {
        return Some(format!("CI run step references package `{}`", package.name));
    }
    None
}

fn rust_manifest_ci_run_match_reason(
    package: &crate::model::PackageInfo,
    command: &str,
) -> Option<String> {
    if !command_uses_any(command, &["cargo"]) {
        return None;
    }
    if package.path == "." {
        return Some("CI run step uses root Cargo manifest".to_string());
    }
    if command_references_package(package, command) {
        return Some(format!("CI run step references Cargo package `{}`", package.name));
    }
    None
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

fn schema_ci_run_match_reason(
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

fn command_references_package(package: &crate::model::PackageInfo, command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    let name = package.name.to_ascii_lowercase();
    let path = package.path.to_ascii_lowercase();
    (!name.is_empty() && lower.contains(&name))
        || (package.path != "." && !path.is_empty() && lower.contains(&path))
}

fn command_uses_any(command: &str, needles: &[&str]) -> bool {
    let lower = command.to_ascii_lowercase();
    needles.iter().any(|needle| lower.contains(needle))
}

fn command_invokes_script(command: &str, script: &str) -> bool {
    let script = script.to_ascii_lowercase();
    command_tokens(command).iter().any(|token| token == &script)
}

fn command_tokens(command: &str) -> Vec<String> {
    command
        .split(|ch: char| {
            !(ch.is_ascii_alphanumeric()
                || matches!(ch, '_' | '-' | ':' | '@' | '/' | '.' | '=' | '*'))
        })
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

fn strip_inline_shell_comment(line: &str) -> String {
    let mut in_single = false;
    let mut in_double = false;
    let mut previous_was_escape = false;
    let mut previous_was_whitespace = true;
    for (index, ch) in line.char_indices() {
        if previous_was_escape {
            previous_was_escape = false;
            previous_was_whitespace = ch.is_whitespace();
            continue;
        }
        if ch == '\\' && !in_single {
            previous_was_escape = true;
            previous_was_whitespace = false;
            continue;
        }
        if ch == '\'' && !in_double {
            in_single = !in_single;
            previous_was_whitespace = false;
            continue;
        }
        if ch == '"' && !in_single {
            in_double = !in_double;
            previous_was_whitespace = false;
            continue;
        }
        if ch == '#' && !in_single && !in_double && previous_was_whitespace {
            return line[..index].trim_end().to_string();
        }
        previous_was_whitespace = ch.is_whitespace();
    }
    line.to_string()
}

fn prisma_env_names(line: &str) -> Vec<String> {
    let code = code_shape_without_literal_content(line);
    let mut out = Vec::new();
    for start in find_all(&code, "env(") {
        if let Some(name) = quoted_literal_at(&line[start + "env(".len()..]) {
            out.push(name);
        }
    }
    out
}
