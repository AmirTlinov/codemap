// Responsibility: map-symbols-where-locator
use crate::map::{
    ConeXrayInput, ConsumerObservationInput, ObservationProjection, cone_xray_card,
    consumer_observed_count, definition_match_observation,
};
use crate::map::{
    cone_symbol_report, shell_quote, sort_edges, symbol_anchor_path, symbol_file_summary,
    symbol_local_incoming_edges, symbol_reference_edges, unknown,
};
use crate::model::{
    FileInfo, ObservationLedger, Project, StructuralEdge, WhereDefinition, WhereReport,
    WhereSuggestion,
};
use std::collections::BTreeMap;

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
    let limit = limit.max(1);
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

    let mut observations = ObservationLedger::default();
    let definition_limit = if include_hidden {
        usize::MAX
    } else {
        limit.min(4)
    };
    let shown = if matched.len() <= definition_limit {
        matched.clone()
    } else {
        matched[..definition_limit].to_vec()
    };
    let definition_expand =
        (shown.len() < total_matches).then(|| definition_expand_command(query, kind_filter));
    let definition_scope = definition_observation_scope(query, kind_filter);
    definition_match_observation(
        project,
        query,
        ObservationProjection {
            group: "definition_matches",
            scope: &definition_scope,
            observed: total_matches,
            shown: shown.len(),
            expand: definition_expand,
        },
        &mut observations,
    );

    let mut definitions = Vec::new();
    let mut detail = None;
    for file_rel in &shown {
        let Some(info) = project.files.get(file_rel) else {
            continue;
        };
        let Some(anchor) = symbol_file_summary(project, info, query) else {
            continue;
        };
        let anchor_path = symbol_anchor_path(file_rel, query);
        let mut all_consumers = symbol_reference_edges(project, file_rel, query, false);
        sort_edges(&mut all_consumers);
        let consumers_raw = all_consumers.len();
        let consumer_limit = if include_hidden {
            consumers_raw
        } else if total_matches == 1 {
            limit.min(2)
        } else {
            0
        };
        let consumers = all_consumers
            .iter()
            .take(consumer_limit)
            .cloned()
            .collect::<Vec<_>>();
        let consumer_expand = (consumers.len() < consumers_raw)
            .then(|| format!("codemap cone {} --all", shell_quote(&anchor_path)));
        let mut definition_observations = ObservationLedger::default();
        let consumers_total = consumer_observed_count(
            project,
            ConsumerObservationInput {
                rel: file_rel,
                symbol: Some(query),
                raw: consumers_raw,
                shown: consumers.len(),
                group: "consumers",
                expand: consumer_expand,
                include_local: false,
            },
            &mut definition_observations,
        );
        let mut incoming = Vec::new();
        let mut verification = Vec::new();
        if total_matches == 1 {
            detail = cone_symbol_report(project, file_rel, query, 1, include_hidden, limit).map(
                |mut report| {
                    let mut all_incoming = all_consumers.clone();
                    all_incoming.extend(symbol_local_incoming_edges(project, info, query));
                    sort_edges(&mut all_incoming);
                    let remaining_limit = if include_hidden {
                        all_incoming.len()
                    } else {
                        limit.min(2)
                    };
                    let remaining_incoming = all_incoming
                        .iter()
                        .filter(|edge| !contains_edge(&all_consumers, edge))
                        .take(remaining_limit)
                        .cloned()
                        .collect::<Vec<_>>();
                    let mut visible_incoming = consumers.clone();
                    visible_incoming.extend(remaining_incoming.iter().cloned());
                    sort_edges(&mut visible_incoming);
                    reproject_horizon(
                        &mut report.observations,
                        "incoming",
                        visible_incoming.len(),
                        &anchor_path,
                    );
                    rebuild_where_xray(
                        project,
                        file_rel,
                        &mut report,
                        &remaining_incoming,
                        limit,
                        include_hidden,
                    );
                    report.incoming.clone_from(&visible_incoming);
                    incoming = visible_incoming;
                    verification.clone_from(&report.proof);
                    Box::new(report)
                },
            );
            if let Some(report) = detail.as_deref() {
                definition_observations.merge(&report.observations);
            }
        } else if let Some(mut report) =
            cone_symbol_report(project, file_rel, query, 1, include_hidden, limit)
        {
            if include_hidden {
                incoming.clone_from(&report.incoming);
                verification.clone_from(&report.proof);
            } else {
                reproject_horizon(&mut report.observations, "incoming", 0, &anchor_path);
                reproject_horizon(&mut report.observations, "verification", 0, &anchor_path);
                report.incoming.clear();
                report.proof.clear();
            }
            definition_observations.merge(&report.observations);
        }
        definitions.push(WhereDefinition {
            anchor,
            consumers,
            consumers_total,
            incoming,
            verification,
            observations: definition_observations,
            hidden: Vec::new(),
            expand: vec![
                format!("codemap cone {}", shell_quote(&anchor_path)),
                format!("codemap ls {}", shell_quote(file_rel)),
            ],
        });
    }

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
    } else if shown.len() < total_matches {
        expand.push(definition_expand_command(query, kind_filter));
    }

    WhereReport {
        kind: "where_report",
        schema_version: "6",
        query: query.to_string(),
        kind_filter: kind_filter.map(|kind| kind.to_string()),
        total_matches,
        observations,
        definitions,
        soft_suggestions,
        unknowns,
        hidden: Vec::new(),
        expand,
        detail,
    }
}

fn definition_observation_scope(query: &str, kind_filter: Option<&str>) -> String {
    let typed_query = serde_json::to_string(&(query, kind_filter))
        .expect("where definition coverage scope should serialize");
    format!("where:{typed_query}")
}

fn definition_expand_command(query: &str, kind_filter: Option<&str>) -> String {
    let mut command = format!("codemap where {}", shell_quote(query));
    if let Some(kind) = kind_filter {
        command.push_str(" --kind ");
        command.push_str(&shell_quote(kind));
    }
    command.push_str(" --all");
    command
}

fn contains_edge(edges: &[StructuralEdge], candidate: &StructuralEdge) -> bool {
    edges.iter().any(|edge| {
        edge.from == candidate.from
            && edge.to == candidate.to
            && edge.edge_type == candidate.edge_type
            && edge.evidence == candidate.evidence
    })
}

fn reproject_horizon(
    observations: &mut ObservationLedger,
    group: &str,
    shown: usize,
    anchor_path: &str,
) {
    let Some(horizon) = observations
        .horizons
        .iter_mut()
        .find(|horizon| horizon.group == group)
    else {
        return;
    };
    let shown = (shown as u64).min(horizon.count.observed);
    horizon.shown = shown;
    horizon.hidden = horizon.count.observed - shown;
    horizon.expand =
        (horizon.hidden > 0).then(|| format!("codemap cone {} --all", shell_quote(anchor_path)));
}

fn rebuild_where_xray(
    project: &Project,
    file_rel: &str,
    report: &mut crate::model::ConeReport,
    disjoint_incoming: &[StructuralEdge],
    limit: usize,
    include_hidden: bool,
) {
    let seed_files = [file_rel.to_string()];
    report.xray = cone_xray_card(ConeXrayInput {
        project,
        anchor: &report.anchor,
        seed_files: &seed_files,
        declared_env: &report.declared_env,
        outgoing: &report.outgoing,
        incoming: disjoint_incoming,
        proof: &report.proof,
        unknowns: &report.unknowns,
        limit,
        include_hidden,
    });
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
