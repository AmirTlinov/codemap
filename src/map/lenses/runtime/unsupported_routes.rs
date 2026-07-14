// Responsibility: runtime-lens-unsupported-routes
use crate::map::{code_shape_without_literal_content, is_identifier_char, runtime_code_lines};
use std::collections::BTreeSet;

pub(crate) fn unsupported_framework_route_context(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for (_, line) in runtime_code_lines(text) {
        for local_name in unsupported_framework_route_import_line(&line) {
            out.insert(local_name);
        }
    }
    out
}

fn unsupported_framework_route_import_line(line: &str) -> Vec<String> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("import ")
        || !(trimmed.contains(" from '@nestjs/common'")
            || trimmed.contains(" from \"@nestjs/common\""))
    {
        return Vec::new();
    }
    let Some((_, rest)) = trimmed.split_once('{') else {
        return Vec::new();
    };
    let Some((names, _)) = rest.split_once('}') else {
        return Vec::new();
    };
    names
        .split(',')
        .filter_map(unsupported_framework_route_import_name)
        .collect()
}

fn unsupported_framework_route_import_name(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (imported, local) = if let Some((imported, local)) = trimmed.split_once(" as ") {
        (imported.trim(), local.trim())
    } else {
        (trimmed, trimmed)
    };
    if !unsupported_framework_route_decorator_names().contains(&imported) {
        return None;
    }
    if local.is_empty() || !local.chars().all(is_identifier_char) {
        return None;
    }
    Some(local.to_string())
}

pub(crate) fn unsupported_framework_route_line(
    line: &str,
    imported_decorators: &BTreeSet<String>,
) -> bool {
    if imported_decorators.is_empty() {
        return false;
    }
    let code = code_shape_without_literal_content(line);
    let trimmed = code.trim_start();
    let Some(rest) = trimmed.strip_prefix('@') else {
        return false;
    };
    let decorator = rest
        .chars()
        .take_while(|ch| is_identifier_char(*ch))
        .collect::<String>();
    if decorator.is_empty() || !imported_decorators.contains(&decorator) {
        return false;
    }
    rest[decorator.len()..]
        .chars()
        .next()
        .is_some_and(|ch| ch == '(' || ch.is_whitespace())
}

fn unsupported_framework_route_decorator_names() -> &'static [&'static str] {
    &[
        "Controller",
        "Get",
        "Post",
        "Put",
        "Patch",
        "Delete",
        "Options",
        "Head",
        "All",
    ]
}
