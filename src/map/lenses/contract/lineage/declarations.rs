// Responsibility: syntax-owned-contract-type-field-and-operation-declarations
use super::entity_surface;
use crate::model::{Project, Surface};
use std::path::Path;

pub(super) fn contract_declarations(project: &Project, rel: &str, out: &mut Vec<Surface>) {
    let Some(text) = project.read_indexed_text(rel) else {
        return;
    };
    match Path::new(rel).extension().and_then(|ext| ext.to_str()) {
        Some("yaml" | "yml") => yaml_declarations(rel, &text, out),
        Some("graphql" | "gql") => graphql_declarations(rel, &text, out),
        Some("proto") => proto_declarations(rel, &text, out),
        Some("prisma") => prisma_declarations(rel, &text, out),
        Some("json" | "avsc") => json_declarations(rel, &text, out),
        _ => {}
    }
}

fn yaml_declarations(rel: &str, text: &str, out: &mut Vec<Surface>) {
    let mut in_paths = false;
    let mut in_schemas = false;
    let mut current_schema: Option<String> = None;
    let mut in_properties = false;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        let indent = line.len() - line.trim_start().len();
        if indent == 0 && trimmed == "paths:" {
            in_paths = true;
            in_schemas = false;
            continue;
        }
        if in_paths && indent == 0 && !trimmed.is_empty() {
            in_paths = false;
        }
        if in_paths && indent == 2 && trimmed.starts_with('/') && trimmed.ends_with(':') {
            let path = trimmed.trim_end_matches(':');
            push(
                out,
                rel,
                index + 1,
                "contract_path",
                path,
                "openapi_path_declaration",
            );
        }
        if indent == 2 && trimmed == "schemas:" {
            in_schemas = true;
            in_paths = false;
            current_schema = None;
            continue;
        }
        if in_schemas && indent <= 2 && trimmed != "schemas:" && !trimmed.is_empty() {
            in_schemas = false;
        }
        if in_schemas && indent == 4 && trimmed.ends_with(':') {
            let name = trimmed.trim_end_matches(':');
            current_schema = Some(name.to_string());
            in_properties = false;
            push(
                out,
                rel,
                index + 1,
                "schema_type",
                name,
                "openapi_schema_declaration",
            );
        } else if in_schemas && indent == 6 && trimmed == "properties:" {
            in_properties = true;
        } else if in_schemas && in_properties && indent == 8 && trimmed.ends_with(':') {
            if let Some(owner) = current_schema.as_deref() {
                let field = trimmed.trim_end_matches(':');
                push_field(
                    out,
                    rel,
                    index + 1,
                    owner,
                    field,
                    "openapi_field_declaration",
                );
            }
        } else if in_properties && indent <= 6 && !trimmed.is_empty() {
            in_properties = false;
        }
    }
}

fn graphql_declarations(rel: &str, text: &str, out: &mut Vec<Surface>) {
    let mut owner: Option<String> = None;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('}') {
            owner = None;
            continue;
        }
        if let Some((kind, name)) = definition(trimmed, &["type", "input", "interface", "enum"]) {
            push(
                out,
                rel,
                index + 1,
                "schema_type",
                name,
                &format!("graphql_{kind}_declaration"),
            );
            owner = Some(name.to_string());
            continue;
        }
        if let Some((kind, name)) = definition(trimmed, &["scalar", "union"]) {
            push(
                out,
                rel,
                index + 1,
                "schema_type",
                name,
                &format!("graphql_{kind}_declaration"),
            );
            continue;
        }
        if let Some(owner) = owner.as_deref()
            && let Some(field) = trimmed
                .split(['(', ':'])
                .next()
                .filter(|name| identifier(name))
        {
            push_field(
                out,
                rel,
                index + 1,
                owner,
                field,
                "graphql_field_declaration",
            );
        }
    }
}

fn proto_declarations(rel: &str, text: &str, out: &mut Vec<Surface>) {
    let mut message: Option<String> = None;
    let mut service: Option<String> = None;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('}') {
            message = None;
            service = None;
            continue;
        }
        if let Some((kind, name)) = definition(trimmed, &["message", "enum"]) {
            push(
                out,
                rel,
                index + 1,
                "schema_type",
                name,
                &format!("protobuf_{kind}_declaration"),
            );
            message = Some(name.to_string());
            continue;
        }
        if let Some((_, name)) = definition(trimmed, &["service"]) {
            push(
                out,
                rel,
                index + 1,
                "contract_service",
                name,
                "protobuf_service_declaration",
            );
            service = Some(name.to_string());
            continue;
        }
        if let Some(owner) = service.as_deref()
            && let Some(name) = trimmed
                .strip_prefix("rpc ")
                .and_then(|tail| tail.split('(').next())
                .map(str::trim)
            && identifier(name)
        {
            push_field(out, rel, index + 1, owner, name, "protobuf_rpc_declaration");
            continue;
        }
        if let Some(owner) = message.as_deref()
            && let Some(left) = trimmed.split('=').next().filter(|_| trimmed.contains('='))
            && let Some(name) = left
                .split_whitespace()
                .last()
                .filter(|name| identifier(name))
        {
            push_field(
                out,
                rel,
                index + 1,
                owner,
                name,
                "protobuf_field_declaration",
            );
        }
    }
}

fn prisma_declarations(rel: &str, text: &str, out: &mut Vec<Surface>) {
    let mut owner: Option<String> = None;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('}') {
            owner = None;
            continue;
        }
        if let Some((kind, name)) = definition(trimmed, &["model", "type", "enum"]) {
            push(
                out,
                rel,
                index + 1,
                "schema_type",
                name,
                &format!("prisma_{kind}_declaration"),
            );
            owner = Some(name.to_string());
            continue;
        }
        if let Some(owner) = owner.as_deref()
            && let Some(name) = trimmed.split_whitespace().next()
            && identifier(name)
            && !name.starts_with('@')
        {
            push_field(out, rel, index + 1, owner, name, "prisma_field_declaration");
        }
    }
}

fn json_declarations(rel: &str, text: &str, out: &mut Vec<Surface>) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return;
    };
    if let Some(paths) = value.get("paths").and_then(|value| value.as_object()) {
        for path in paths.keys() {
            push(
                out,
                rel,
                first_line(text, path),
                "contract_path",
                path,
                "openapi_path_declaration",
            );
        }
    }
    for definitions in ["$defs", "definitions"] {
        if let Some(types) = value.get(definitions).and_then(|value| value.as_object()) {
            for (name, schema) in types {
                push(
                    out,
                    rel,
                    first_line(text, name),
                    "schema_type",
                    name,
                    "json_schema_declaration",
                );
                json_fields(rel, text, name, schema, out);
            }
        }
    }
    if value.get("type").and_then(|value| value.as_str()) == Some("record")
        && let Some(name) = value.get("name").and_then(|value| value.as_str())
    {
        push(
            out,
            rel,
            first_line(text, name),
            "schema_type",
            name,
            "avro_record_declaration",
        );
        if let Some(fields) = value.get("fields").and_then(|value| value.as_array()) {
            for field in fields
                .iter()
                .filter_map(|field| field.get("name")?.as_str())
            {
                push_field(
                    out,
                    rel,
                    first_line(text, field),
                    name,
                    field,
                    "avro_field_declaration",
                );
            }
        }
    }
}

fn json_fields(
    rel: &str,
    text: &str,
    owner: &str,
    schema: &serde_json::Value,
    out: &mut Vec<Surface>,
) {
    if let Some(properties) = schema.get("properties").and_then(|value| value.as_object()) {
        for field in properties.keys() {
            push_field(
                out,
                rel,
                first_line(text, field),
                owner,
                field,
                "json_schema_field",
            );
        }
    }
}

fn definition<'a>(line: &'a str, kinds: &[&'a str]) -> Option<(&'a str, &'a str)> {
    kinds.iter().find_map(|kind| {
        let name = line
            .strip_prefix(&format!("{kind} "))?
            .split_whitespace()
            .next()?;
        let name = name.trim_end_matches(['{', '=']);
        identifier(name).then_some((*kind, name))
    })
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn push(out: &mut Vec<Surface>, rel: &str, line: usize, kind: &str, name: &str, evidence: &str) {
    out.push(entity_surface(
        format!("{kind}:{name}"),
        kind,
        rel,
        line,
        evidence,
    ));
}

fn push_field(
    out: &mut Vec<Surface>,
    rel: &str,
    line: usize,
    owner: &str,
    field: &str,
    evidence: &str,
) {
    out.push(entity_surface(
        format!("field:{owner}.{field}"),
        "field",
        rel,
        line,
        evidence,
    ));
}

fn first_line(text: &str, needle: &str) -> usize {
    text.lines()
        .position(|line| line.contains(&format!("\"{needle}\"")))
        .map(|index| index + 1)
        .unwrap_or(1)
}
