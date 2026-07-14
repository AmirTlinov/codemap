// Responsibility: repo-symbol-language-starts
use crate::repo::{
    SymbolStart, go_func_symbol_re, go_type_symbol_re, js_default_symbol_re, js_symbol_re,
    leading_spaces, python_symbol_re, rust_impl_re, rust_symbol_re, swift_func_symbol_re,
    swift_property_symbol_re, swift_type_symbol_re,
};

pub(crate) fn extract_js_symbols(text: &str) -> Vec<SymbolStart> {
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

pub(crate) fn extract_rust_symbols(text: &str) -> Vec<SymbolStart> {
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

pub(crate) fn extract_python_symbols(text: &str) -> Vec<SymbolStart> {
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

pub(crate) fn extract_go_symbols(text: &str) -> Vec<SymbolStart> {
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

pub(crate) fn extract_swift_symbols(text: &str) -> Vec<SymbolStart> {
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

pub(crate) fn is_uppercase_symbol(name: &str) -> bool {
    name.chars()
        .next()
        .map(|ch| ch.is_ascii_uppercase())
        .unwrap_or(false)
}
