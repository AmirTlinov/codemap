fn extract_imports_exports(root: &Path, info: &mut FileInfo) {
    let path = root.join(&info.rel);
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    info.line_count = line_count(&text);
    if !is_source_ext(&info.ext) {
        return;
    }
    let surfaces = extract_surfaces(&text, &info.ext);
    info.surface_tokens = surfaces.tokens;
    info.surface_phrases = surfaces.phrases;
    info.visited_route_paths = surfaces.visited_routes;
    info.symbols = extract_symbols(&text, &info.ext);
    info.references = extract_identifier_references(&text, &info.ext);
    info.jsx_tags = extract_jsx_tags(&text, &info.ext);
    info.local_bindings = extract_local_bindings(&text, &info.ext);
    match info.ext.as_str() {
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte" => {
            info.imports.extend(extract_js_import_specs(&text));
            info.import_bindings = extract_js_import_bindings(&text);
            let export_re = js_export_re();
            for cap in export_re.captures_iter(&text) {
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
            let def_re = py_def_re();
            for cap in def_re.captures_iter(&text) {
                if let Some(m) = cap.get(1) {
                    info.exports.insert(m.as_str().trim().to_string());
                }
            }
        }
        "rs" => {
            let use_re = rust_use_re();
            for cap in use_re.captures_iter(&text) {
                if let Some(m) = cap.get(1) {
                    info.imports.insert(m.as_str().trim().to_string());
                }
            }
            let mod_re = rust_mod_re();
            for cap in mod_re.captures_iter(&text) {
                if let Some(m) = cap.get(1) {
                    info.imports.insert(m.as_str().trim().to_string());
                }
            }
        }
        "go" => {
            info.imports.extend(extract_go_imports(&text));
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

fn line_count(text: &str) -> usize {
    text.lines().count()
}

#[derive(Debug, Clone)]
struct SymbolStart {
    name: String,
    kind: String,
    exported: bool,
    line_start: usize,
    indent: usize,
}

