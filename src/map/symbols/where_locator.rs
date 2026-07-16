// Responsibility: map-symbols-where-locator
use crate::map::{ConsumerObservationInput, ObservationProjection};
use crate::map::{
    cone_symbol_report_with_references, consumer_observed_count, definition_match_observation,
    shell_quote, symbol_anchor_path, symbol_local_incoming_edges, symbol_reference_edge_set,
    unknown,
};
use crate::model::{
    FileInfo, ObservationLedger, Project, WhereDefinition, WhereReport, WhereSuggestion,
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
    let (shown, _) = crate::map::BoundedProjection::ordered(
        "where definitions hidden by limit",
        matched.clone(),
        definition_limit,
        &definition_expand_command(query, kind_filter),
    )
    .into_parts();
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
        let references = symbol_reference_edge_set(project, file_rel, query);
        let Some(mut cone_report) = cone_symbol_report_with_references(
            project,
            file_rel,
            query,
            1,
            include_hidden,
            limit,
            &references,
        ) else {
            continue;
        };
        let anchor = cone_report.anchor.clone();
        let anchor_path = symbol_anchor_path(file_rel, query);
        let all_consumers = references.production();
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
                observed_sources: Some(references.production_sources()),
            },
            &mut definition_observations,
        );
        let mut incoming = Vec::new();
        let mut verification = Vec::new();
        if total_matches == 1 {
            detail = Some({
                let mut report = cone_report;
                let all_remaining_incoming = symbol_local_incoming_edges(project, info, query);
                let remaining_limit = if include_hidden {
                    all_remaining_incoming.len()
                } else {
                    limit.min(2)
                };
                let remaining_incoming = all_remaining_incoming
                    .iter()
                    .take(remaining_limit)
                    .cloned()
                    .collect::<Vec<_>>();
                let expand = || format!("codemap cone {} --all", shell_quote(&anchor_path));
                let mut incoming_observations = ObservationLedger::default();
                consumer_observed_count(
                    project,
                    ConsumerObservationInput {
                        rel: file_rel,
                        symbol: Some(query),
                        raw: all_remaining_incoming.len(),
                        shown: remaining_incoming.len(),
                        group: "incoming",
                        expand: (remaining_incoming.len() < all_remaining_incoming.len())
                            .then(expand),
                        include_local: true,
                        observed_sources: Some(references.production_sources()),
                    },
                    &mut incoming_observations,
                );
                report.observations.merge(&incoming_observations);
                report.incoming.clone_from(&remaining_incoming);
                incoming = remaining_incoming;
                verification.clone_from(&report.proof);
                Box::new(report)
            });
            if let Some(report) = detail.as_deref() {
                definition_observations.merge(&report.observations);
            }
        } else {
            let report = &mut cone_report;
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
        schema_version: WhereReport::SCHEMA_VERSION,
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
