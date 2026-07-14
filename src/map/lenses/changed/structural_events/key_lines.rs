// Responsibility: changed-key-line-parsing
use crate::map::{ci_run_steps, json_key_line};
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) fn env_key_lines(text: &str) -> BTreeMap<String, usize> {
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| env_key_from_line(line).map(|key| (key, index + 1)))
        .collect()
}

fn env_key_from_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let (name, _) = trimmed.split_once('=')?;
    let name = name.trim().trim_start_matches("export ").trim();
    (!name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_'))
    .then(|| name.to_string())
}

pub(crate) fn config_key_lines(rel: &str, text: &str) -> BTreeMap<String, usize> {
    let ext = Path::new(rel)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| config_key_from_line(ext, line).map(|key| (key, index + 1)))
        .collect()
}

fn config_key_from_line(ext: &str, line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.starts_with("//")
        || trimmed.starts_with('{')
        || trimmed.starts_with('}')
        || trimmed.starts_with('[')
    {
        return None;
    }
    match ext {
        "json" => json_object_key_from_line(trimmed),
        "toml" => toml_key_from_line(trimmed),
        "yaml" | "yml" => yaml_key_from_line(trimmed),
        _ => None,
    }
}

fn json_object_key_from_line(line: &str) -> Option<String> {
    let rest = line.strip_prefix('"')?;
    let end = rest.find('"')?;
    let after = rest[end + 1..].trim_start();
    after
        .starts_with(':')
        .then(|| rest[..end].to_string())
        .filter(|key| !key.is_empty())
}

fn toml_key_from_line(line: &str) -> Option<String> {
    let (key, _) = line.split_once('=')?;
    let key = key.trim().trim_matches('"').trim_matches('\'');
    (!key.is_empty()
        && key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.')))
    .then(|| key.to_string())
}

fn yaml_key_from_line(line: &str) -> Option<String> {
    let (key, _) = line.split_once(':')?;
    let key = key.trim().trim_matches('"').trim_matches('\'');
    (!key.is_empty()
        && key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.')))
    .then(|| key.to_string())
}

pub(crate) fn package_script_lines_from_text(text: &str) -> BTreeMap<String, usize> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return BTreeMap::new();
    };
    let Some(scripts) = value.get("scripts").and_then(|scripts| scripts.as_object()) else {
        return BTreeMap::new();
    };
    scripts
        .keys()
        .map(|script| (script.clone(), json_key_line(text, script).unwrap_or(1)))
        .collect()
}

pub(crate) fn schema_decl_lines(text: &str) -> BTreeMap<String, usize> {
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| schema_decl_from_line(line).map(|decl| (decl, index + 1)))
        .collect()
}

fn schema_decl_from_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with("//") || trimmed.starts_with('#') {
        return None;
    }
    let mut parts = trimmed.split_whitespace();
    let kind = parts.next()?;
    if !matches!(
        kind,
        "model" | "enum" | "type" | "view" | "datasource" | "generator" | "table"
    ) {
        return None;
    }
    let name = parts.next()?.trim_matches('{');
    (!name.is_empty()).then(|| format!("{kind}:{name}"))
}

pub(crate) fn ci_run_lines(text: &str) -> BTreeMap<String, usize> {
    ci_run_steps(text)
        .into_iter()
        .map(|step| (step.command, step.line))
        .collect()
}
