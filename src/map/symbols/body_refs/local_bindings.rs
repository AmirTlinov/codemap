// Responsibility: map-symbols-local-bindings
use crate::map::{
    identifier_ranges, js_code_line_without_strings_and_comments, next_nonspace_byte,
    previous_nonspace_byte, previous_word_is,
};
use crate::repo;

pub(crate) fn symbol_body_declares_js_local_binding(body: &str, name: &str, ext: &str) -> bool {
    if repo::extract_local_bindings(body, ext).contains(name) {
        return true;
    }
    let mut in_block_comment = false;
    let mut quote = None;
    for line in body.lines() {
        let code =
            js_code_line_without_strings_and_comments(line, &mut in_block_comment, &mut quote);
        if js_line_declares_local_binding(&code, name) {
            return true;
        }
    }
    false
}

fn js_line_declares_local_binding(line: &str, name: &str) -> bool {
    identifier_ranges(line, name).any(|(start, end)| {
        let before = &line[..start];
        let after = &line[end..];
        previous_word_is(before, "const")
            || previous_word_is(before, "let")
            || previous_word_is(before, "var")
            || previous_word_is(before, "function")
            || previous_word_is(before, "class")
            || identifier_occurrence_is_declaration_param(line, start, end)
            || ((previous_nonspace_byte(before) == Some(b'{')
                || previous_nonspace_byte(before) == Some(b'['))
                && next_nonspace_byte(after)
                    .map(|byte| matches!(byte, b'}' | b':' | b',' | b'='))
                    .unwrap_or(false)
                && identifier_occurrence_is_binding_pattern(line, start))
    })
}

fn identifier_occurrence_is_declaration_param(line: &str, start: usize, end: usize) -> bool {
    let before = &line[..start];
    let after = &line[end..];
    if after.trim_start().starts_with("=>") {
        return true;
    }
    let Some(open) = before.rfind('(') else {
        return false;
    };
    let Some(close_after) = after.find(')') else {
        return false;
    };
    let before_open = before[..open].trim_end();
    let tail = after[close_after + 1..].trim_start();
    if before_open.contains("function") || before_open.ends_with("catch") {
        return true;
    }
    if tail.starts_with("=>") {
        return true;
    }
    if tail.starts_with('{') {
        let name_before_params = before_open
            .rsplit(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'))
            .find(|part| !part.is_empty())
            .unwrap_or_default();
        return !matches!(
            name_before_params,
            "if" | "for" | "while" | "switch" | "catch" | "with"
        );
    }
    false
}

fn identifier_occurrence_is_binding_pattern(line: &str, start: usize) -> bool {
    let before = &line[..start];
    let opener = before.rfind(['{', '[']).unwrap_or(0);
    let before_opener = before[..opener].trim_end();
    previous_word_is(before_opener, "const")
        || previous_word_is(before_opener, "let")
        || previous_word_is(before_opener, "var")
        || before_opener.ends_with('(')
        || before_opener.ends_with(',')
}
