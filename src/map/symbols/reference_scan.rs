// Responsibility: map-symbols-reference-scan
use crate::map::{
    NonJsCodeState, first_js_identifier_reference_line_after_imports, is_identifier_byte,
    non_js_code_line_without_strings_and_comments,
};
use crate::model::{EvidenceLocation, FileInfo, Project};
use std::collections::BTreeSet;

pub(crate) fn file_references_static_expression_after_imports(
    project: &Project,
    file: &FileInfo,
    expression: &str,
) -> bool {
    first_static_expression_reference_line(project, file, expression).is_some()
}

pub(crate) fn static_expression_reference_location(
    project: &Project,
    file: &FileInfo,
    expression: &str,
    kind: &str,
) -> Vec<EvidenceLocation> {
    match first_static_expression_reference_line(project, file, expression) {
        Some(line) if line > 0 => vec![EvidenceLocation::line(&file.rel, line, kind)],
        _ => vec![EvidenceLocation::path(&file.rel, kind)],
    }
}

pub(crate) fn file_referenced_static_expressions(
    project: &Project,
    file: &FileInfo,
    expressions: &BTreeSet<String>,
) -> BTreeSet<String> {
    if expressions.is_empty() {
        return BTreeSet::new();
    }
    if is_js_like(&file.ext) {
        return expressions
            .iter()
            .filter(|expression| {
                file_references_static_expression_after_imports(project, file, expression)
            })
            .cloned()
            .collect();
    }
    let Some(text) = project.read_indexed_text(&file.rel) else {
        return BTreeSet::new();
    };
    let mut remaining = expressions.clone();
    let mut matched = BTreeSet::new();
    let mut state = NonJsCodeState::default();
    let mut skipping_import = false;
    for line in text.lines() {
        let code = non_js_code_line_without_strings_and_comments(line, &file.ext, &mut state);
        let trimmed = code.trim_start();
        if skipping_import {
            if import_statement_ends(trimmed, &file.ext) {
                skipping_import = false;
            }
            continue;
        }
        if import_statement_starts(trimmed, &file.ext) {
            skipping_import = import_statement_continues(trimmed, &file.ext);
            continue;
        }
        let found = remaining
            .iter()
            .filter(|expression| expression_ranges(&code, expression).next().is_some())
            .cloned()
            .collect::<Vec<_>>();
        for expression in found {
            remaining.remove(&expression);
            matched.insert(expression);
        }
        if remaining.is_empty() {
            break;
        }
    }
    matched
}

fn first_static_expression_reference_line(
    project: &Project,
    file: &FileInfo,
    expression: &str,
) -> Option<usize> {
    if is_js_like(&file.ext) && !expression.contains([':', '.']) {
        return first_js_identifier_reference_line_after_imports(project, file, expression);
    }
    let text = project.read_indexed_text(&file.rel)?;
    let mut state = NonJsCodeState::default();
    let mut skipping_import = false;
    for (index, line) in text.lines().enumerate() {
        let code = non_js_code_line_without_strings_and_comments(line, &file.ext, &mut state);
        let trimmed = code.trim_start();
        if skipping_import {
            if import_statement_ends(trimmed, &file.ext) {
                skipping_import = false;
            }
            continue;
        }
        if import_statement_starts(trimmed, &file.ext) {
            skipping_import = import_statement_continues(trimmed, &file.ext);
            continue;
        }
        if expression_ranges(&code, expression).next().is_some() {
            return Some(index + 1);
        }
    }
    None
}

fn import_statement_continues(line: &str, ext: &str) -> bool {
    match ext {
        "rs" => !line.contains(';'),
        "go" => line.contains('(') && !line.contains(')'),
        "py" | "swift" => false,
        _ => !line.contains(';') && !line.contains(" from "),
    }
}

fn import_statement_ends(line: &str, ext: &str) -> bool {
    match ext {
        "go" => line.contains(')'),
        "rs" => line.contains(';'),
        _ => true,
    }
}

fn import_statement_starts(line: &str, ext: &str) -> bool {
    match ext {
        "rs" => {
            line.starts_with("use ")
                || (line.starts_with("pub") && line.find("use ").is_some_and(|index| index < 32))
        }
        "go" => line.starts_with("import ") || line == "import(",
        "py" => line.starts_with("import ") || line.starts_with("from "),
        "swift" => line.starts_with("import "),
        _ => {
            line.starts_with("import ") || (line.starts_with("export ") && line.contains(" from "))
        }
    }
}

fn expression_ranges<'a>(
    line: &'a str,
    expression: &'a str,
) -> impl Iterator<Item = (usize, usize)> + 'a {
    let bytes = line.as_bytes();
    let needle = expression.as_bytes();
    let mut index = 0usize;
    std::iter::from_fn(move || {
        if needle.is_empty() {
            return None;
        }
        while index + needle.len() <= bytes.len() {
            let start = index;
            index += 1;
            if &bytes[start..start + needle.len()] != needle {
                continue;
            }
            let before = start.checked_sub(1).and_then(|offset| bytes.get(offset));
            let after = bytes.get(start + needle.len());
            if before.is_none_or(|byte| !is_identifier_byte(*byte))
                && after.is_none_or(|byte| !is_identifier_byte(*byte))
            {
                return Some((start, start + needle.len()));
            }
        }
        None
    })
}

fn is_js_like(ext: &str) -> bool {
    matches!(
        ext,
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte"
    )
}

#[cfg(test)]
mod tests {
    use super::expression_ranges;

    #[test]
    fn qualified_expression_requires_identifier_boundaries() {
        assert_eq!(
            expression_ranges("map::where_report()", "map::where_report").count(),
            1
        );
        assert_eq!(
            expression_ranges("other_map::where_report()", "map::where_report").count(),
            0
        );
    }
}
