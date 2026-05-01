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
            let export_text = code_without_comments_or_strings(&text, &info.ext);
            let export_re = js_export_re();
            for cap in export_re.captures_iter(&export_text) {
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
            info.imports.extend(extract_rust_include_specs(&text));
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

fn extract_rust_include_specs(text: &str) -> BTreeSet<String> {
    let bytes = text.as_bytes();
    let mut specs = BTreeSet::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            let mut depth = 1usize;
            index += 2;
            while index + 1 < bytes.len() && depth > 0 {
                if bytes[index] == b'/' && bytes[index + 1] == b'*' {
                    depth += 1;
                    index += 2;
                    continue;
                }
                if bytes[index] == b'*' && bytes[index + 1] == b'/' {
                    depth = depth.saturating_sub(1);
                    index += 2;
                    continue;
                }
                index += 1;
            }
            continue;
        }
        if let Some((len, hashes)) = rust_raw_string_start(bytes, index) {
            index += len;
            while index < bytes.len() && !rust_raw_string_end_at(bytes, index, hashes) {
                index += 1;
            }
            index = (index + hashes + 1).min(bytes.len());
            continue;
        }
        if bytes[index] == b'"' {
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(bytes.len());
                    continue;
                }
                if bytes[index] == b'"' {
                    index += 1;
                    break;
                }
                index += 1;
            }
            continue;
        }
        if let Some(len) = rust_char_literal_len(bytes, index) {
            index += len;
            continue;
        }
        if rust_identifier_at(bytes, index, b"include")
            && let Some((spec, next)) = parse_rust_include_spec(bytes, index + "include".len())
        {
            specs.insert(spec);
            index = next;
            continue;
        }
        index += 1;
    }
    specs
}

fn parse_rust_include_spec(bytes: &[u8], mut index: usize) -> Option<(String, usize)> {
    index = skip_ascii_space(bytes, index);
    if bytes.get(index) != Some(&b'!') {
        return None;
    }
    index = skip_ascii_space(bytes, index + 1);
    if bytes.get(index) != Some(&b'(') {
        return None;
    }
    index = skip_ascii_space(bytes, index + 1);
    parse_rust_string_literal(bytes, index)
        .and_then(|(spec, next)| spec.ends_with(".rs").then_some((spec, next)))
}

fn parse_rust_string_literal(bytes: &[u8], mut index: usize) -> Option<(String, usize)> {
    if let Some((len, hashes)) = rust_raw_string_start(bytes, index) {
        let start = index + len;
        index = start;
        while index < bytes.len() && !rust_raw_string_end_at(bytes, index, hashes) {
            index += 1;
        }
        if index >= bytes.len() {
            return None;
        }
        let spec = std::str::from_utf8(&bytes[start..index]).ok()?.to_string();
        return Some((spec, index + hashes + 1));
    }
    if bytes.get(index) != Some(&b'"') {
        return None;
    }
    let start = index + 1;
    index = start;
    while index < bytes.len() {
        if bytes[index] == b'\\' {
            index += 2;
            continue;
        }
        if bytes[index] == b'"' {
            let spec = std::str::from_utf8(&bytes[start..index]).ok()?.to_string();
            return Some((spec, index + 1));
        }
        index += 1;
    }
    None
}

fn skip_ascii_space(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    index
}

fn rust_identifier_at(bytes: &[u8], index: usize, ident: &[u8]) -> bool {
    bytes
        .get(index..index + ident.len())
        .is_some_and(|slice| slice == ident)
        && index
            .checked_sub(1)
            .and_then(|before| bytes.get(before))
            .is_none_or(|byte| !rust_identifier_byte(*byte))
        && bytes
            .get(index + ident.len())
            .is_none_or(|byte| !rust_identifier_byte(*byte))
}

fn rust_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn rust_char_literal_len(bytes: &[u8], index: usize) -> Option<usize> {
    if bytes.get(index) != Some(&b'\'') {
        return None;
    }
    let mut cursor = index + 1;
    if cursor >= bytes.len() || bytes[cursor] == b'\n' {
        return None;
    }
    if bytes[cursor] == b'\\' {
        cursor = cursor.saturating_add(2);
    } else {
        while cursor < bytes.len() && bytes[cursor] != b'\'' && bytes[cursor] != b'\n' {
            cursor += 1;
            if cursor.saturating_sub(index) > 8 {
                return None;
            }
        }
    }
    (bytes.get(cursor) == Some(&b'\'')).then_some(cursor - index + 1)
}

fn rust_raw_string_start(bytes: &[u8], index: usize) -> Option<(usize, usize)> {
    if index
        .checked_sub(1)
        .and_then(|previous| bytes.get(previous))
        .copied()
        .is_some_and(rust_identifier_byte)
    {
        return None;
    }
    let mut cursor = index;
    if bytes.get(cursor) == Some(&b'b') && bytes.get(cursor + 1) == Some(&b'r') {
        cursor += 2;
    } else if bytes.get(cursor) == Some(&b'r') {
        cursor += 1;
    } else {
        return None;
    }
    let mut hashes = 0usize;
    while bytes.get(cursor) == Some(&b'#') {
        hashes += 1;
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b'"') {
        return None;
    }
    Some((cursor - index + 1, hashes))
}

fn rust_raw_string_end_at(bytes: &[u8], index: usize, hashes: usize) -> bool {
    bytes.get(index) == Some(&b'"')
        && (0..hashes).all(|offset| bytes.get(index + 1 + offset) == Some(&b'#'))
}

#[derive(Debug, Clone)]
struct SymbolStart {
    name: String,
    kind: String,
    exported: bool,
    line_start: usize,
    indent: usize,
}
