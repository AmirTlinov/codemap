// Responsibility: map-symbols-body-refs
mod local_bindings;
mod non_js_code_lines;

pub(crate) use local_bindings::*;
pub(crate) use non_js_code_lines::*;

use crate::map::{
    default_export_symbol_name, js_brace_depth_after_line,
    js_code_line_without_strings_and_comments, js_type_context_line_is_complete,
    js_type_context_line_starts, line_has_jsx_tag_identifier_reference,
    line_has_value_identifier_reference, matching_symbols,
};
use crate::model::{FileInfo, Project};

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
        if line_has_value_identifier_reference(&code, local) {
            return true;
        }
    }
    false
}
