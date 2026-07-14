// Responsibility: repo-packages-cargo-toml
use crate::repo::{unique_pairs, unique_triples};
use std::collections::BTreeMap;

pub(crate) fn cargo_package_name(text: &str) -> Option<String> {
    parse_toml_value(text)?
        .get("package")?
        .get("name")?
        .as_str()
        .map(str::to_string)
        .filter(|s| !s.is_empty())
}

pub(crate) fn cargo_path_dependencies(text: &str) -> Vec<(String, String, String)> {
    let Some(value) = parse_toml_value(text) else {
        return Vec::new();
    };
    let mut deps = Vec::new();
    for (table, kind) in cargo_dependency_tables(&value) {
        deps.extend(cargo_table_path_dependencies(table, kind));
    }
    unique_triples(deps)
}

pub(crate) fn cargo_workspace_path_dependencies(text: &str) -> BTreeMap<String, String> {
    let Some(value) = parse_toml_value(text) else {
        return BTreeMap::new();
    };
    let mut deps = BTreeMap::new();
    if let Some(table) = value
        .get("workspace")
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(toml::Value::as_table)
    {
        for (name, dependency) in table {
            if let Some(path) = toml_path_field(dependency) {
                deps.insert(name.to_string(), path);
            }
        }
    }
    deps
}

pub(crate) fn cargo_workspace_dependency_names(text: &str) -> Vec<(String, String)> {
    let Some(value) = parse_toml_value(text) else {
        return Vec::new();
    };
    let mut deps = Vec::new();
    for (table, kind) in cargo_dependency_tables(&value) {
        for (name, dependency) in table {
            if toml_workspace_field(dependency) == Some(true) {
                deps.push((name.to_string(), kind.to_string()));
            }
        }
    }
    unique_pairs(deps)
}

pub(crate) fn cargo_workspace_declared(text: &str) -> bool {
    parse_toml_value(text).is_some_and(|value| value.get("workspace").is_some())
}

pub(crate) fn cargo_workspace_array_values(text: &str, key: &str) -> Vec<String> {
    parse_toml_value(text)
        .and_then(|value| value.get("workspace").cloned())
        .and_then(|workspace| workspace.get(key).cloned())
        .and_then(|value| toml_string_array(&value))
        .unwrap_or_default()
}

pub(crate) fn parse_toml_value(text: &str) -> Option<toml::Value> {
    toml::from_str::<toml::Value>(text).ok()
}

fn cargo_dependency_tables(value: &toml::Value) -> Vec<(&toml::Table, &'static str)> {
    let mut tables = Vec::new();
    collect_cargo_dependency_tables(value, &mut tables);
    if let Some(targets) = value.get("target").and_then(toml::Value::as_table) {
        for target in targets.values() {
            collect_cargo_dependency_tables(target, &mut tables);
        }
    }
    tables
}

fn collect_cargo_dependency_tables<'a>(
    value: &'a toml::Value,
    out: &mut Vec<(&'a toml::Table, &'static str)>,
) {
    for (section, kind) in [
        ("dependencies", "runtime"),
        ("dev-dependencies", "dev"),
        ("build-dependencies", "build"),
    ] {
        if let Some(table) = value.get(section).and_then(toml::Value::as_table) {
            out.push((table, kind));
        }
    }
}

fn cargo_table_path_dependencies(table: &toml::Table, kind: &str) -> Vec<(String, String, String)> {
    table
        .iter()
        .filter_map(|(name, dependency)| {
            toml_path_field(dependency).map(|path| (name.to_string(), path, kind.to_string()))
        })
        .collect()
}

pub(crate) fn toml_path_field(value: &toml::Value) -> Option<String> {
    value
        .get("path")
        .and_then(toml::Value::as_str)
        .map(str::to_string)
        .filter(|path| !path.is_empty())
}

fn toml_workspace_field(value: &toml::Value) -> Option<bool> {
    value.get("workspace").and_then(toml::Value::as_bool)
}

pub(crate) fn toml_string_array(value: &toml::Value) -> Option<Vec<String>> {
    Some(
        value
            .as_array()?
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .filter(|item| !item.is_empty())
            .collect(),
    )
}
