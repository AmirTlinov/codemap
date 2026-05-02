fn symbols_with_ranges(mut starts: Vec<SymbolStart>, text: &str, ext: &str) -> Vec<SymbolInfo> {
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
    let mut last_non_blank = line_start;
    for (idx, line) in lines.iter().enumerate().skip(line_start) {
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

fn leading_spaces(line: &str) -> usize {
    line.chars().take_while(|ch| *ch == ' ').count()
}

fn extract_js_symbols(text: &str) -> Vec<SymbolStart> {
    let mut symbols = Vec::new();
    let mut import_export_block_depth = 0usize;
    for (idx, line) in text.lines().enumerate() {
        if is_noise_line(line, "//") {
            continue;
        }
        if js_import_export_block_line(line, &mut import_export_block_depth) {
            continue;
        }
        let line_start = idx + 1;
        if let Some(cap) = js_default_symbol_re().captures(line) {
            let raw_kind = cap.name("kind").map(|m| m.as_str()).unwrap_or("default");
            let name = cap
                .name("name")
                .map(|m| m.as_str())
                .unwrap_or("default")
                .to_string();
            symbols.push(SymbolStart {
                kind: js_symbol_kind(raw_kind, &name, true),
                name,
                exported: true,
                line_start,
                indent: leading_spaces(line),
            });
            continue;
        }
        if let Some(cap) = js_symbol_re().captures(line) {
            let raw_kind = cap.name("kind").map(|m| m.as_str()).unwrap_or("symbol");
            let Some(name) = cap.name("name").map(|m| m.as_str().to_string()) else {
                continue;
            };
            let exported = cap.name("export").is_some();
            symbols.push(SymbolStart {
                kind: js_symbol_kind(raw_kind, &name, exported),
                name,
                exported,
                line_start,
                indent: leading_spaces(line),
            });
        }
    }
    symbols
}

fn js_import_export_block_line(line: &str, depth: &mut usize) -> bool {
    let trimmed = line.trim_start();
    let starts_block = trimmed.starts_with("import {")
        || trimmed.starts_with("import type {")
        || trimmed.starts_with("export {")
        || trimmed.starts_with("export type {");
    if !starts_block && *depth == 0 {
        return false;
    }
    let opens = trimmed.matches('{').count();
    let closes = trimmed.matches('}').count();
    *depth = depth.saturating_add(opens).saturating_sub(closes);
    true
}

fn js_symbol_kind(raw_kind: &str, name: &str, exported: bool) -> String {
    if is_hook_name(name) && matches!(raw_kind, "function" | "const" | "let" | "var") {
        return "hook".to_string();
    }
    if exported
        && is_uppercase_symbol(name)
        && matches!(raw_kind, "function" | "const" | "let" | "var")
    {
        return "component".to_string();
    }
    match raw_kind {
        "let" | "var" => "variable",
        other => other,
    }
    .to_string()
}

fn extract_rust_symbols(text: &str) -> Vec<SymbolStart> {
    let mut symbols = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        if is_noise_line(line, "//") {
            continue;
        }
        let line_start = idx + 1;
        if let Some(cap) = rust_symbol_re().captures(line) {
            let Some(name) = cap.name("name").map(|m| m.as_str().to_string()) else {
                continue;
            };
            let raw_kind = cap.name("kind").map(|m| m.as_str()).unwrap_or("symbol");
            symbols.push(SymbolStart {
                name,
                kind: rust_symbol_kind(raw_kind).to_string(),
                exported: cap.name("pub").is_some(),
                line_start,
                indent: leading_spaces(line),
            });
            continue;
        }
        if let Some(cap) = rust_impl_re().captures(line) {
            let Some(name) = cap.name("name").map(|m| m.as_str().trim().to_string()) else {
                continue;
            };
            symbols.push(SymbolStart {
                name,
                kind: "impl".to_string(),
                exported: false,
                line_start,
                indent: leading_spaces(line),
            });
        }
    }
    symbols
}

fn rust_symbol_kind(raw_kind: &str) -> &str {
    match raw_kind {
        "fn" => "function",
        "mod" => "module",
        other => other,
    }
}

fn extract_python_symbols(text: &str) -> Vec<SymbolStart> {
    let mut symbols = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        if is_noise_line(line, "#") {
            continue;
        }
        let Some(cap) = python_symbol_re().captures(line) else {
            continue;
        };
        let Some(name) = cap.name("name").map(|m| m.as_str().to_string()) else {
            continue;
        };
        let raw_kind = cap.name("kind").map(|m| m.as_str()).unwrap_or("def");
        symbols.push(SymbolStart {
            name,
            kind: if raw_kind == "class" {
                "class".to_string()
            } else {
                "function".to_string()
            },
            exported: false,
            line_start: idx + 1,
            indent: leading_spaces(line),
        });
    }
    symbols
}

fn extract_go_symbols(text: &str) -> Vec<SymbolStart> {
    let mut symbols = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        if is_noise_line(line, "//") {
            continue;
        }
        let line_start = idx + 1;
        if let Some(cap) = go_func_symbol_re().captures(line) {
            let Some(name) = cap.name("name").map(|m| m.as_str().to_string()) else {
                continue;
            };
            symbols.push(SymbolStart {
                kind: if cap.name("receiver").is_some() {
                    "method".to_string()
                } else {
                    "function".to_string()
                },
                exported: is_uppercase_symbol(&name),
                name,
                line_start,
                indent: leading_spaces(line),
            });
            continue;
        }
        if let Some(cap) = go_type_symbol_re().captures(line) {
            let Some(name) = cap.name("name").map(|m| m.as_str().to_string()) else {
                continue;
            };
            let raw_kind = cap.name("kind").map(|m| m.as_str()).unwrap_or("type");
            let kind = match raw_kind {
                "struct" => "struct",
                "interface" => "interface",
                _ => "type",
            };
            symbols.push(SymbolStart {
                name: name.clone(),
                kind: kind.to_string(),
                exported: is_uppercase_symbol(&name),
                line_start,
                indent: leading_spaces(line),
            });
        }
    }
    symbols
}

fn extract_swift_symbols(text: &str) -> Vec<SymbolStart> {
    let mut symbols = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        if is_noise_line(line, "//") {
            continue;
        }
        let line_start = idx + 1;
        if let Some(cap) = swift_type_symbol_re().captures(line) {
            let Some(name) = cap.name("name").map(|m| m.as_str().to_string()) else {
                continue;
            };
            let raw_kind = cap.name("kind").map(|m| m.as_str()).unwrap_or("symbol");
            let modifiers = cap.name("mods").map(|m| m.as_str()).unwrap_or_default();
            symbols.push(SymbolStart {
                name,
                kind: raw_kind.to_string(),
                exported: swift_modifiers_are_exported(modifiers),
                line_start,
                indent: leading_spaces(line),
            });
            continue;
        }
        if let Some(cap) = swift_func_symbol_re().captures(line) {
            let Some(name) = cap.name("name").map(|m| m.as_str().to_string()) else {
                continue;
            };
            let modifiers = cap.name("mods").map(|m| m.as_str()).unwrap_or_default();
            symbols.push(SymbolStart {
                name,
                kind: "function".to_string(),
                exported: swift_modifiers_are_exported(modifiers),
                line_start,
                indent: leading_spaces(line),
            });
            continue;
        }
        if let Some(cap) = swift_property_symbol_re().captures(line) {
            let Some(name) = cap.name("name").map(|m| m.as_str().to_string()) else {
                continue;
            };
            let raw_kind = cap.name("kind").map(|m| m.as_str()).unwrap_or("var");
            let modifiers = cap.name("mods").map(|m| m.as_str()).unwrap_or_default();
            symbols.push(SymbolStart {
                name,
                kind: if raw_kind == "let" {
                    "constant".to_string()
                } else {
                    "property".to_string()
                },
                exported: swift_modifiers_are_exported(modifiers),
                line_start,
                indent: leading_spaces(line),
            });
        }
    }
    symbols
}

fn swift_modifiers_are_exported(modifiers: &str) -> bool {
    modifiers
        .split_whitespace()
        .any(|modifier| matches!(modifier, "public" | "open" | "package"))
}

fn is_noise_line(line: &str, comment_prefix: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with(comment_prefix) || trimmed.starts_with('*')
}

fn is_hook_name(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("use") else {
        return false;
    };
    rest.chars()
        .next()
        .map(|ch| ch.is_ascii_uppercase())
        .unwrap_or(false)
}

fn is_uppercase_symbol(name: &str) -> bool {
    name.chars()
        .next()
        .map(|ch| ch.is_ascii_uppercase())
        .unwrap_or(false)
}
