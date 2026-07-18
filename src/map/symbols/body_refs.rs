// Responsibility: map-symbols-body-refs
mod local_bindings;
mod non_js_code_lines;

pub(crate) use local_bindings::*;
pub(crate) use non_js_code_lines::*;

use crate::map::{
    default_export_symbol_name, js_brace_depth_after_line,
    js_code_line_without_strings_and_comments, js_type_context_line_is_complete,
    js_type_context_line_starts, line_has_jsx_tag_identifier_reference,
    line_has_type_identifier_reference, line_has_value_identifier_reference, matching_symbols,
};
use crate::model::{FileInfo, Project};
use std::collections::BTreeMap;

pub(crate) fn symbol_body_text(
    project: &Project,
    info: &FileInfo,
    symbol_name: &str,
) -> Option<String> {
    let symbols = matching_symbols(info, symbol_name);
    if symbols.is_empty() {
        return None;
    }
    let line_start = symbols.iter().map(|symbol| symbol.line_start).min()?;
    let line_end = symbols
        .iter()
        .map(|symbol| symbol.line_end)
        .max()
        .unwrap_or(line_start);
    let text = project.read_indexed_text(&info.rel)?;
    Some(
        text.lines()
            .skip(line_start.saturating_sub(1))
            .take(line_end.saturating_sub(line_start).saturating_add(1))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

pub(crate) fn symbol_public_type_text(
    project: &Project,
    info: &FileInfo,
    symbol_name: &str,
) -> Option<String> {
    let symbols = matching_symbols(info, symbol_name);
    let body = symbol_body_text(project, info, symbol_name)?;
    if !matches!(
        info.ext.as_str(),
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs"
    ) || symbols.iter().any(|symbol| {
        matches!(
            symbol.kind.as_str(),
            "class" | "interface" | "type" | "enum"
        )
    }) {
        return Some(body);
    }
    let kind = symbols.first().map(|symbol| symbol.kind.as_str())?;
    if matches!(kind, "const" | "variable" | "component" | "hook")
        && let Some(arrow) = body.find("=>")
    {
        return Some(body[..arrow].to_string());
    }
    if matches!(kind, "const" | "variable")
        && let Some(assign) = body.find('=')
    {
        return Some(body[..assign].to_string());
    }
    Some(js_function_signature(&body))
}

fn js_function_signature(body: &str) -> String {
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    for (index, ch) in body.char_indices() {
        match ch {
            '(' => paren_depth = paren_depth.saturating_add(1),
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth = bracket_depth.saturating_add(1),
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' if paren_depth == 0 && bracket_depth == 0 => return body[..index].to_string(),
            _ => {}
        }
    }
    body.to_string()
}

pub(crate) fn symbol_body_references_imported_type(body: &str, local: &str, ext: &str) -> bool {
    if !matches!(
        ext,
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte"
    ) {
        return false;
    }
    let mut in_block_comment = false;
    let mut quote = None;
    body.lines().any(|line| {
        let code =
            js_code_line_without_strings_and_comments(line, &mut in_block_comment, &mut quote);
        line_has_type_identifier_reference(&code, local)
            && (!matches!(ext, "tsx" | "jsx" | "vue" | "svelte")
                || !line_has_jsx_tag_identifier_reference(&code, local))
    })
}

pub(crate) fn rust_qualified_symbol_body_references(
    project: &Project,
    info: &FileInfo,
    symbol_name: &str,
    namespace: &str,
) -> Vec<(String, usize)> {
    if info.ext != "rs" || namespace.is_empty() {
        return Vec::new();
    }
    let symbols = matching_symbols(info, symbol_name);
    let Some(line_start) = symbols.iter().map(|symbol| symbol.line_start).min() else {
        return Vec::new();
    };
    let line_end = symbols
        .iter()
        .map(|symbol| symbol.line_end)
        .max()
        .unwrap_or(line_start);
    let Some(text) = project.read_indexed_text(&info.rel) else {
        return Vec::new();
    };
    let mut found = BTreeMap::new();
    let mut state = NonJsCodeState::default();
    for (offset, line) in text
        .lines()
        .skip(line_start.saturating_sub(1))
        .take(line_end.saturating_sub(line_start).saturating_add(1))
        .enumerate()
    {
        let code = non_js_code_line_without_strings_and_comments(line, "rs", &mut state);
        for member in rust_qualified_members(&code, namespace) {
            found.entry(member).or_insert(line_start + offset);
        }
    }
    found.into_iter().collect()
}

fn rust_qualified_members(line: &str, namespace: &str) -> Vec<String> {
    let needle = format!("{namespace}::");
    let bytes = line.as_bytes();
    let mut offset = 0;
    let mut members = Vec::new();
    while let Some(found) = line[offset..].find(&needle) {
        let start = offset + found;
        let member_start = start + needle.len();
        offset = member_start;
        if start > 0 && crate::map::is_identifier_byte(bytes[start - 1]) {
            continue;
        }
        let mut end = member_start;
        while bytes
            .get(end)
            .is_some_and(|byte| crate::map::is_identifier_byte(*byte))
        {
            end += 1;
        }
        if end == member_start || bytes.get(end..end + 2) == Some(b"::") {
            continue;
        }
        members.push(line[member_start..end].to_string());
        offset = end;
    }
    members
}

pub(crate) fn imported_binding_target_symbol_name(
    project: &Project,
    target_rel: &str,
    imported: &str,
) -> Option<String> {
    let target = project.files.get(target_rel)?;
    if imported == "default" {
        let name = default_export_symbol_name(project, target_rel)?;
        return (!matching_symbols(target, &name).is_empty()).then_some(name);
    }
    (!matching_symbols(target, imported).is_empty()).then(|| imported.to_string())
}

pub(crate) fn symbol_body_references_imported_local(body: &str, local: &str, ext: &str) -> bool {
    if !matches!(
        ext,
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte"
    ) {
        return non_js_symbol_body_references_local(body, local, ext);
    }
    let mut in_block_comment = false;
    let mut quote = None;
    let mut type_brace_depth: Option<usize> = None;
    let jsx_capable = matches!(ext, "tsx" | "jsx" | "vue" | "svelte");
    for line in body.lines() {
        let trimmed = line.trim_start();
        let code =
            js_code_line_without_strings_and_comments(line, &mut in_block_comment, &mut quote);
        if let Some(depth) = type_brace_depth.as_mut() {
            *depth = js_brace_depth_after_line(*depth, &code);
            if js_type_context_line_is_complete(trimmed, *depth) {
                type_brace_depth = None;
            }
            continue;
        }
        if js_type_context_line_starts(trimmed) {
            let depth = js_brace_depth_after_line(0, &code);
            if !js_type_context_line_is_complete(trimmed, depth) {
                type_brace_depth = Some(depth);
            }
            continue;
        }
        if line_has_value_identifier_reference(&code, local)
            || (jsx_capable && line_has_jsx_tag_identifier_reference(&code, local))
        {
            return true;
        }
    }
    false
}

fn non_js_symbol_body_references_local(body: &str, local: &str, ext: &str) -> bool {
    let mut state = NonJsCodeState::default();
    for line in body.lines() {
        let code = non_js_code_line_without_strings_and_comments(line, ext, &mut state);
        if non_js_identifier_call(&code, local) || line_has_value_identifier_reference(&code, local)
        {
            return true;
        }
    }
    false
}

fn non_js_identifier_call(line: &str, name: &str) -> bool {
    crate::map::identifier_ranges(line, name)
        .any(|(_, end)| line[end..].bytes().find(|byte| !byte.is_ascii_whitespace()) == Some(b'('))
}
