fn unsafe_proof_command_reason(command: &str) -> Option<&'static str> {
    let lower = command
        .to_ascii_lowercase()
        .replace(['\'', '"', '`'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let deny = [
        ("deploy", "deploy command"),
        ("release", "release command"),
        ("publish", "publish command"),
        ("migrate", "migration command"),
        ("db push", "database mutation"),
        ("db:push", "database mutation"),
        ("db:normalize", "database mutation"),
        (" watch", "watch command"),
        (":watch", "watch command"),
        (" run dev", "development server"),
        (" run start", "service startup"),
        (" run serve", "service startup"),
        (" run preview", "service startup"),
        ("kubectl", "cluster mutation"),
        ("helm", "cluster mutation"),
        ("terraform", "infrastructure mutation"),
        ("pulumi", "infrastructure mutation"),
        ("ansible", "remote mutation"),
        ("ssh ", "remote shell"),
        ("scp ", "remote copy"),
        ("rsync ", "remote/local sync"),
        ("curl ", "network mutation risk"),
        ("wget ", "network mutation risk"),
        ("rm -", "destructive file operation"),
        ("dropdb", "database mutation"),
        ("psql ", "database mutation"),
        ("mysql ", "database mutation"),
        ("docker compose up", "service startup"),
        ("docker-compose up", "service startup"),
    ];
    deny.iter()
        .find_map(|(needle, reason)| lower.contains(needle).then_some(*reason))
}

fn unsafe_shell_syntax_reason(command: &str) -> Option<&'static str> {
    let mut chars = command.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\n' | '\r' => return Some("multi-line shell command"),
            ';' | '|' | '`' | '$' | '>' | '<' => return Some("shell control syntax"),
            '&' if chars.peek() == Some(&'&') => {
                chars.next();
            }
            '&' => return Some("shell background syntax"),
            _ => {}
        }
    }
    let and_count = command.match_indices("&&").count();
    if and_count > 1 {
        return Some("multiple shell command separators");
    }
    if and_count == 1 {
        let Some((prefix, _tail)) = command.split_once("&&") else {
            return Some("shell command separator");
        };
        if !safe_cd_prefix(prefix) {
            return Some("only scoped cd prefix may compose proof commands");
        }
    }
    None
}

fn safe_proof_command(command: &str) -> bool {
    let command = command.trim();
    if let Some((prefix, tail)) = command.split_once("&&") {
        return safe_cd_prefix(prefix) && safe_proof_command(tail);
    }
    let lower = command.to_ascii_lowercase();
    if crate::proof_classification::proof_command_is_readonly_migration_status(command) {
        return true;
    }
    if safe_proof_command_has_direct_prefix(&lower) {
        return true;
    }
    safe_package_script_command(&lower)
        || safe_package_selector_command(&lower)
        || safe_direct_script_command(&lower)
        || safe_make_or_just_command(&lower)
}

fn safe_proof_command_has_direct_prefix(command: &str) -> bool {
    let safe_prefixes = [
        "cargo test",
        "cargo nextest",
        "cargo clippy",
        "cargo check",
        "cargo build",
        "cargo fmt --check",
        "go test",
        "pytest",
        "python -m pytest",
        "swift test",
        "npm test",
        "pnpm test",
        "yarn test",
        "bun test",
        "vitest",
        "playwright test",
        "pnpm exec vitest",
        "pnpm exec jest",
        "pnpm exec uvu",
        "pnpm exec ava",
        "pnpm exec mocha",
        "pnpm exec node --test",
        "pnpm exec tsx",
        "npm exec vitest",
        "npm exec jest",
        "npm exec uvu",
        "npm exec ava",
        "npm exec mocha",
        "npm exec node --test",
        "npm exec tsx",
        "npx vitest",
        "npx jest",
        "npx uvu",
        "npx ava",
        "npx mocha",
        "npx node --test",
        "npx tsx",
        "yarn vitest",
        "yarn jest",
        "yarn uvu",
        "yarn ava",
        "yarn mocha",
        "yarn node --test",
        "yarn tsx",
        "bunx vitest",
        "bunx jest",
        "bunx uvu",
        "bunx ava",
        "bunx mocha",
        "bunx node --test",
        "bunx tsx",
        "pnpm exec playwright test",
        "npm exec playwright test",
        "npx playwright test",
        "yarn playwright test",
        "bunx playwright test",
        "codemap boundaries",
        "codemap proof-map",
        "codemap changed",
    ];
    safe_prefixes
        .iter()
        .any(|prefix| command_has_prefix(command, prefix))
}

fn safe_cd_prefix(prefix: &str) -> bool {
    let prefix = prefix.trim();
    let Some(path) = cd_prefix_path(prefix) else {
        return false;
    };
    !path.is_empty()
        && path != "~"
        && !path.starts_with('/')
        && !path.starts_with("~/")
        && !path.starts_with('-')
        && !path.split('/').any(|part| part == "..")
}

fn cd_prefix_path(prefix: &str) -> Option<String> {
    let rest = prefix.strip_prefix("cd ")?.trim();
    if rest.is_empty() {
        return None;
    }
    if let Some(quote) = rest.chars().next().filter(|ch| *ch == '\'' || *ch == '"') {
        let suffix = rest.strip_prefix(quote)?.strip_suffix(quote)?;
        return Some(suffix.to_string());
    }
    if rest.chars().any(char::is_whitespace) {
        return None;
    }
    Some(rest.to_string())
}

fn command_has_prefix(command: &str, prefix: &str) -> bool {
    command == prefix
        || command
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with(char::is_whitespace))
}

fn safe_package_script_command(command: &str) -> bool {
    let parts = command.split_whitespace().collect::<Vec<_>>();
    let Some((runner, script)) = package_runner_and_script(&parts) else {
        return false;
    };
    matches!(runner, "npm" | "pnpm" | "yarn" | "bun") && safe_script_name(script)
}

fn package_runner_and_script<'a>(parts: &'a [&str]) -> Option<(&'a str, &'a str)> {
    match parts {
        ["npm", "run", script, ..] => Some(("npm", *script)),
        ["pnpm", "run", script, ..] => Some(("pnpm", *script)),
        ["yarn", script, ..] => Some(("yarn", *script)),
        ["bun", "run", script, ..] => Some(("bun", *script)),
        _ => None,
    }
}

fn safe_package_selector_command(command: &str) -> bool {
    let parts = command.split_whitespace().collect::<Vec<_>>();
    let Some(after_selector) = package_selector_command_after_selector(&parts) else {
        return false;
    };
    match after_selector {
        ["run", script, ..] | [script, ..] => safe_script_name(script),
        _ => false,
    }
}

fn package_selector_command_after_selector<'a>(parts: &'a [&str]) -> Option<&'a [&'a str]> {
    let [runner, tail @ ..] = parts else {
        return None;
    };
    if !matches!(*runner, "npm" | "pnpm" | "yarn" | "bun") {
        return None;
    }
    let mut index = 0usize;
    while index < tail.len() {
        let token = tail[index].trim_matches(|ch| matches!(ch, '"' | '\''));
        if selector_flag_takes_value(token) {
            if index + 1 >= tail.len() || !selector_value_is_safe(tail[index + 1]) {
                return None;
            }
            index += 2;
        } else if selector_flag_is_inline(token) || selector_flag_is_value_less(token) {
            index += 1;
        } else {
            return Some(&tail[index..]);
        }
    }
    None
}

fn selector_flag_takes_value(token: &str) -> bool {
    matches!(token, "--filter" | "-f" | "-F" | "--workspace" | "--package" | "-p")
}

fn selector_flag_is_inline(token: &str) -> bool {
    token.split_once('=').is_some_and(|(flag, value)| {
        selector_flag_takes_value(flag) && selector_value_is_safe(value)
    })
}

fn selector_flag_is_value_less(token: &str) -> bool {
    matches!(token, "--workspace-root" | "-w" | "--recursive" | "-r")
}

fn selector_value_is_safe(selector: &str) -> bool {
    let selector = selector.trim_matches(|ch| matches!(ch, '"' | '\''));
    !selector.is_empty()
        && !selector.starts_with('-')
        && !selector.split('/').any(|part| part == "..")
}

fn safe_direct_script_command(command: &str) -> bool {
    let parts = command.split_whitespace().collect::<Vec<_>>();
    let Some(script) = parts.first().copied() else {
        return false;
    };
    let script = script.trim_matches(|ch| matches!(ch, '"' | '\''));
    if !safe_relative_script_path(script) {
        return false;
    }
    let name = Path::new(script)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(script);
    safe_script_name(script_name_stem(name))
}

fn safe_relative_script_path(script: &str) -> bool {
    !script.is_empty()
        && !script.starts_with('/')
        && !script.starts_with("~/")
        && !script.starts_with('-')
        && !script.split('/').any(|part| part == "..")
        && (script.starts_with("./") || script.contains('/'))
}

fn script_name_stem(name: &str) -> &str {
    name.strip_suffix(".sh")
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
        .unwrap_or(name)
}

fn safe_script_name(script: &str) -> bool {
    let script = script.trim_matches('\'').trim_matches('"');
    let lower = script.to_ascii_lowercase();
    if readonly_migration_status_script_name(&lower) {
        return true;
    }
    if [
        "deploy",
        "release",
        "publish",
        "migrate",
        "db:push",
        "db:normalize",
        "install",
        "codegen",
        "generate",
        "seed",
        "studio",
        "watch",
        "destroy",
        "delete",
        "drop",
        "reset",
        "truncate",
        "wipe",
        "remove",
        "prune",
    ]
        .iter()
        .any(|marker| lower.contains(marker))
        || unsafe_lifecycle_script_name(&lower)
    {
        return false;
    }
    script.contains("test")
        || script.starts_with("typecheck")
        || script == "check"
        || script.starts_with("check:")
        || script.contains("validate")
        || script.contains("verify")
        || script.contains("e2e")
        || script.contains("smoke")
        || script == "doctor"
        || script.starts_with("doctor:")
        || script.contains("proof")
        || script == "lint"
        || script.starts_with("lint:")
        || script == "build"
        || script.starts_with("build:")
}

fn readonly_migration_status_script_name(lower: &str) -> bool {
    lower == "db:migrate:status"
        || lower == "migrate:status"
        || lower.ends_with(":db:migrate:status")
        || lower.ends_with(":migrate:status")
}

fn unsafe_lifecycle_script_name(lower: &str) -> bool {
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

fn safe_make_or_just_command(command: &str) -> bool {
    let parts = command.split_whitespace().collect::<Vec<_>>();
    matches!(
        parts.as_slice(),
        ["make", target, ..] | ["just", target, ..]
            if safe_script_name(target)
    )
}
