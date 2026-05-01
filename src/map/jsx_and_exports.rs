fn line_has_jsx_tag_identifier_reference(line: &str, name: &str) -> bool {
    if !name
        .bytes()
        .next()
        .map(|byte| byte.is_ascii_uppercase())
        .unwrap_or(false)
    {
        return false;
    }
    identifier_ranges(line, name).any(|(start, end)| {
        let before = &line[..start];
        let Some(tag_start) = before.rfind('<') else {
            return false;
        };
        if previous_nonspace_byte(&before[tag_start + 1..]).is_some() {
            return false;
        }
        let tag_before = before[..tag_start].bytes().next_back();
        if tag_before
            .map(|byte| is_identifier_byte(byte) || matches!(byte, b'.' | b'$' | b'/'))
            .unwrap_or(false)
        {
            return false;
        }
        let after = line[end..].trim_start();
        if matches!(after.bytes().next(), Some(b'|' | b'&' | b',' | b'=')) {
            return false;
        }
        if after.starts_with("extends ")
            || after.starts_with("extends\t")
            || after.starts_with("extends\n")
            || after.starts_with(">()")
            || after.starts_with("> ()")
        {
            return false;
        }
        line[end..]
            .bytes()
            .next()
            .map(|byte| byte.is_ascii_whitespace() || matches!(byte, b'>' | b'/'))
            .unwrap_or(true)
    })
}

fn symbol_contract_edges(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
) -> Vec<StructuralEdge> {
    let exported = symbol_is_exported(project, file_rel, symbol_name);
    if !exported {
        return Vec::new();
    }
    vec![StructuralEdge {
        from: symbol_anchor_path(file_rel, symbol_name),
        to: file_rel.to_string(),
        edge_type: "contract".to_string(),
        evidence: "exported_symbol".to_string(),
        strength: EvidenceStrength::High,
    }]
}

fn file_imported_symbol_reference_kind(
    project: &Project,
    file: &FileInfo,
    file_rel: &str,
    symbol_name: &str,
) -> Option<ImportedSymbolReferenceKind> {
    if file_directly_references_imported_symbol(project, file, file_rel, symbol_name) {
        return Some(ImportedSymbolReferenceKind::Direct);
    }
    if file_references_reexported_symbol(project, file, file_rel, symbol_name) {
        return Some(ImportedSymbolReferenceKind::Reexported);
    }
    None
}

fn file_directly_references_imported_symbol(
    project: &Project,
    file: &FileInfo,
    file_rel: &str,
    symbol_name: &str,
) -> bool {
    let Some(bindings) = file.resolved_import_bindings.get(file_rel) else {
        return false;
    };
    bindings.iter().any(|(local, imported)| {
        imported_symbol_binding_matches(project, file_rel, symbol_name, imported)
            && !file_has_local_value_shadow(file, local)
            && (file.jsx_tags.contains(local)
                || file_references_identifier_after_imports(project, file, local))
    })
}

fn file_references_reexported_symbol(
    project: &Project,
    file: &FileInfo,
    file_rel: &str,
    symbol_name: &str,
) -> bool {
    for (barrel_rel, imported_from_barrel) in &file.resolved_import_bindings {
        if barrel_rel == file_rel {
            continue;
        }
        let Some(barrel) = project.files.get(barrel_rel) else {
            continue;
        };
        for (local, imported_public_name) in imported_from_barrel {
            if !barrel_reexports_symbol_from_file(
                project,
                barrel,
                file_rel,
                symbol_name,
                imported_public_name,
            ) {
                continue;
            }
            if file_has_local_value_shadow(file, local) {
                continue;
            }
            if file.jsx_tags.contains(local)
                || file_references_identifier_after_imports(project, file, local)
            {
                return true;
            }
        }
    }
    false
}

fn barrel_reexports_symbol_from_file(
    project: &Project,
    barrel: &FileInfo,
    file_rel: &str,
    symbol_name: &str,
    imported_public_name: &str,
) -> bool {
    let mut seen = BTreeSet::new();
    barrel_reexports_symbol_from_file_inner(
        project,
        barrel,
        file_rel,
        symbol_name,
        imported_public_name,
        &mut seen,
    )
}

fn barrel_reexports_symbol_from_file_inner(
    project: &Project,
    barrel: &FileInfo,
    file_rel: &str,
    symbol_name: &str,
    imported_public_name: &str,
    seen: &mut BTreeSet<(String, String)>,
) -> bool {
    if !seen.insert((barrel.rel.clone(), imported_public_name.to_string())) {
        return false;
    }
    match barrel_public_name_resolution(project, barrel, imported_public_name) {
        Some(BarrelPublicNameResolution::Explicit {
            target_rel,
            imported_name,
        }) => {
            let direct_match = target_rel == file_rel
                && imported_symbol_binding_matches(project, file_rel, symbol_name, &imported_name);
            if direct_match {
                return true;
            }
            project
                .files
                .get(&target_rel)
                .map(|target| {
                    barrel_reexports_symbol_from_file_inner(
                        project,
                        target,
                        file_rel,
                        symbol_name,
                        &imported_name,
                        seen,
                    )
                })
                .unwrap_or(false)
        }
        Some(BarrelPublicNameResolution::Star { target_rel }) => {
            let direct_match = target_rel == file_rel
                && symbol_exported_under_public_name(
                    project,
                    file_rel,
                    symbol_name,
                    imported_public_name,
                );
            if direct_match {
                return true;
            }
            project
                .files
                .get(&target_rel)
                .map(|target| {
                    barrel_reexports_symbol_from_file_inner(
                        project,
                        target,
                        file_rel,
                        symbol_name,
                        imported_public_name,
                        seen,
                    )
                })
                .unwrap_or(false)
        }
        None => false,
    }
}

fn barrel_public_name_resolution(
    project: &Project,
    barrel: &FileInfo,
    public_name: &str,
) -> Option<BarrelPublicNameResolution> {
    let mut seen = BTreeSet::new();
    barrel_public_name_resolution_inner(project, barrel, public_name, &mut seen)
}

fn barrel_public_name_resolution_inner(
    project: &Project,
    barrel: &FileInfo,
    public_name: &str,
    seen: &mut BTreeSet<(String, String)>,
) -> Option<BarrelPublicNameResolution> {
    if !seen.insert((barrel.rel.clone(), public_name.to_string())) {
        return None;
    }
    let mut explicit_owners = BTreeSet::new();
    if file_has_inline_named_export(project, &barrel.rel, public_name)
        || barrel_has_local_named_export_public_name(project, barrel, public_name)
    {
        explicit_owners.insert((barrel.rel.clone(), public_name.to_string()));
    }
    for (target_rel, bindings) in &barrel.resolved_import_bindings {
        for (exported, imported_name) in bindings {
            if exported.strip_prefix("export:") == Some(public_name) {
                explicit_owners.insert((target_rel.clone(), imported_name.clone()));
            }
        }
    }
    if !explicit_owners.is_empty() {
        if explicit_owners.len() != 1 {
            return None;
        }
        let (target_rel, imported_name) = explicit_owners.into_iter().next()?;
        return Some(BarrelPublicNameResolution::Explicit {
            target_rel,
            imported_name,
        });
    }
    if public_name == "default" {
        return None;
    }

    let star_owners = barrel
        .resolved_import_bindings
        .iter()
        .filter(|(target_rel, bindings)| {
            bindings
                .get("export:*")
                .map(|value| {
                    value == "*"
                        && file_exposes_public_name_for_star(project, target_rel, public_name, seen)
                })
                .unwrap_or(false)
        })
        .map(|(target_rel, _)| target_rel.clone())
        .collect::<BTreeSet<_>>();
    if star_owners.len() != 1 {
        return None;
    }
    star_owners
        .into_iter()
        .next()
        .map(|target_rel| BarrelPublicNameResolution::Star { target_rel })
}

fn file_exposes_public_name_for_star(
    project: &Project,
    file_rel: &str,
    public_name: &str,
    seen: &BTreeSet<(String, String)>,
) -> bool {
    if public_name == "default" {
        return false;
    }
    if file_has_named_public_export(project, file_rel, public_name) {
        return true;
    }
    let Some(file) = project.files.get(file_rel) else {
        return false;
    };
    let mut candidate_seen = seen.clone();
    barrel_public_name_resolution_inner(project, file, public_name, &mut candidate_seen).is_some()
}

fn barrel_has_local_named_export_public_name(
    project: &Project,
    barrel: &FileInfo,
    public_name: &str,
) -> bool {
    let Ok(text) = std::fs::read_to_string(project.root.join(&barrel.rel)) else {
        return false;
    };
    for statement in local_named_export_statement_slices(&text) {
        if local_named_export_bindings_from_statement(statement).contains_key(public_name) {
            return true;
        }
    }
    false
}

fn local_named_export_bindings(
    project: &Project,
    file_rel: &str,
) -> BTreeMap<String, BTreeSet<String>> {
    let Ok(text) = std::fs::read_to_string(project.root.join(file_rel)) else {
        return BTreeMap::new();
    };
    let mut out = BTreeMap::new();
    for statement in local_named_export_statement_slices(&text) {
        for (public_name, local_names) in local_named_export_bindings_from_statement(statement) {
            out.entry(public_name)
                .or_insert_with(BTreeSet::new)
                .extend(local_names);
        }
    }
    out
}

fn local_named_export_statement_slices(text: &str) -> Vec<&str> {
    js_map_keyword_positions(text, "export")
        .into_iter()
        .filter_map(|start| js_local_named_export_statement_slice(text, start))
        .collect()
}

fn js_local_named_export_statement_slice(text: &str, start: usize) -> Option<&str> {
    let bytes = text.as_bytes();
    let mut index = js_map_skip_ascii_whitespace(text, start + "export".len());
    if js_map_keyword_at(bytes, index, b"type") {
        return None;
    }
    if bytes.get(index) != Some(&b'{') {
        return None;
    }

    let mut state = JsMapScanState::Code;
    let mut brace_depth = 0usize;
    while index < bytes.len() {
        match state {
            JsMapScanState::Code => {
                if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
                    state = JsMapScanState::LineComment;
                    index += 2;
                    continue;
                }
                if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
                    state = JsMapScanState::BlockComment;
                    index += 2;
                    continue;
                }
                if bytes[index] == b'/'
                    && js_regex_literal_can_start(
                        std::string::String::from_utf8_lossy(&bytes[start..index]).as_ref(),
                    )
                    && let Some(end) = js_regex_literal_end(bytes, index)
                {
                    index = end;
                    continue;
                }
                if matches!(bytes[index], b'\'' | b'"') {
                    state = JsMapScanState::Quoted(bytes[index]);
                } else if bytes[index] == b'`' {
                    state = JsMapScanState::Template;
                } else if bytes[index] == b'{' {
                    brace_depth += 1;
                } else if bytes[index] == b'}' {
                    brace_depth = brace_depth.saturating_sub(1);
                    if brace_depth == 0 {
                        let after_brace = js_map_skip_trivia(text, index + 1);
                        if js_map_keyword_at(bytes, after_brace, b"from") {
                            return None;
                        }
                        let end = if bytes.get(after_brace) == Some(&b';') {
                            after_brace + 1
                        } else {
                            index + 1
                        };
                        return Some(&text[start..end]);
                    }
                }
            }
            JsMapScanState::LineComment => {
                if bytes[index] == b'\n' {
                    state = JsMapScanState::Code;
                }
            }
            JsMapScanState::BlockComment => {
                if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                    state = JsMapScanState::Code;
                    index += 2;
                    continue;
                }
            }
            JsMapScanState::Quoted(quote) => {
                if bytes[index] == b'\\' {
                    index = index.saturating_add(2);
                    continue;
                }
                if bytes[index] == quote {
                    state = JsMapScanState::Code;
                }
            }
            JsMapScanState::Template => {
                if bytes[index] == b'\\' {
                    index = index.saturating_add(2);
                    continue;
                }
                if bytes[index] == b'`' {
                    state = JsMapScanState::Code;
                }
            }
        }
        index += 1;
    }
    None
}
