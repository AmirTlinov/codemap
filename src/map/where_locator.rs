// `where <symbol>` is a deterministic resolver from a symbol name to every
// `file#symbol` anchor that defines it, layered on the existing cone engine. It
// is a lookup, not search: definitions are enumerated by path, never ranked, and
// no "best file" is chosen.
pub fn where_report(
    project: &Project,
    query: &str,
    kind_filter: Option<&str>,
    include_hidden: bool,
    limit: usize,
) -> WhereReport {
    let query = query.trim();
    // Accept the displayed kind form (`symbol:function`) as well as the bare kind
    // (`function`), so a kind copied from where output filters correctly.
    let kind_filter = kind_filter.map(|kind| kind.strip_prefix("symbol:").unwrap_or(kind));

    // Every file that defines a symbol with this exact name (BTreeMap iteration is
    // already sorted by path, so enumeration is deterministic).
    let mut matched: Vec<String> = project
        .files
        .values()
        .filter(|info| file_defines_symbol(info, query, kind_filter))
        .map(|info| info.rel.clone())
        .collect();
    matched.sort();
    let total_matches = matched.len();

    let mut hidden = Vec::new();
    let shown = if include_hidden || matched.len() <= limit {
        matched.clone()
    } else {
        let hidden_count = matched.len() - limit;
        let shown = matched[..limit].to_vec();
        hidden.push(HiddenGroup {
            reason: "definitions hidden by limit".to_string(),
            count: hidden_count,
            expand: format!("codemap where {} --all", shell_quote(query)),
        });
        shown
    };

    let mut definitions = Vec::new();
    for file_rel in &shown {
        let Some(info) = project.files.get(file_rel) else {
            continue;
        };
        let Some(anchor) = symbol_file_summary(project, info, query) else {
            continue;
        };
        let anchor_path = symbol_anchor_path(file_rel, query);
        let mut consumers = symbol_reference_edges(project, file_rel, query, false);
        sort_edges(&mut consumers);
        let consumers_total = consumers.len();
        let mut def_hidden = Vec::new();
        if !include_hidden && consumers.len() > limit {
            let hidden_count = consumers.len() - limit;
            consumers.truncate(limit);
            def_hidden.push(HiddenGroup {
                reason: "consumers hidden by limit".to_string(),
                count: hidden_count,
                expand: format!("codemap cone {} --all", shell_quote(&anchor_path)),
            });
        }
        definitions.push(WhereDefinition {
            anchor,
            consumers,
            consumers_total,
            hidden: def_hidden,
            expand: vec![
                format!("codemap cone {}", shell_quote(&anchor_path)),
                format!("codemap ls {}", shell_quote(file_rel)),
            ],
        });
    }

    // Exactly one definition: attach the full cone map so single-`where` is a
    // structural twin of `cone file#symbol`.
    let detail = if shown.len() == 1 {
        cone_symbol_report(project, &shown[0], query, 1, include_hidden, limit).map(Box::new)
    } else {
        None
    };

    let mut unknowns = Vec::new();
    let mut soft_suggestions = Vec::new();
    let mut expand = Vec::new();
    if total_matches == 0 {
        unknowns.push(unknown(
            "symbol_not_found",
            None::<&str>,
            None,
            format!("symbol `{query}` is not defined in the indexed map"),
            "where is an exact name lookup; no definition matched this symbol name",
            Some("codemap ls ."),
        ));
        soft_suggestions = where_soft_suggestions(project, query, kind_filter);
        expand.push("codemap ls .".to_string());
    } else if !hidden.is_empty() {
        expand.push(format!("codemap where {} --all", shell_quote(query)));
    }

    WhereReport {
        kind: "where_report",
        schema_version: "1",
        query: query.to_string(),
        kind_filter: kind_filter.map(|kind| kind.to_string()),
        total_matches,
        definitions,
        soft_suggestions,
        unknowns,
        hidden,
        expand,
        detail,
    }
}

fn file_defines_symbol(info: &FileInfo, name: &str, kind_filter: Option<&str>) -> bool {
    info.symbols.iter().any(|symbol| {
        symbol.name == name
            && kind_filter
                .map(|kind| symbol.kind.eq_ignore_ascii_case(kind))
                .unwrap_or(true)
    })
}

// Soft fallback for a not-found query: exact substring matches over symbol names.
// Deterministic (sorted by name), explicitly not ranked, and never an "answer".
fn where_soft_suggestions(
    project: &Project,
    query: &str,
    kind_filter: Option<&str>,
) -> Vec<WhereSuggestion> {
    if query.len() < 3 {
        return Vec::new();
    }
    let needle = query.to_ascii_lowercase();
    let mut by_name: BTreeMap<String, (String, usize)> = BTreeMap::new();
    for info in project.files.values() {
        for symbol in &info.symbols {
            if symbol.name == query {
                continue;
            }
            if kind_filter.is_some_and(|kind| !symbol.kind.eq_ignore_ascii_case(kind)) {
                continue;
            }
            if symbol.name.to_ascii_lowercase().contains(&needle) {
                let entry = by_name
                    .entry(symbol.name.clone())
                    .or_insert((info.rel.clone(), 0));
                entry.1 += 1;
            }
        }
    }
    by_name
        .into_iter()
        .take(10)
        .map(|(name, (defined_in, definition_count))| WhereSuggestion {
            expand: format!("codemap where {}", shell_quote(&name)),
            name,
            defined_in,
            definition_count,
        })
        .collect()
}
