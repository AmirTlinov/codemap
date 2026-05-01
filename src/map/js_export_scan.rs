#[derive(Clone, Copy)]
enum JsMapScanState {
    Code,
    LineComment,
    BlockComment,
    Quoted(u8),
    Template,
}

fn js_map_keyword_positions(text: &str, keyword: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    let keyword_bytes = keyword.as_bytes();
    let mut out = Vec::new();
    let mut index = 0;
    let mut state = JsMapScanState::Code;
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
                        std::string::String::from_utf8_lossy(&bytes[..index]).as_ref(),
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
                } else if bytes[index..].starts_with(keyword_bytes)
                    && js_map_keyword_boundary(bytes, index, keyword_bytes.len())
                {
                    out.push(index);
                    index += keyword_bytes.len();
                    continue;
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
    out
}

fn js_map_keyword_at(bytes: &[u8], index: usize, keyword: &[u8]) -> bool {
    bytes
        .get(index..)
        .map(|rest| rest.starts_with(keyword))
        .unwrap_or(false)
        && js_map_keyword_boundary(bytes, index, keyword.len())
}

fn js_map_keyword_boundary(bytes: &[u8], start: usize, len: usize) -> bool {
    let before = start
        .checked_sub(1)
        .and_then(|index| bytes.get(index))
        .copied();
    let after = bytes.get(start + len).copied();
    !before.map(is_identifier_byte).unwrap_or(false)
        && !after.map(is_identifier_byte).unwrap_or(false)
}

fn js_map_skip_ascii_whitespace(text: &str, mut index: usize) -> usize {
    let bytes = text.as_bytes();
    while bytes
        .get(index)
        .map(|byte| byte.is_ascii_whitespace())
        .unwrap_or(false)
    {
        index += 1;
    }
    index
}

fn js_map_skip_trivia(text: &str, mut index: usize) -> usize {
    let bytes = text.as_bytes();
    loop {
        index = js_map_skip_ascii_whitespace(text, index);
        if bytes.get(index) == Some(&b'/') && bytes.get(index + 1) == Some(&b'/') {
            index += 2;
            while bytes.get(index).map(|byte| *byte != b'\n').unwrap_or(false) {
                index += 1;
            }
            continue;
        }
        if bytes.get(index) == Some(&b'/') && bytes.get(index + 1) == Some(&b'*') {
            index += 2;
            while index < bytes.len() {
                if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                    index += 2;
                    break;
                }
                index += 1;
            }
            continue;
        }
        return index;
    }
}

fn local_named_export_bindings_from_statement(
    statement: &str,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut out = BTreeMap::new();
    let Some(start) = statement.find('{') else {
        return out;
    };
    let Some(end) = statement.rfind('}') else {
        return out;
    };
    let inner = js_code_without_strings_and_comments(&statement[start + 1..end]);
    for part in inner.split(',') {
        let part = part.trim();
        if part.is_empty() || part.starts_with("type ") {
            continue;
        }
        let (local, public) = part
            .split_once(" as ")
            .map(|(local, alias)| (local.trim(), alias.trim()))
            .unwrap_or((part, part));
        let Some(local_name) = first_js_identifier(local) else {
            continue;
        };
        if let Some(public_name) = first_js_identifier(public) {
            out.entry(public_name)
                .or_insert_with(BTreeSet::new)
                .insert(local_name);
        }
    }
    out
}

fn js_code_without_strings_and_comments(text: &str) -> String {
    let mut out = String::new();
    let mut in_block_comment = false;
    let mut quote = None;
    for line in text.lines() {
        out.push_str(&js_code_line_without_strings_and_comments(
            line,
            &mut in_block_comment,
            &mut quote,
        ));
        out.push('\n');
    }
    out
}

fn first_js_identifier(value: &str) -> Option<String> {
    let (start, _) = value
        .char_indices()
        .find(|(_, ch)| ch.is_ascii_alphabetic() || *ch == '_' || *ch == '$')?;
    let mut end = start;
    for (idx, ch) in value[start..].char_indices() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '$' {
            end = start + idx + ch.len_utf8();
        } else {
            break;
        }
    }
    Some(value[start..end].to_string())
}

fn file_has_local_value_shadow(file: &FileInfo, local: &str) -> bool {
    file.local_bindings.contains(local) || file.symbols.iter().any(|symbol| symbol.name == local)
}

fn symbol_is_exported(project: &Project, file_rel: &str, symbol_name: &str) -> bool {
    let Some(info) = project.files.get(file_rel) else {
        return false;
    };
    matching_symbols(info, symbol_name)
        .iter()
        .any(|symbol| symbol.exported)
        || default_export_symbol_name(project, file_rel).as_deref() == Some(symbol_name)
}

fn symbol_exported_under_public_name(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
    public_name: &str,
) -> bool {
    (file_has_inline_named_export(project, file_rel, symbol_name) && public_name == symbol_name)
        || local_named_export_bindings(project, file_rel)
            .get(public_name)
            .map(|locals| locals.len() == 1 && locals.contains(symbol_name))
            .unwrap_or(false)
}

fn file_has_named_public_export(project: &Project, file_rel: &str, public_name: &str) -> bool {
    file_has_inline_named_export(project, file_rel, public_name)
        || local_named_export_bindings(project, file_rel)
            .get(public_name)
            .map(|locals| {
                locals.len() == 1
                    && locals
                        .iter()
                        .any(|local| !matching_symbols_for_rel(project, file_rel, local).is_empty())
            })
            .unwrap_or(false)
}

fn file_has_inline_named_export(project: &Project, file_rel: &str, symbol_name: &str) -> bool {
    let Some(info) = project.files.get(file_rel) else {
        return false;
    };
    let text = std::fs::read_to_string(project.root.join(file_rel)).unwrap_or_default();
    matching_symbols(info, symbol_name).iter().any(|symbol| {
        if !symbol.exported {
            return false;
        }
        text.lines()
            .nth(symbol.line_start.saturating_sub(1))
            .map(|line| !line.trim_start().starts_with("export default"))
            .unwrap_or(true)
    })
}

fn matching_symbols_for_rel(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
) -> Vec<crate::model::SymbolInfo> {
    project
        .files
        .get(file_rel)
        .map(|info| matching_symbols(info, symbol_name))
        .unwrap_or_default()
}

fn imported_symbol_binding_matches(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
    imported: &str,
) -> bool {
    if symbol_exported_under_public_name(project, file_rel, symbol_name, imported) {
        return true;
    }
    imported == "default"
        && (symbol_name == "default"
            || default_export_symbol_name(project, file_rel).as_deref() == Some(symbol_name))
}
