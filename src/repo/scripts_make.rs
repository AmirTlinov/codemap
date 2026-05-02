fn makefile_scripts(root: &Path) -> Vec<ScriptInfo> {
    ["Makefile", "makefile"]
        .iter()
        .find_map(|name| {
            let path = root.join(name);
            let text = fs::read_to_string(&path).ok()?;
            Some(make_like_scripts_from_text(&text, "make", "Makefile target"))
        })
        .unwrap_or_default()
}

fn justfile_scripts(root: &Path) -> Vec<ScriptInfo> {
    ["justfile", "Justfile"]
        .iter()
        .find_map(|name| {
            let path = root.join(name);
            let text = fs::read_to_string(&path).ok()?;
            Some(make_like_scripts_from_text(&text, "just", "justfile target"))
        })
        .unwrap_or_default()
}

fn make_like_scripts_from_text(text: &str, runner: &str, reason: &str) -> Vec<ScriptInfo> {
    let mut scripts = Vec::new();
    for line in text.lines() {
        let Some(targets) = make_like_targets(line) else {
            continue;
        };
        for target in targets {
            scripts.push(ScriptInfo {
                name: target.clone(),
                command: format!("{runner} {}", shell_quote_script_target(&target)),
                reason: format!("{reason}: {target}"),
            });
        }
    }
    scripts
}

fn make_like_targets(line: &str) -> Option<Vec<String>> {
    if line.starts_with(char::is_whitespace) {
        return None;
    }
    let code = line.split('#').next()?.trim();
    if code.is_empty() || code.starts_with('.') || code.contains(":=") || code.contains("?=") {
        return None;
    }
    let (left, _) = code.split_once(':')?;
    if left.contains('=') {
        return None;
    }
    let targets = left
        .split_whitespace()
        .filter(|target| !target.is_empty() && !target.contains('%') && !target.starts_with('.'))
        .map(str::to_string)
        .collect::<Vec<_>>();
    (!targets.is_empty()).then_some(targets)
}

fn shell_quote_script_target(target: &str) -> String {
    if target
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':' | '/'))
    {
        target.to_string()
    } else {
        format!("'{}'", target.replace('\'', "'\\''"))
    }
}
