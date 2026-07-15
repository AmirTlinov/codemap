// Responsibility: javascript-declaration-binding-closure-audit

pub(super) fn declaration_binding_occurs(code: &str, query: &str) -> bool {
    for keyword in [
        "function",
        "class",
        "interface",
        "type",
        "enum",
        "namespace",
        "module",
    ] {
        if crate::repo::js_keyword_positions(code, keyword)
            .into_iter()
            .any(|start| declaration_name_after(code, start + keyword.len()) == Some(query))
        {
            return true;
        }
    }
    for keyword in ["const", "let", "var", "using"] {
        for start in crate::repo::js_keyword_positions(code, keyword) {
            let tail = declaration_tail(code, start + keyword.len());
            if crate::repo::js_split_top_level_commas(tail)
                .iter()
                .map(|declarator| binding_before_assignment(declarator))
                .any(|binding| declaration_binding_matches(binding, query))
            {
                return true;
            }
        }
    }
    false
}

fn declaration_binding_matches(binding: &str, query: &str) -> bool {
    let binding = binding.trim();
    if matches!(binding.as_bytes().first(), Some(b'{' | b'[')) {
        let Some(pattern_end) =
            crate::repo::js_balanced_pattern_end(binding, 0).map(|index| index + 1)
        else {
            return exact_identifier_occurs(binding, query);
        };
        return destructuring_binding_matches(&binding[..pattern_end], query);
    }
    declaration_name_after(binding, 0) == Some(query)
}

fn destructuring_binding_matches(pattern: &str, query: &str) -> bool {
    let pattern = pattern.trim();
    let Some(open) = pattern.as_bytes().first().copied() else {
        return false;
    };
    if !matches!(open, b'{' | b'[') || pattern.len() < 2 {
        return simple_pattern_binding_matches(pattern, query);
    }
    let inner = &pattern[1..pattern.len() - 1];
    crate::repo::js_split_top_level_commas(inner)
        .iter()
        .any(|item| {
            let item = item.trim().trim_start_matches("...").trim();
            if open == b'{'
                && let Some(colon) = top_level_delimiter(item, ':')
            {
                return pattern_value_matches(&item[colon + 1..], query);
            }
            pattern_value_matches(item, query)
        })
}

fn pattern_value_matches(value: &str, query: &str) -> bool {
    let value = value.trim().trim_start_matches("...").trim();
    let binding = top_level_delimiter(value, '=')
        .map(|index| &value[..index])
        .unwrap_or(value)
        .trim();
    if matches!(binding.as_bytes().first(), Some(b'{' | b'[')) {
        destructuring_binding_matches(binding, query)
    } else {
        simple_pattern_binding_matches(binding, query)
    }
}

fn simple_pattern_binding_matches(binding: &str, query: &str) -> bool {
    declaration_name_after(binding, 0) == Some(query)
}

fn top_level_delimiter(value: &str, delimiter: char) -> Option<usize> {
    let mut parens = 0usize;
    let mut braces = 0usize;
    let mut brackets = 0usize;
    for (index, ch) in value.char_indices() {
        match ch {
            '(' => parens += 1,
            ')' => parens = parens.saturating_sub(1),
            '{' => braces += 1,
            '}' => braces = braces.saturating_sub(1),
            '[' => brackets += 1,
            ']' => brackets = brackets.saturating_sub(1),
            _ if ch == delimiter && parens == 0 && braces == 0 && brackets == 0 => {
                return Some(index);
            }
            _ => {}
        }
    }
    None
}

pub(super) fn declaration_name_after(code: &str, mut index: usize) -> Option<&str> {
    let bytes = code.as_bytes();
    while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
        index += 1;
    }
    if bytes.get(index) == Some(&b'*') {
        index += 1;
        while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
            index += 1;
        }
    }
    let start = index;
    for (offset, ch) in code[start..].char_indices() {
        if !is_identifier_char(ch) {
            break;
        }
        index = start + offset + ch.len_utf8();
    }
    (index > start).then_some(&code[start..index])
}

fn declaration_tail(code: &str, start: usize) -> &str {
    let mut parens = 0usize;
    let mut braces = 0usize;
    let mut brackets = 0usize;
    for (offset, ch) in code[start..].char_indices() {
        match ch {
            '(' => parens += 1,
            ')' => parens = parens.saturating_sub(1),
            '{' => braces += 1,
            '}' => braces = braces.saturating_sub(1),
            '[' => brackets += 1,
            ']' => brackets = brackets.saturating_sub(1),
            ';' if parens == 0 && braces == 0 && brackets == 0 => {
                return &code[start..start + offset];
            }
            '\n' if parens == 0
                && braces == 0
                && brackets == 0
                && !code[start..start + offset].trim().is_empty()
                && !code[start..start + offset].trim_end().ends_with(',') =>
            {
                return &code[start..start + offset];
            }
            _ => {}
        }
    }
    &code[start..]
}

fn binding_before_assignment(declarator: &str) -> &str {
    let mut parens = 0usize;
    let mut braces = 0usize;
    let mut brackets = 0usize;
    for (index, ch) in declarator.char_indices() {
        match ch {
            '(' => parens += 1,
            ')' => parens = parens.saturating_sub(1),
            '{' => braces += 1,
            '}' => braces = braces.saturating_sub(1),
            '[' => brackets += 1,
            ']' => brackets = brackets.saturating_sub(1),
            '=' if parens == 0 && braces == 0 && brackets == 0 => {
                return declarator[..index].trim();
            }
            _ => {}
        }
    }
    declarator.trim()
}

pub(super) fn exact_identifier_occurs(code: &str, query: &str) -> bool {
    !query.is_empty()
        && code.match_indices(query).any(|(start, value)| {
            let before = code[..start].chars().next_back();
            let after = code[start + value.len()..].chars().next();
            before.is_none_or(|ch| !is_identifier_char(ch))
                && after.is_none_or(|ch| !is_identifier_char(ch))
        })
}

fn is_identifier_char(ch: char) -> bool {
    ch.is_alphanumeric()
        || matches!(ch, '_' | '$' | '\u{200c}' | '\u{200d}')
        || (!ch.is_ascii() && !ch.is_whitespace() && !ch.is_ascii_punctuation())
}

#[cfg(test)]
mod tests {
    use super::declaration_binding_occurs;

    #[test]
    fn declaration_audit_ignores_references_but_finds_later_bindings() {
        assert!(!declaration_binding_occurs(
            "import { target } from      ; target();",
            "target"
        ));
        assert!(declaration_binding_occurs(
            "const first = target(), Needle = 2;",
            "Needle"
        ));
        assert!(!declaration_binding_occurs(
            "const value: Needle = makeValue();",
            "Needle"
        ));
        assert!(!declaration_binding_occurs(
            "const { Needle: local } = object;",
            "Needle"
        ));
        assert!(declaration_binding_occurs(
            "const { Needle, nested: { deepNeedle }, ...rest } = object;",
            "Needle"
        ));
        assert!(declaration_binding_occurs(
            "const { Needle, nested: { deepNeedle }, ...rest } = object;",
            "deepNeedle"
        ));
    }
}
