fn runtime_manifest_entrypoints(project: &Project, file: &FileInfo) -> Vec<Surface> {
    let package = project
        .packages
        .iter()
        .find(|package| package.manifest == file.rel);
    let package_path = package
        .map(|package| package.path.clone())
        .unwrap_or_else(|| manifest_parent(&file.rel));
    let package_name = package
        .map(|package| package.name.clone())
        .unwrap_or_else(|| package_name_from_manifest_path(&package_path));
    let Some(name) = Path::new(&file.rel).file_name().and_then(|name| name.to_str()) else {
        return Vec::new();
    };
    match name.to_ascii_lowercase().as_str() {
        "package.json" => {
            js_manifest_cli_entrypoints(project, &file.rel, &package_path, &package_name)
        }
        "cargo.toml" => {
            cargo_manifest_cli_entrypoints(project, &file.rel, &package_path, &package_name)
        }
        "pyproject.toml" => pyproject_manifest_cli_entrypoints(project, &file.rel, &package_path),
        _ => Vec::new(),
    }
}

fn runtime_code_entrypoints(project: &Project, file: &FileInfo) -> Vec<Surface> {
    if file.ext != "rs" {
        return Vec::new();
    }
    let Ok(text) = std::fs::read_to_string(project.root.join(&file.rel)) else {
        return Vec::new();
    };
    clap_subcommand_surfaces(&file.rel, &text)
}

fn clap_subcommand_surfaces(rel: &str, text: &str) -> Vec<Surface> {
    let mut surfaces = Vec::new();
    let mut in_derive_attr = false;
    let mut pending_subcommand_derive = false;
    let mut in_enum = false;
    let mut enum_name = String::new();
    let mut brace_depth = 0isize;
    let mut pending_about = None;
    let mut pending_name = None;
    let mut pending_aliases = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim();
        if !in_enum {
            if trimmed.starts_with("#[derive(") {
                in_derive_attr = !trimmed.contains(')');
                if trimmed.contains("Subcommand") {
                    pending_subcommand_derive = true;
                }
                continue;
            }
            if in_derive_attr {
                if trimmed.contains("Subcommand") {
                    pending_subcommand_derive = true;
                }
                if trimmed.contains(')') {
                    in_derive_attr = false;
                }
                continue;
            }
            if pending_subcommand_derive
                && let Some(name) = rust_enum_name(trimmed)
            {
                enum_name = name.to_string();
                in_enum = true;
                brace_depth += brace_delta(line);
                if brace_depth <= 0 {
                    in_enum = false;
                    pending_subcommand_derive = false;
                }
                continue;
            }
            if !trimmed.starts_with("#[") && !trimmed.is_empty() {
                pending_subcommand_derive = false;
            }
            continue;
        }

        if brace_depth == 1 && trimmed.starts_with("#[command(") {
            if let Some(value) = rust_attr_string_value(trimmed, "about") {
                pending_about = Some(value);
            }
            if let Some(value) = rust_attr_string_value(trimmed, "name") {
                pending_name = Some(value);
            }
            if let Some(value) = rust_attr_string_value(trimmed, "alias") {
                pending_aliases.push(value);
            }
        } else if brace_depth == 1
            && let Some(variant) = rust_enum_variant_name(trimmed)
        {
            let command = pending_name
                .take()
                .unwrap_or_else(|| clap_case(&variant));
            surfaces.push(clap_subcommand_surface(
                rel,
                &enum_name,
                &variant,
                &command,
                pending_about.take(),
                std::mem::take(&mut pending_aliases),
                line_number,
            ));
        }

        brace_depth += brace_delta(line);
        if brace_depth <= 0 {
            in_enum = false;
            pending_subcommand_derive = false;
            enum_name.clear();
            pending_about = None;
            pending_name = None;
            pending_aliases.clear();
        }
    }
    surfaces
}

fn clap_subcommand_surface(
    rel: &str,
    enum_name: &str,
    variant: &str,
    command: &str,
    about: Option<String>,
    aliases: Vec<String>,
    line_number: usize,
) -> Surface {
    let mut example = format!("{command} -> {rel}:{line_number}");
    if !aliases.is_empty() {
        example.push_str(&format!(" (alias: {})", aliases.join(", ")));
    }
    if let Some(about) = about {
        example.push_str(&format!(" - {about}"));
    }
    Surface {
        id: format!("surface:cli_command:{rel}:{enum_name}:{variant}"),
        kind: "cli_command".to_string(),
        path: Some(format!("{rel}#{enum_name}::{variant}")),
        role: Some("runtime_entrypoint".to_string()),
        evidence: "clap_subcommand_enum".to_string(),
        strength: EvidenceStrength::High,
        count: Some(1),
        examples: vec![example],
        hidden_count: 0,
    }
}

fn rust_enum_name(line: &str) -> Option<&str> {
    let mut parts = line.split_whitespace();
    let first = parts.next()?;
    let enum_token = if first == "pub" || first.starts_with("pub(") {
        parts.next()?
    } else {
        first
    };
    if enum_token != "enum" {
        return None;
    }
    parts
        .next()
        .map(|name| name.trim_end_matches('{'))
        .map(|name| name.split('<').next().unwrap_or(name))
        .filter(|name| !name.is_empty())
}

fn rust_enum_variant_name(line: &str) -> Option<String> {
    if line.starts_with('#') || line.starts_with("//") || line.starts_with('}') {
        return None;
    }
    let name = line
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect::<String>();
    if name
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
    {
        Some(name)
    } else {
        None
    }
}

fn rust_attr_string_value(line: &str, key: &str) -> Option<String> {
    let key_at = line.find(key)?;
    let after_key = line[key_at + key.len()..].trim_start();
    let after_equals = after_key.strip_prefix('=')?.trim_start();
    let quote = after_equals.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let mut escaped = false;
    let mut value = String::new();
    for ch in after_equals[quote.len_utf8()..].chars() {
        if escaped {
            value.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == quote {
            return Some(value);
        } else {
            value.push(ch);
        }
    }
    None
}

fn clap_case(value: &str) -> String {
    let mut out = String::new();
    let mut previous_lower_or_digit = false;
    for ch in value.chars() {
        if ch == '_' {
            if !out.ends_with('-') {
                out.push('-');
            }
            previous_lower_or_digit = false;
            continue;
        }
        if ch.is_ascii_uppercase() && previous_lower_or_digit && !out.ends_with('-') {
            out.push('-');
        }
        out.push(ch.to_ascii_lowercase());
        previous_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
    }
    out
}

fn brace_delta(line: &str) -> isize {
    line.chars().fold(0, |depth, ch| match ch {
        '{' => depth + 1,
        '}' => depth - 1,
        _ => depth,
    })
}

fn js_manifest_cli_entrypoints(
    project: &Project,
    manifest: &str,
    package_path: &str,
    package_name: &str,
) -> Vec<Surface> {
    let Ok(text) = std::fs::read_to_string(project.root.join(manifest)) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let Some(bin) = value.get("bin") else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    if let Some(target) = bin.as_str() {
        entries.push((package_name.to_string(), target.to_string()));
    } else if let Some(map) = bin.as_object() {
        entries.extend(
            map.iter()
                .filter_map(|(command, target)| Some((command.clone(), target.as_str()?.to_string()))),
        );
    }
    entries
        .into_iter()
        .map(|(command, target)| {
            let resolved = repo::package_public_target_candidates(package_path, &target)
                .into_iter()
                .find(|candidate| project.files.contains_key(candidate));
            cli_entrypoint_surface(
                manifest,
                &command,
                &target,
                resolved,
                "package_json_bin",
            )
        })
        .collect()
}

fn cargo_manifest_cli_entrypoints(
    project: &Project,
    manifest: &str,
    package_path: &str,
    package_name: &str,
) -> Vec<Surface> {
    let Ok(text) = std::fs::read_to_string(project.root.join(manifest)) else {
        return Vec::new();
    };
    let Ok(value) = toml::from_str::<toml::Value>(&text) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut explicit_bins = BTreeSet::new();
    if let Some(bins) = value.get("bin").and_then(|value| value.as_array()) {
        for bin in bins {
            let Some(command) = bin.get("name").and_then(|value| value.as_str()) else {
                continue;
            };
            let Some(target) = bin.get("path").and_then(|value| value.as_str()) else {
                continue;
            };
            let resolved = repo::package_target_path(package_path, target)
                .filter(|candidate| project.files.contains_key(candidate));
            explicit_bins.insert((command.to_string(), resolved.clone()));
            out.push(cli_entrypoint_surface(
                manifest,
                command,
                target,
                resolved,
                "cargo_bin_target",
            ));
        }
    }
    let default_target = repo::package_target_path(package_path, "src/main.rs");
    if let Some(path) = default_target.filter(|candidate| project.files.contains_key(candidate)) {
        if explicit_bins.contains(&(package_name.to_string(), Some(path.clone()))) {
            return out;
        }
        out.push(cli_entrypoint_surface(
            manifest,
            package_name,
            "src/main.rs",
            Some(path),
            "cargo_default_bin_convention",
        ));
    }
    out
}

fn pyproject_manifest_cli_entrypoints(
    project: &Project,
    manifest: &str,
    package_path: &str,
) -> Vec<Surface> {
    let Ok(text) = std::fs::read_to_string(project.root.join(manifest)) else {
        return Vec::new();
    };
    let Ok(value) = toml::from_str::<toml::Value>(&text) else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    collect_toml_script_table(
        &value,
        &["project", "scripts"],
        "pyproject_project_scripts",
        &mut entries,
    );
    collect_toml_script_table(
        &value,
        &["project", "gui-scripts"],
        "pyproject_project_gui_scripts",
        &mut entries,
    );
    collect_toml_script_table(
        &value,
        &["tool", "poetry", "scripts"],
        "pyproject_poetry_scripts",
        &mut entries,
    );
    entries
        .into_iter()
        .map(|(command, target, evidence)| {
            let resolved = python_entrypoint_target(project, package_path, &target);
            cli_entrypoint_surface(manifest, &command, &target, resolved, evidence)
        })
        .collect()
}

fn collect_toml_script_table(
    value: &toml::Value,
    path: &[&str],
    evidence: &'static str,
    out: &mut Vec<(String, String, &'static str)>,
) {
    let Some(table) = toml_path(value, path).and_then(|value| value.as_table()) else {
        return;
    };
    out.extend(
        table
            .iter()
            .filter_map(|(command, target)| Some((command.clone(), target.as_str()?.to_string(), evidence))),
    );
}

fn toml_path<'a>(value: &'a toml::Value, path: &[&str]) -> Option<&'a toml::Value> {
    path.iter()
        .try_fold(value, |current, segment| current.get(*segment))
}

fn python_entrypoint_target(project: &Project, package_path: &str, target: &str) -> Option<String> {
    let module = target.split(':').next()?.trim();
    if module.is_empty() {
        return None;
    }
    let rel = module.replace('.', "/");
    [
        format!("{rel}.py"),
        format!("{rel}/__init__.py"),
        format!("src/{rel}.py"),
        format!("src/{rel}/__init__.py"),
    ]
    .into_iter()
    .filter_map(|candidate| repo::package_target_path(package_path, &candidate))
    .find(|candidate| project.files.contains_key(candidate))
}

fn manifest_parent(rel: &str) -> String {
    Path::new(rel)
        .parent()
        .map(|parent| repo::normalize_rel_path(&parent.to_string_lossy()))
        .filter(|parent| !parent.is_empty())
        .unwrap_or_else(|| ".".to_string())
}

fn package_name_from_manifest_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
        .to_string()
}

fn cli_entrypoint_surface(
    manifest: &str,
    command: &str,
    target: &str,
    resolved: Option<String>,
    evidence: &str,
) -> Surface {
    let display_target = resolved.clone().unwrap_or_else(|| target.to_string());
    Surface {
        id: format!("surface:cli_entrypoint:{manifest}:{command}"),
        kind: "cli_entrypoint".to_string(),
        path: Some(resolved.unwrap_or_else(|| manifest.to_string())),
        role: Some("runtime_entrypoint".to_string()),
        evidence: evidence.to_string(),
        strength: EvidenceStrength::Hard,
        count: Some(1),
        examples: vec![format!("{command} -> {display_target}")],
        hidden_count: 0,
    }
}
