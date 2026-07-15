// Responsibility: map-symbols-js-identifier-refs
use crate::model::{FileInfo, Project};

pub(crate) fn default_export_symbol_name(project: &Project, file_rel: &str) -> Option<String> {
    let text = project.read_indexed_text(file_rel)?;
    for line in text.lines() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("export default ") else {
            continue;
        };
        let rest = rest.strip_prefix("async ").unwrap_or(rest).trim_start();
        let rest = rest
            .strip_prefix("function ")
            .or_else(|| rest.strip_prefix("class "))
            .unwrap_or(rest);
        let name = rest
            .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'))
            .next()
            .unwrap_or_default();
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }
    None
}

pub(crate) fn first_js_identifier_reference_line_after_imports(
    project: &Project,
    file: &FileInfo,
    name: &str,
) -> Option<usize> {
    let Some(text) = project.read_indexed_text(&file.rel) else {
        return file.references.contains(name).then_some(0);
    };
    let mut skipping_import = false;
    let mut type_brace_depth: Option<usize> = None;
    let mut in_block_comment = false;
    let mut quote = None;
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if skipping_import {
            if trimmed.contains(';') || trimmed.contains(" from ") {
                skipping_import = false;
            }
            continue;
        }
        let code =
            js_code_line_without_strings_and_comments(line, &mut in_block_comment, &mut quote);
        if let Some(depth) = type_brace_depth.as_mut() {
            *depth = js_brace_depth_after_line(*depth, &code);
            if js_type_context_line_is_complete(trimmed, *depth) {
                type_brace_depth = None;
            }
            continue;
        }
        if js_import_or_export_from_line_starts(trimmed) {
            if !js_import_or_export_from_line_is_complete(trimmed) {
                skipping_import = true;
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
        if line_has_value_identifier_reference(&code, name) {
            return Some(line_index + 1);
        }
    }
    None
}

fn js_import_or_export_from_line_starts(trimmed: &str) -> bool {
    trimmed.starts_with("import ")
        || trimmed.starts_with("import type ")
        || (trimmed.starts_with("export ") && trimmed.contains(" from "))
}

fn js_import_or_export_from_line_is_complete(trimmed: &str) -> bool {
    trimmed.contains(';')
        || trimmed.contains(" from ")
        || trimmed.starts_with("import '")
        || trimmed.starts_with("import \"")
}

pub(crate) fn js_type_context_line_starts(trimmed: &str) -> bool {
    trimmed.starts_with("type ")
        || trimmed.starts_with("export type ")
        || trimmed.starts_with("interface ")
        || trimmed.starts_with("export interface ")
}

pub(crate) fn js_type_context_line_is_complete(trimmed: &str, brace_depth: usize) -> bool {
    brace_depth == 0 && (trimmed.contains(';') || trimmed.ends_with('}') || trimmed.ends_with("};"))
}

pub(crate) fn js_brace_depth_after_line(mut depth: usize, code: &str) -> usize {
    for byte in code.bytes() {
        match byte {
            b'{' => depth = depth.saturating_add(1),
            b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth
}

pub(crate) fn line_has_value_identifier_reference(line: &str, name: &str) -> bool {
    line_value_identifier_reference_count(line, name) > 0
}

pub(crate) fn line_value_identifier_reference_count(line: &str, name: &str) -> usize {
    identifier_ranges(line, name)
        .filter(|(start, end)| identifier_occurrence_is_value_evidence(line, *start, *end))
        .count()
}

pub(crate) fn identifier_ranges<'a>(
    line: &'a str,
    name: &'a str,
) -> impl Iterator<Item = (usize, usize)> + 'a {
    let bytes = line.as_bytes();
    let name_bytes = name.as_bytes();
    let mut index = 0;
    std::iter::from_fn(move || {
        if name_bytes.is_empty() {
            return None;
        }
        while index + name_bytes.len() <= bytes.len() {
            let start = index;
            index += 1;
            if &bytes[start..start + name_bytes.len()] != name_bytes {
                continue;
            }
            let before = start.checked_sub(1).and_then(|idx| bytes.get(idx)).copied();
            let after = bytes.get(start + name_bytes.len()).copied();
            if !before.map(is_identifier_byte).unwrap_or(false)
                && !after.map(is_identifier_byte).unwrap_or(false)
            {
                return Some((start, start + name_bytes.len()));
            }
        }
        None
    })
}

fn identifier_occurrence_is_type_only(line: &str, start: usize, end: usize) -> bool {
    let before = &line[..start];
    let after = &line[end..];
    if ["typeof", "as", "implements", "satisfies", "keyof", "infer"]
        .iter()
        .any(|word| previous_word_is(before, word))
    {
        return true;
    }
    if previous_nonspace_byte(before) == Some(b'<') || next_nonspace_byte(after) == Some(b'>') {
        return true;
    }
    js_type_annotation_before_identifier(before)
}

fn identifier_occurrence_is_value_evidence(line: &str, start: usize, end: usize) -> bool {
    if identifier_occurrence_is_type_only(line, start, end) {
        return false;
    }
    let before = &line[..start];
    let after = &line[end..];
    let previous = previous_nonspace_byte(before);
    let next = next_nonspace_byte(after);
    if matches!(previous, Some(b'.' | b'/' | b':' | b'\'' | b'"' | b'`'))
        || matches!(next, Some(b':' | b'/'))
    {
        return false;
    }
    if previous_word_is(before, "return") || previous_word_is(before, "throw") {
        return true;
    }
    matches!(next, Some(b'('))
        || (matches!(previous, Some(b'(' | b',' | b'=' | b'['))
            && (next.is_none()
                || matches!(next, Some(b')' | b',' | b'.' | b';' | b']' | b'}' | b'?'))))
}

pub(crate) fn previous_word_is(before: &str, word: &str) -> bool {
    let bytes = before.as_bytes();
    let mut end = bytes.len();
    while end > 0 && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    if end == 0 {
        return false;
    }
    let mut start = end;
    while start > 0 && is_identifier_byte(bytes[start - 1]) {
        start -= 1;
    }
    std::str::from_utf8(&bytes[start..end]) == Ok(word)
}

pub(crate) fn previous_nonspace_byte(before: &str) -> Option<u8> {
    before
        .bytes()
        .rev()
        .find(|byte| !byte.is_ascii_whitespace())
}

pub(crate) fn next_nonspace_byte(after: &str) -> Option<u8> {
    after.bytes().find(|byte| !byte.is_ascii_whitespace())
}

fn js_type_annotation_before_identifier(before: &str) -> bool {
    let Some(colon) = before.rfind(':') else {
        return false;
    };
    let last_value_separator = before.rfind(['=', '(', ',', '{', '[', ';']).unwrap_or(0);
    colon > last_value_separator
}

pub(crate) fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$')
}

pub(crate) fn js_code_line_without_strings_and_comments(
    line: &str,
    in_block_comment: &mut bool,
    quote: &mut Option<u8>,
) -> String {
    let bytes = line.as_bytes();
    let mut out = String::new();
    let mut index = 0;
    while index < bytes.len() {
        if *in_block_comment {
            if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                *in_block_comment = false;
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }
        if let Some(quote_byte) = *quote {
            if bytes[index] == b'\\' {
                index += 2;
                continue;
            }
            if bytes[index] == quote_byte {
                *quote = None;
            }
            index += 1;
            continue;
        }
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            break;
        }
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            *in_block_comment = true;
            index += 2;
            continue;
        }
        if bytes[index] == b'/'
            && js_regex_literal_can_start(&out)
            && let Some(end) = js_regex_literal_end(bytes, index)
        {
            index = end;
            continue;
        }
        if matches!(bytes[index], b'\'' | b'"' | b'`') {
            *quote = Some(bytes[index]);
            index += 1;
            continue;
        }
        out.push(bytes[index] as char);
        index += 1;
    }
    out
}

pub(crate) fn js_regex_literal_can_start(prefix: &str) -> bool {
    [
        "return", "await", "throw", "yield", "case", "delete", "void", "typeof", "else",
    ]
    .iter()
    .any(|word| previous_word_is(prefix, word))
        || prefix.trim_end().ends_with("=>")
        || previous_nonspace_byte(prefix)
            .map(|byte| {
                matches!(
                    byte,
                    b'(' | b')'
                        | b'='
                        | b':'
                        | b'['
                        | b'{'
                        | b','
                        | b'!'
                        | b'?'
                        | b';'
                        | b'|'
                        | b'&'
                )
            })
            .unwrap_or(true)
}

pub(crate) fn js_regex_literal_end(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'/') || matches!(bytes.get(start + 1), Some(b'/' | b'*') | None) {
        return None;
    }
    let mut index = start + 1;
    let mut in_class = false;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => {
                index = index.saturating_add(2);
                continue;
            }
            b'[' => in_class = true,
            b']' => in_class = false,
            b'/' if !in_class => {
                index += 1;
                while bytes
                    .get(index)
                    .map(|byte| byte.is_ascii_alphabetic())
                    .unwrap_or(false)
                {
                    index += 1;
                }
                return Some(index);
            }
            _ => {}
        }
        index += 1;
    }
    None
}
