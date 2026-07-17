// Responsibility: repo-symbol-ranges
mod language_starts;

pub(crate) use language_starts::*;

use crate::model::SymbolInfo;
use crate::repo::{SymbolStart, previous_nonspace_byte};

pub(crate) fn symbols_with_ranges(
    mut starts: Vec<SymbolStart>,
    text: &str,
    ext: &str,
) -> Vec<SymbolInfo> {
    starts.sort_by(|a, b| {
        a.line_start
            .cmp(&b.line_start)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.kind.cmp(&b.kind))
    });
    let lines = text.lines().collect::<Vec<_>>();
    let text_line_count = lines.len();
    starts
        .iter()
        .enumerate()
        .map(|(idx, symbol)| {
            let fallback_end = starts
                .iter()
                .skip(idx + 1)
                .find(|next| next.indent <= symbol.indent)
                .and_then(|next| next.line_start.checked_sub(1))
                .unwrap_or(text_line_count)
                .max(symbol.line_start);
            SymbolInfo {
                name: symbol.name.clone(),
                kind: symbol.kind.clone(),
                exported: symbol.exported,
                line_start: symbol.line_start,
                line_end: symbol_end(&lines, ext, symbol.line_start, fallback_end),
            }
        })
        .collect()
}

fn symbol_end(lines: &[&str], ext: &str, line_start: usize, fallback_end: usize) -> usize {
    match ext {
        "py" => python_symbol_end(lines, line_start, fallback_end),
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte" => {
            javascript_symbol_end(lines, line_start, fallback_end).unwrap_or(line_start)
        }
        "rs" | "go" | "swift" => {
            brace_symbol_end(lines, line_start, fallback_end).unwrap_or(line_start)
        }
        _ => fallback_end,
    }
}

fn javascript_symbol_end(lines: &[&str], line_start: usize, scan_end: usize) -> Option<usize> {
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut body_depth: Option<isize> = None;
    for (idx, line) in lines
        .iter()
        .enumerate()
        .skip(line_start.saturating_sub(1))
        .take(scan_end.saturating_sub(line_start).saturating_add(1))
    {
        for (byte_idx, ch) in line.char_indices() {
            if let Some(depth) = body_depth.as_mut() {
                match ch {
                    '{' => *depth += 1,
                    '}' => {
                        *depth -= 1;
                        if *depth <= 0 {
                            return Some(idx + 1);
                        }
                    }
                    _ => {}
                }
                continue;
            }
            match ch {
                '(' => paren_depth = paren_depth.saturating_add(1),
                ')' => paren_depth = paren_depth.saturating_sub(1),
                '[' => bracket_depth = bracket_depth.saturating_add(1),
                ']' => bracket_depth = bracket_depth.saturating_sub(1),
                '{' if paren_depth == 0
                    && bracket_depth == 0
                    && javascript_body_open_context(line, byte_idx) =>
                {
                    body_depth = Some(1);
                }
                _ => {}
            }
        }
        if body_depth.is_none() && paren_depth == 0 && bracket_depth == 0 {
            let trimmed = line.trim_end();
            if trimmed.ends_with(';')
                || trimmed.ends_with("=> null")
                || trimmed.ends_with("=> undefined")
            {
                return Some(idx + 1);
            }
        }
    }
    body_depth.map(|_| scan_end)
}

fn javascript_body_open_context(line: &str, byte_idx: usize) -> bool {
    let before = &line[..byte_idx];
    !matches!(previous_nonspace_byte(before), Some(b':' | b'?'))
}

fn brace_symbol_end(lines: &[&str], line_start: usize, scan_end: usize) -> Option<usize> {
    let mut depth: isize = 0;
    let mut saw_open = false;
    for (idx, line) in lines
        .iter()
        .enumerate()
        .skip(line_start.saturating_sub(1))
        .take(scan_end.saturating_sub(line_start).saturating_add(1))
    {
        if !saw_open && line.trim_end().ends_with(';') {
            return Some(idx + 1);
        }
        for ch in line.chars() {
            match ch {
                '{' => {
                    saw_open = true;
                    depth += 1;
                }
                '}' if saw_open => {
                    depth -= 1;
                    if depth <= 0 {
                        return Some(idx + 1);
                    }
                }
                _ => {}
            }
        }
        if !saw_open {
            let trimmed = line.trim();
            if trimmed.ends_with("=> null") || trimmed.ends_with("=> undefined") {
                return Some(idx + 1);
            }
        }
    }
    None
}

fn python_symbol_end(lines: &[&str], line_start: usize, fallback_end: usize) -> usize {
    let Some(start_line) = lines.get(line_start.saturating_sub(1)) else {
        return fallback_end;
    };
    let base_indent = leading_spaces(start_line);
    let header_end = python_header_end(lines, line_start, fallback_end);
    let mut last_non_blank = header_end;
    for (idx, line) in lines.iter().enumerate().skip(header_end) {
        if line.trim().is_empty() {
            continue;
        }
        let line_no = idx + 1;
        let indent = leading_spaces(line);
        if indent <= base_indent {
            return last_non_blank.max(line_start);
        }
        last_non_blank = line_no;
    }
    last_non_blank.max(line_start)
}

fn python_header_end(lines: &[&str], line_start: usize, fallback_end: usize) -> usize {
    let mut depth = 0isize;
    for (idx, line) in lines
        .iter()
        .enumerate()
        .skip(line_start.saturating_sub(1))
        .take(fallback_end.saturating_sub(line_start).saturating_add(1))
    {
        let code = crate::repo::code_without_comments_or_strings(line, "py");
        for ch in code.chars() {
            match ch {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        if depth == 0 && code.trim_end().ends_with(':') {
            return idx + 1;
        }
    }
    line_start
}

pub(crate) fn leading_spaces(line: &str) -> usize {
    line.chars().take_while(|ch| *ch == ' ').count()
}
