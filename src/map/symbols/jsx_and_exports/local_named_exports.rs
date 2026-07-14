// Responsibility: map-symbols-local-named-exports
use crate::map::{
    JsMapScanState, js_map_keyword_at, js_map_keyword_positions, js_map_skip_ascii_whitespace,
    js_map_skip_trivia, js_regex_literal_can_start, js_regex_literal_end,
    local_named_export_bindings_from_statement,
};
use crate::model::Project;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

pub(crate) fn local_named_export_bindings(
    project: &Project,
    file_rel: &str,
) -> BTreeMap<String, BTreeSet<String>> {
    let Ok(text) = std::fs::read_to_string(project.root.join(file_rel)) else {
        return BTreeMap::new();
    };
    let mut out = BTreeMap::new();
    for statement in local_named_export_statement_slices(&text) {
        for (public_name, local_names) in local_named_export_bindings_from_statement(statement) {
            out.entry(public_name)
                .or_insert_with(BTreeSet::new)
                .extend(local_names);
        }
    }
    out
}

pub(crate) fn local_named_export_statement_slices(text: &str) -> Vec<&str> {
    js_map_keyword_positions(text, "export")
        .into_iter()
        .filter_map(|start| js_local_named_export_statement_slice(text, start))
        .collect()
}

fn js_local_named_export_statement_slice(text: &str, start: usize) -> Option<&str> {
    let bytes = text.as_bytes();
    let mut index = js_map_skip_ascii_whitespace(text, start + "export".len());
    if js_map_keyword_at(bytes, index, b"type") {
        return None;
    }
    if bytes.get(index) != Some(&b'{') {
        return None;
    }

    let mut state = JsMapScanState::Code;
    let mut brace_depth = 0usize;
    while index < bytes.len() {
        match state {
            JsMapScanState::Code => {
                if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
                    state = JsMapScanState::LineComment;
                    index += 2;
                    continue;
                }
                if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
                    state = JsMapScanState::BlockComment;
                    index += 2;
                    continue;
                }
                if bytes[index] == b'/'
                    && js_regex_literal_can_start(
                        std::string::String::from_utf8_lossy(&bytes[start..index]).as_ref(),
                    )
                    && let Some(end) = js_regex_literal_end(bytes, index)
                {
                    index = end;
                    continue;
                }
                if matches!(bytes[index], b'\'' | b'"') {
                    state = JsMapScanState::Quoted(bytes[index]);
                } else if bytes[index] == b'`' {
                    state = JsMapScanState::Template;
                } else if bytes[index] == b'{' {
                    brace_depth += 1;
                } else if bytes[index] == b'}' {
                    brace_depth = brace_depth.saturating_sub(1);
                    if brace_depth == 0 {
                        let after_brace = js_map_skip_trivia(text, index + 1);
                        if js_map_keyword_at(bytes, after_brace, b"from") {
                            return None;
                        }
                        let end = if bytes.get(after_brace) == Some(&b';') {
                            after_brace + 1
                        } else {
                            index + 1
                        };
                        return Some(&text[start..end]);
                    }
                }
            }
            JsMapScanState::LineComment => {
                if bytes[index] == b'\n' {
                    state = JsMapScanState::Code;
                }
            }
            JsMapScanState::BlockComment => {
                if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                    state = JsMapScanState::Code;
                    index += 2;
                    continue;
                }
            }
            JsMapScanState::Quoted(quote) => {
                if bytes[index] == b'\\' {
                    index = index.saturating_add(2);
                    continue;
                }
                if bytes[index] == quote {
                    state = JsMapScanState::Code;
                }
            }
            JsMapScanState::Template => {
                if bytes[index] == b'\\' {
                    index = index.saturating_add(2);
                    continue;
                }
                if bytes[index] == b'`' {
                    state = JsMapScanState::Code;
                }
            }
        }
        index += 1;
    }
    None
}
