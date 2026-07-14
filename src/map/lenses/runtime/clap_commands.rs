// Responsibility: runtime-lens-clap-commands
use crate::model::{EvidenceStrength, FileInfo, Project, Surface};
use std::collections::BTreeMap;
use std::collections::BTreeSet;

#[derive(Clone)]
struct ClapCommandEnum {
    name: String,
    variants: Vec<ClapCommandVariant>,
}

#[derive(Clone)]
struct ClapCommandVariant {
    name: String,
    command: String,
    about: Option<String>,
    aliases: Vec<String>,
    line_number: usize,
    payload_type: Option<String>,
}

pub(crate) fn runtime_code_entrypoints(project: &Project, file: &FileInfo) -> Vec<Surface> {
    if file.ext != "rs" {
        return Vec::new();
    }
    let Ok(text) = std::fs::read_to_string(project.root.join(&file.rel)) else {
        return Vec::new();
    };
    clap_subcommand_surfaces(&file.rel, &text)
}

fn clap_subcommand_surfaces(rel: &str, text: &str) -> Vec<Surface> {
    let enums = clap_subcommand_enums(text);
    let nested = clap_subcommand_struct_fields(text);
    let prefixes = clap_command_prefixes(&enums, &nested);
    let mut surfaces = Vec::new();
    for command_enum in &enums {
        let enum_prefixes = prefixes
            .get(&command_enum.name)
            .cloned()
            .unwrap_or_else(|| vec![String::new()]);
        for variant in &command_enum.variants {
            for prefix in &enum_prefixes {
                let command = join_command_prefix(prefix, &variant.command);
                surfaces.push(clap_subcommand_surface(
                    rel,
                    &command_enum.name,
                    variant,
                    &command,
                ));
            }
        }
    }
    surfaces
}

fn clap_command_prefixes(
    enums: &[ClapCommandEnum],
    nested: &BTreeMap<String, String>,
) -> BTreeMap<String, Vec<String>> {
    let enum_names = enums
        .iter()
        .map(|item| item.name.clone())
        .collect::<BTreeSet<_>>();
    let mut parents: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for command_enum in enums {
        for variant in &command_enum.variants {
            let Some(payload) = &variant.payload_type else {
                continue;
            };
            let Some(child_enum) = nested.get(payload) else {
                continue;
            };
            if enum_names.contains(child_enum) {
                parents
                    .entry(child_enum.clone())
                    .or_default()
                    .push((command_enum.name.clone(), variant.command.clone()));
            }
        }
    }
    let mut memo = BTreeMap::new();
    for name in enum_names {
        clap_prefixes_for_enum(&name, &parents, &mut memo, &mut BTreeSet::new());
    }
    memo
}

fn clap_prefixes_for_enum(
    name: &str,
    parents: &BTreeMap<String, Vec<(String, String)>>,
    memo: &mut BTreeMap<String, Vec<String>>,
    stack: &mut BTreeSet<String>,
) -> Vec<String> {
    if let Some(prefixes) = memo.get(name) {
        return prefixes.clone();
    }
    if !stack.insert(name.to_string()) {
        return vec![String::new()];
    }
    let mut out = Vec::new();
    if let Some(parent_edges) = parents.get(name) {
        for (parent_enum, parent_command) in parent_edges {
            for prefix in clap_prefixes_for_enum(parent_enum, parents, memo, stack) {
                out.push(join_command_prefix(&prefix, parent_command));
            }
        }
    }
    stack.remove(name);
    if out.is_empty() {
        out.push(String::new());
    }
    out.sort();
    out.dedup();
    memo.insert(name.to_string(), out.clone());
    out
}

fn clap_subcommand_enums(text: &str) -> Vec<ClapCommandEnum> {
    let mut enums = Vec::new();
    let mut in_derive_attr = false;
    let mut pending_subcommand_derive = false;
    let mut current = None;
    let mut brace_depth = 0isize;
    let mut pending_about = None;
    let mut pending_name = None;
    let mut pending_aliases = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim();
        if current.is_none() {
            clap_update_derive_state(trimmed, &mut in_derive_attr, &mut pending_subcommand_derive);
            if pending_subcommand_derive && let Some(name) = rust_enum_name(trimmed) {
                current = Some(ClapCommandEnum {
                    name: name.to_string(),
                    variants: Vec::new(),
                });
                brace_depth += brace_delta(line);
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
            && let Some((name, payload_type)) = rust_enum_variant(trimmed)
        {
            let command = pending_name.take().unwrap_or_else(|| clap_case(&name));
            if let Some(command_enum) = &mut current {
                command_enum.variants.push(ClapCommandVariant {
                    name,
                    command,
                    about: pending_about.take(),
                    aliases: std::mem::take(&mut pending_aliases),
                    line_number,
                    payload_type,
                });
            }
        }

        brace_depth += brace_delta(line);
        if brace_depth <= 0 {
            if let Some(command_enum) = current.take() {
                enums.push(command_enum);
            }
            pending_subcommand_derive = false;
            pending_about = None;
            pending_name = None;
            pending_aliases.clear();
        }
    }
    enums
}

fn clap_subcommand_struct_fields(text: &str) -> BTreeMap<String, String> {
    let mut fields = BTreeMap::new();
    let mut current_struct = None;
    let mut brace_depth = 0isize;
    let mut pending_subcommand_field = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if current_struct.is_none() {
            if let Some(name) = rust_struct_name(trimmed) {
                current_struct = Some(name.to_string());
                brace_depth += brace_delta(line);
            }
            continue;
        }
        if brace_depth == 1 && trimmed.starts_with("#[command(") && trimmed.contains("subcommand") {
            pending_subcommand_field = true;
        } else if brace_depth == 1
            && pending_subcommand_field
            && let Some(field_type) = rust_field_type(trimmed)
        {
            if let Some(name) = &current_struct {
                fields.insert(name.clone(), field_type);
            }
            pending_subcommand_field = false;
        }
        brace_depth += brace_delta(line);
        if brace_depth <= 0 {
            current_struct = None;
            pending_subcommand_field = false;
        }
    }
    fields
}

fn clap_subcommand_surface(
    rel: &str,
    enum_name: &str,
    variant: &ClapCommandVariant,
    command: &str,
) -> Surface {
    let mut example = format!("{command} -> {rel}:{}", variant.line_number);
    if !variant.aliases.is_empty() {
        example.push_str(&format!(" (alias: {})", variant.aliases.join(", ")));
    }
    if let Some(about) = &variant.about {
        example.push_str(&format!(" - {about}"));
    }
    Surface {
        id: format!("surface:cli_command:{rel}:{enum_name}:{}", variant.name),
        kind: "cli_command".to_string(),
        path: Some(format!("{rel}#{enum_name}::{}", variant.name)),
        role: Some("runtime_entrypoint".to_string()),
        evidence: "clap_subcommand_enum".to_string(),
        strength: EvidenceStrength::High,
        count: Some(1),
        examples: vec![example],
        hidden_count: 0,
    }
}

fn clap_update_derive_state(line: &str, in_attr: &mut bool, pending_subcommand: &mut bool) {
    if line.starts_with("#[derive(") {
        *in_attr = !line.contains(')');
        if line.contains("Subcommand") {
            *pending_subcommand = true;
        }
    } else if *in_attr {
        if line.contains("Subcommand") {
            *pending_subcommand = true;
        }
        if line.contains(')') {
            *in_attr = false;
        }
    } else if !line.starts_with("#[") && !line.is_empty() && rust_enum_name(line).is_none() {
        *pending_subcommand = false;
    }
}

fn rust_enum_name(line: &str) -> Option<&str> {
    rust_item_name(line, "enum")
}

fn rust_struct_name(line: &str) -> Option<&str> {
    rust_item_name(line, "struct")
}

fn rust_item_name<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    let mut parts = line.split_whitespace();
    let first = parts.next()?;
    let item_token = if first == "pub" || first.starts_with("pub(") {
        parts.next()?
    } else {
        first
    };
    if item_token != keyword {
        return None;
    }
    parts
        .next()
        .map(|name| name.trim_end_matches('{'))
        .map(|name| name.split('<').next().unwrap_or(name))
        .filter(|name| !name.is_empty())
}

fn rust_enum_variant(line: &str) -> Option<(String, Option<String>)> {
    if line.starts_with('#') || line.starts_with("//") || line.starts_with('}') {
        return None;
    }
    let name = line
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect::<String>();
    if !name
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
    {
        return None;
    }
    let payload_type = line[name.len()..]
        .trim_start()
        .strip_prefix('(')
        .and_then(|rest| rest.split(')').next())
        .and_then(rust_type_name);
    Some((name, payload_type))
}

fn rust_field_type(line: &str) -> Option<String> {
    let type_part = line.split_once(':')?.1;
    rust_type_name(type_part)
}

fn rust_type_name(value: &str) -> Option<String> {
    let value = value.trim();
    let value = value
        .strip_prefix("Option<")
        .and_then(|inner| inner.split('>').next())
        .unwrap_or(value);
    let name = value
        .trim()
        .trim_end_matches(',')
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == ':'))
        .next()?
        .rsplit("::")
        .next()?
        .to_string();
    (!name.is_empty()).then_some(name)
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

fn join_command_prefix(prefix: &str, command: &str) -> String {
    if prefix.is_empty() {
        command.to_string()
    } else {
        format!("{prefix} {command}")
    }
}

fn brace_delta(line: &str) -> isize {
    line.chars().fold(0, |depth, ch| match ch {
        '{' => depth + 1,
        '}' => depth - 1,
        _ => depth,
    })
}
