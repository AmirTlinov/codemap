// Responsibility: kustomization-resource-path-extraction
use std::collections::BTreeSet;

pub(crate) fn extract_kustomization_resources(text: &str) -> BTreeSet<String> {
    let Ok(value) = yaml_serde::from_str::<serde_json::Value>(text) else {
        return BTreeSet::new();
    };
    value
        .get("resources")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.contains("://"))
        .map(|value| {
            if value.starts_with('.') {
                value.to_string()
            } else {
                format!("./{value}")
            }
        })
        .collect()
}
