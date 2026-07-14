// Responsibility: map-proof-wiring-helpers-receipt-fields
use crate::map::json_key_line;
use crate::model::{EvidenceLocation, Project};

pub(crate) fn declared_receipt_fields(rel: &str, text: &str) -> Vec<(String, usize)> {
    if rel.ends_with(".json")
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(text)
        && let Some(object) = value.as_object()
    {
        return object
            .keys()
            .filter(|key| receipt_field_should_track(key))
            .map(|key| (key.clone(), json_key_line(text, key).unwrap_or(1)))
            .collect();
    }
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let trimmed = line.trim();
            let (key, _) = trimmed.split_once(':')?;
            let key = key.trim().trim_matches(['"', '\'', '`', '-']).to_string();
            receipt_field_should_track(&key).then_some((key, index + 1))
        })
        .collect()
}

pub(crate) fn json_string_field(text: &str, key: &str) -> Option<String> {
    let value = serde_json::from_str::<serde_json::Value>(text).ok()?;
    value
        .get(key)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(crate) fn field_line_locations(text: &str, rel: &str, keys: &[&str]) -> Vec<EvidenceLocation> {
    let mut locations = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if keys
            .iter()
            .any(|key| line.contains(&format!("\"{key}\"")) || line.contains(&format!("{key}:")))
        {
            locations.push(EvidenceLocation::line(rel, index + 1, "field"));
        }
    }
    if locations.is_empty() {
        locations.push(EvidenceLocation::path(rel, "artifact"));
    }
    locations
}

fn receipt_field_should_track(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    [
        "status",
        "pass",
        "passed",
        "success",
        "ok",
        "exit_code",
        "exit",
        "schema",
        "schema_version",
        "evidence",
        "controls",
        "control",
        "command",
        "artifact",
        "receipt",
    ]
    .iter()
    .any(|part| lower == *part || lower.contains(part))
}

pub(crate) fn markdown_declared_fields(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    for part in line.split('`').skip(1).step_by(2) {
        if receipt_field_should_track(part) || field_name_looks_structural(part) {
            fields.push(part.to_string());
        }
    }
    let trimmed = line.trim().trim_start_matches('-').trim();
    if let Some((left, _)) = trimmed.split_once(':') {
        let key = left.trim().trim_matches(['`', '"', '\'']);
        if receipt_field_should_track(key) || field_name_looks_structural(key) {
            fields.push(key.to_string());
        }
    }
    fields.sort();
    fields.dedup();
    fields
}

fn field_name_looks_structural(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.chars().any(|ch| matches!(ch, '_' | '-'))
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
}

pub(crate) fn receipt_field_is_execution(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    matches!(lower.as_str(), "exit" | "exit_code" | "exit_status")
}

pub(crate) fn receipt_field_is_schema(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "schema" | "schema_version" | "receipt_version"
    )
}

pub(crate) fn file_has_predicate_language(project: &Project, rel: &str) -> bool {
    let Ok(text) = std::fs::read_to_string(project.root.join(rel)) else {
        return false;
    };
    text_has_predicate_language(&text)
}

pub(crate) fn text_has_predicate_language(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "pass",
        "passed",
        "fail",
        "success",
        "exit_code",
        "assert",
        "expect",
        "validate",
        "predicate",
        "control",
        "required",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}
