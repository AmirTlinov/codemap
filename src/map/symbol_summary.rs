fn file_summary(
    project: &Project,
    info: &FileInfo,
    include_hidden: bool,
    limit: usize,
) -> FileSummary {
    let symbols = file_symbol_visibility(info, include_hidden, limit).symbols;
    FileSummary {
        path: info.rel.clone(),
        kind: file_kind_for_ls(info),
        package: package_name_for_file(project, &info.rel),
        language: info.language.clone(),
        lines: info.line_count,
        roles: structural_roles_for_ls(info),
        symbols,
        exports: info.exports.iter().cloned().collect(),
        imports: info.imports.iter().cloned().collect(),
        imported_by_count: project
            .reverse_imports
            .get(&info.rel)
            .map(|importers| importers.len())
            .unwrap_or(0),
    }
}

struct SymbolVisibility {
    symbols: Vec<crate::model::SymbolInfo>,
    hidden_by_default: usize,
    hidden_by_limit: usize,
}

fn file_symbol_visibility(info: &FileInfo, include_hidden: bool, limit: usize) -> SymbolVisibility {
    if include_hidden {
        return SymbolVisibility {
            symbols: info.symbols.clone(),
            hidden_by_default: 0,
            hidden_by_limit: 0,
        };
    }
    let mut symbols = info
        .symbols
        .iter()
        .filter(|symbol| symbol_is_default_visible(info, symbol))
        .cloned()
        .collect::<Vec<_>>();
    let hidden_by_default = info.symbols.len().saturating_sub(symbols.len());
    let before_limit = symbols.len();
    symbols.truncate(limit);
    SymbolVisibility {
        hidden_by_default,
        hidden_by_limit: before_limit.saturating_sub(symbols.len()),
        symbols,
    }
}

fn push_symbol_hidden_groups(
    hidden: &mut Vec<HiddenGroup>,
    info: &FileInfo,
    include_hidden: bool,
    limit: usize,
    expand: &str,
) {
    let visibility = file_symbol_visibility(info, include_hidden, limit);
    if visibility.hidden_by_default > 0 {
        hidden.push(HiddenGroup {
            reason: "nested symbols hidden by default".to_string(),
            count: visibility.hidden_by_default,
            expand: expand.to_string(),
        });
    }
    if visibility.hidden_by_limit > 0 {
        hidden.push(HiddenGroup {
            reason: "symbols hidden by limit".to_string(),
            count: visibility.hidden_by_limit,
            expand: expand.to_string(),
        });
    }
}

fn symbol_is_default_visible(info: &FileInfo, symbol: &crate::model::SymbolInfo) -> bool {
    if symbol.exported {
        return true;
    }
    if symbol_is_low_signal_nested_member(info, symbol) {
        return false;
    }
    true
}

fn symbol_is_low_signal_nested_member(info: &FileInfo, symbol: &crate::model::SymbolInfo) -> bool {
    if !matches!(
        symbol.kind.as_str(),
        "property" | "constant" | "const" | "variable"
    ) {
        return false;
    }
    info.symbols.iter().any(|container| {
        container.line_start < symbol.line_start
            && container.line_end >= symbol.line_end
            && symbol_kind_can_contain_members(&container.kind)
    })
}

fn symbol_kind_can_contain_members(kind: &str) -> bool {
    matches!(
        kind,
        "class"
            | "struct"
            | "enum"
            | "interface"
            | "type"
            | "impl"
            | "trait"
            | "function"
            | "method"
            | "component"
            | "hook"
    )
}

fn symbol_file_summary(
    project: &Project,
    info: &FileInfo,
    symbol_name: &str,
) -> Option<FileSummary> {
    let symbols = matching_symbols(info, symbol_name);
    if symbols.is_empty() {
        return None;
    }
    let line_start = symbols
        .iter()
        .map(|symbol| symbol.line_start)
        .min()
        .unwrap_or(1);
    let line_end = symbols
        .iter()
        .map(|symbol| symbol.line_end)
        .max()
        .unwrap_or(line_start);
    let exported = symbol_is_exported(project, &info.rel, symbol_name);
    let kind = if symbols.len() == 1 {
        format!("symbol:{}", symbols[0].kind)
    } else {
        "symbol".to_string()
    };
    Some(FileSummary {
        path: symbol_anchor_path(&info.rel, symbol_name),
        kind,
        package: package_name_for_file(project, &info.rel),
        language: info.language.clone(),
        lines: line_end.saturating_sub(line_start).saturating_add(1),
        roles: structural_roles_for_ls(info),
        symbols,
        exports: exported
            .then(|| symbol_name.to_string())
            .into_iter()
            .collect(),
        imports: Vec::new(),
        imported_by_count: symbol_reference_edges(project, &info.rel, symbol_name, true).len(),
    })
}

fn matching_symbols(info: &FileInfo, symbol_name: &str) -> Vec<crate::model::SymbolInfo> {
    info.symbols
        .iter()
        .filter(|symbol| symbol.name == symbol_name)
        .cloned()
        .collect()
}

fn split_symbol_anchor(rel: &str) -> Option<(String, String)> {
    let (file, symbol) = rel.rsplit_once('#')?;
    let file = repo::normalize_rel_path(file);
    let symbol = symbol.trim();
    if file == "." || symbol.is_empty() || symbol.contains('/') || symbol.contains('\\') {
        return None;
    }
    Some((file, symbol.to_string()))
}

fn symbol_anchor_path(file_rel: &str, symbol_name: &str) -> String {
    format!("{file_rel}#{symbol_name}")
}
