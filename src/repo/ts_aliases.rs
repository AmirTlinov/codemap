#[derive(Debug, Clone)]
struct TsPathAlias {
    config_dir: String,
    pattern: String,
    targets: Vec<String>,
}

fn detect_ts_path_aliases(root: &Path, files: &BTreeMap<String, FileInfo>) -> Vec<TsPathAlias> {
    let mut aliases = Vec::new();
    for rel in files.keys() {
        if Path::new(rel).file_name().and_then(|name| name.to_str()) != Some("tsconfig.json") {
            continue;
        }
        aliases.extend(read_ts_path_aliases(root, rel));
    }
    aliases.sort_by(|a, b| {
        b.pattern
            .len()
            .cmp(&a.pattern.len())
            .then_with(|| a.pattern.cmp(&b.pattern))
    });
    aliases
}

fn read_ts_path_aliases(root: &Path, rel: &str) -> Vec<TsPathAlias> {
    let Ok(text) = fs::read_to_string(root.join(rel)) else {
        return Vec::new();
    };
    let Ok(value) = parse_tsconfig_json(&text) else {
        return Vec::new();
    };
    let Some(options) = value
        .get("compilerOptions")
        .and_then(|value| value.as_object())
    else {
        return Vec::new();
    };
    let config_dir = manifest_dir(rel);
    let base_url = options
        .get("baseUrl")
        .and_then(|value| value.as_str())
        .unwrap_or(".");
    let base = normalize_rel_path(&format!("{config_dir}/{base_url}"));
    let Some(paths) = options.get("paths").and_then(|value| value.as_object()) else {
        return Vec::new();
    };
    let mut aliases = Vec::new();
    for (pattern, targets) in paths {
        let Some(targets) = targets.as_array() else {
            continue;
        };
        let targets = targets
            .iter()
            .filter_map(|target| target.as_str())
            .map(|target| normalize_rel_path(&format!("{base}/{target}")))
            .collect::<Vec<_>>();
        if !targets.is_empty() {
            aliases.push(TsPathAlias {
                config_dir: config_dir.clone(),
                pattern: pattern.to_string(),
                targets,
            });
        }
    }
    aliases
}

fn parse_tsconfig_json(text: &str) -> std::result::Result<serde_json::Value, serde_json::Error> {
    serde_json::from_str(text).or_else(|strict_error| {
        let Some(json) = strip_jsonc_comments_and_trailing_commas(text) else {
            return Err(strict_error);
        };
        serde_json::from_str(&json)
    })
}

fn strip_jsonc_comments_and_trailing_commas(text: &str) -> Option<String> {
    Some(strip_json_trailing_commas(&strip_jsonc_comments(text)?))
}

fn strip_jsonc_comments(text: &str) -> Option<String> {
    let chars = text.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let mut in_string = false;
    let mut escape = false;

    while i < chars.len() {
        let ch = chars[i];
        if in_string {
            out.push(ch);
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if ch == '"' {
            in_string = true;
            out.push(ch);
            i += 1;
            continue;
        }

        if ch == '/' && chars.get(i + 1) == Some(&'/') {
            i += 2;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            if i < chars.len() {
                out.push('\n');
                i += 1;
            }
            continue;
        }

        if ch == '/' && chars.get(i + 1) == Some(&'*') {
            i += 2;
            let mut closed = false;
            while i + 1 < chars.len() {
                if chars[i] == '\n' {
                    out.push('\n');
                }
                if chars[i] == '*' && chars[i + 1] == '/' {
                    i += 2;
                    closed = true;
                    break;
                }
                i += 1;
            }
            if !closed {
                return None;
            }
            continue;
        }

        out.push(ch);
        i += 1;
    }

    Some(out)
}

fn strip_json_trailing_commas(text: &str) -> String {
    let mut out = Vec::with_capacity(text.len());
    let mut in_string = false;
    let mut escape = false;

    for ch in text.chars() {
        if in_string {
            out.push(ch);
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            out.push(ch);
            continue;
        }

        if matches!(ch, '}' | ']') {
            let mut index = out.len();
            while index > 0 && out[index - 1].is_whitespace() {
                index -= 1;
            }
            if index > 0 && out[index - 1] == ',' {
                out.remove(index - 1);
            }
        }

        out.push(ch);
    }

    out.into_iter().collect()
}

fn ts_alias_applies_to_importer(alias: &TsPathAlias, from: &str) -> bool {
    alias.config_dir == "."
        || from == alias.config_dir
        || from.starts_with(&format!("{}/", alias.config_dir.trim_end_matches('/')))
}

fn resolve_ts_path_alias(
    alias: &TsPathAlias,
    spec: &str,
    paths: &BTreeSet<String>,
) -> Option<String> {
    let wildcard = match_pattern_wildcard(&alias.pattern, spec)?;
    for target in &alias.targets {
        let base = if let Some(wildcard) = wildcard.as_deref() {
            target.replace('*', wildcard)
        } else {
            target.clone()
        };
        if let Some(resolved) = resolve_path_like(&base, paths) {
            return Some(resolved);
        }
    }
    None
}

fn match_pattern_wildcard(pattern: &str, value: &str) -> Option<Option<String>> {
    if !pattern.contains('*') {
        return (pattern == value).then_some(None);
    }
    let (prefix, suffix) = pattern.split_once('*')?;
    if !value.starts_with(prefix) || !value.ends_with(suffix) {
        return None;
    }
    let end = value.len().saturating_sub(suffix.len());
    Some(Some(value[prefix.len()..end].to_string()))
}
