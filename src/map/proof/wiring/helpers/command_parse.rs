// Responsibility: map-proof-wiring-helpers-command-parse
use crate::map::package_for_rel;
use crate::model::{Project, ProofSurface};
use crate::repo;

#[derive(Debug)]
pub(crate) struct ParsedProofCommand {
    pub(crate) cwd: Option<String>,
    pub(crate) runner: String,
    pub(crate) args: Vec<String>,
}

pub(crate) fn parse_static_command(command: &str) -> Option<ParsedProofCommand> {
    let mut tokens = shell_words(command);
    let mut cwd = None;
    if tokens.len() >= 4 && tokens.first().is_some_and(|token| token == "cd") {
        cwd = tokens.get(1).cloned();
        if tokens.get(2).is_none_or(|token| token != "&&") {
            return None;
        }
        tokens.drain(0..3);
    }
    while tokens
        .first()
        .is_some_and(|token| token.contains('=') && !token.starts_with('-'))
    {
        tokens.remove(0);
    }
    let runner = tokens.first()?.to_ascii_lowercase();
    Some(ParsedProofCommand {
        cwd,
        runner,
        args: tokens.into_iter().skip(1).collect(),
    })
}

fn shell_words(command: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for ch in command.chars() {
        if matches!(quote, Some(q) if q == ch) {
            quote = None;
            continue;
        }
        if quote.is_none() && matches!(ch, '\'' | '"') {
            quote = Some(ch);
            continue;
        }
        if quote.is_none() && ch.is_whitespace() {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(ch);
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

pub(crate) fn unquote_shell_token(token: &str) -> &str {
    token
        .strip_prefix('\'')
        .and_then(|value| value.strip_suffix('\''))
        .or_else(|| {
            token
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
        })
        .unwrap_or(token)
}

pub(crate) fn package_for_parsed_command<'a>(
    project: &'a Project,
    parsed: &ParsedProofCommand,
    proof: &ProofSurface,
) -> Option<&'a crate::model::PackageInfo> {
    if let Some(filter) = package_filter_arg(&parsed.args) {
        return project.packages.iter().find(|package| {
            package.name == filter || package.name.ends_with(&format!("/{filter}"))
        });
    }
    if let Some(cwd) = parsed.cwd.as_deref() {
        let cwd = repo::normalize_rel_path(cwd);
        return project.packages.iter().find(|package| {
            package.path == cwd || package.manifest.starts_with(&format!("{cwd}/"))
        });
    }
    proof
        .path
        .as_deref()
        .and_then(|path| package_for_rel(project, path))
        .or_else(|| project.packages.iter().find(|package| package.path == "."))
}

fn package_filter_arg(args: &[String]) -> Option<String> {
    for pair in args.windows(2) {
        if pair[0] == "--filter" {
            return Some(pair[1].trim_start_matches('@').to_string());
        }
    }
    args.iter().find_map(|arg| {
        arg.strip_prefix("--filter=")
            .map(|value| value.trim_start_matches('@').to_string())
    })
}

pub(crate) fn package_script_name_from_command(parsed: &ParsedProofCommand) -> Option<String> {
    match parsed.runner.as_str() {
        "npm" | "pnpm" | "bun" => {
            let args = &parsed.args;
            for (index, arg) in args.iter().enumerate() {
                if arg == "run" {
                    return args.get(index + 1).cloned();
                }
            }
            if args.first().is_some_and(|arg| arg == "test") {
                return Some("test".to_string());
            }
            None
        }
        "yarn" => parsed.args.first().cloned(),
        _ => None,
    }
}
