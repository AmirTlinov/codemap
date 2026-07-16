// Responsibility: repo-file-extract
mod css_imports;
mod language_import_bindings;
mod rust_includes;
mod rust_reexports;

pub(crate) use css_imports::*;
pub(crate) use language_import_bindings::*;
pub(crate) use rust_includes::*;
pub(crate) use rust_reexports::*;

use crate::model::FileInfo;
use crate::repo::{
    code_without_comments_or_strings, extract_go_imports, extract_identifier_references,
    extract_identifier_references_from_cleaned, extract_js_import_bindings,
    extract_js_import_specs, extract_js_symbols_from_cleaned, extract_jsx_tags,
    extract_jsx_tags_from_cleaned, extract_local_bindings, extract_local_bindings_from_cleaned,
    extract_surfaces, extract_symbols, is_asset_ext, is_source_ext, js_export_re,
    js_has_dynamic_import, py_import_re, rust_mod_re, swift_import_re,
};
use std::fs;
use std::path::Path;

pub(crate) fn extract_imports_exports(root: &Path, info: &mut FileInfo) {
    if is_asset_ext(&info.ext) && info.ext != "svg" {
        return;
    }
    let path = root.join(&info.rel);
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    info.content_hash = Some(scan_content_hash(text.as_bytes()));
    info.line_count = line_count(&text);
    if matches!(info.ext.as_str(), "css" | "scss" | "sass" | "less") {
        info.imports.extend(extract_css_import_specs(&text));
        return;
    }
    if !is_source_ext(&info.ext) {
        return;
    }
    let js_like = matches!(
        info.ext.as_str(),
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte"
    );
    let cleaned = js_like.then(|| code_without_comments_or_strings(&text, &info.ext));
    let cleaned_text = cleaned.as_deref().unwrap_or(&text);
    let surfaces = extract_surfaces(&text, &info.ext);
    info.surface_tokens = surfaces.tokens;
    info.surface_phrases = surfaces.phrases;
    info.visited_route_paths = surfaces.visited_routes;
    if js_like {
        info.symbols = extract_js_symbols_from_cleaned(&text, cleaned_text, &info.ext);
        info.references = extract_identifier_references_from_cleaned(cleaned_text);
        info.jsx_tags = extract_jsx_tags_from_cleaned(cleaned_text, &info.ext);
        info.local_bindings = extract_local_bindings_from_cleaned(cleaned_text, &info.ext);
    } else {
        info.symbols = extract_symbols(&text, &info.ext);
        info.references = extract_identifier_references(&text, &info.ext);
        info.jsx_tags = extract_jsx_tags(&text, &info.ext);
        info.local_bindings = extract_local_bindings(&text, &info.ext);
    }
    match info.ext.as_str() {
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte" => {
            info.imports.extend(extract_js_import_specs(&text));
            info.import_bindings = extract_js_import_bindings(&text);
            info.has_dynamic_import = js_has_dynamic_import(cleaned_text);
            info.has_dynamic_require = text.lines().any(crate::map::dynamic_require_line);
            let export_re = js_export_re();
            for cap in export_re.captures_iter(cleaned_text) {
                if let Some(m) = cap.get(1) {
                    info.exports.insert(m.as_str().trim().to_string());
                }
            }
        }
        "py" => {
            let import_re = py_import_re();
            for cap in import_re.captures_iter(&text) {
                if let Some(m) = cap.get(1).or_else(|| cap.get(2)) {
                    info.imports.insert(m.as_str().trim().to_string());
                }
            }
            merge_import_bindings(
                &mut info.import_bindings,
                extract_python_import_bindings(&text),
            );
        }
        "rs" => {
            let use_facts = extract_rust_use_facts(&text);
            info.imports.extend(use_facts.imports);
            merge_import_bindings(&mut info.import_bindings, use_facts.bindings);
            let qualified_facts = extract_rust_qualified_path_facts(&text);
            info.imports.extend(qualified_facts.imports);
            merge_import_bindings(&mut info.import_bindings, qualified_facts.bindings);
            let mod_re = rust_mod_re();
            for cap in mod_re.captures_iter(&text) {
                if let Some(m) = cap.get(1) {
                    info.imports.insert(m.as_str().trim().to_string());
                }
            }
            info.imports.extend(extract_rust_include_specs(&text));
        }
        "go" => {
            info.imports.extend(extract_go_imports(&text));
            merge_import_bindings(&mut info.import_bindings, extract_go_import_bindings(&text));
        }
        "swift" => {
            let import_re = swift_import_re();
            let import_text = code_without_comments_or_strings(&text, &info.ext);
            for cap in import_re.captures_iter(&import_text) {
                if let Some(m) = cap.get(1) {
                    info.imports.insert(m.as_str().trim().to_string());
                }
            }
        }
        _ => {}
    }
    for symbol in &info.symbols {
        if symbol.exported {
            info.exports.insert(symbol.name.clone());
        }
    }
}

fn merge_import_bindings(
    target: &mut crate::model::ImportBindingsBySpec,
    source: crate::model::ImportBindingsBySpec,
) {
    for (spec, bindings) in source {
        target.entry(spec).or_default().extend(bindings);
    }
}

pub(crate) fn line_count(text: &str) -> usize {
    text.lines().count()
}

pub(crate) fn scan_content_hash(bytes: &[u8]) -> String {
    let hash = <sha2::Sha256 as sha2::Digest>::digest(bytes);
    hash.iter()
        .flat_map(|b| [b >> 4, b & 0x0f])
        .take(16)
        .map(|n| char::from_digit(n as u32, 16).expect("hex digit"))
        .collect()
}

#[derive(Debug, Clone)]
pub(crate) struct SymbolStart {
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) exported: bool,
    pub(crate) line_start: usize,
    pub(crate) indent: usize,
}
