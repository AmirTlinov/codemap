// Responsibility: query-specific-unjoined-consumer-candidate-audit
use std::collections::BTreeSet;

use crate::model::{FileInfo, Project};

use super::{ConsumerBlindSpot, push_unsupported_binding_gap};

pub(super) fn collect_unobserved_query_gap(
    project: &Project,
    anchor_rel: &str,
    candidate: &FileInfo,
    symbol: Option<&str>,
    observed_sources: &BTreeSet<String>,
    out: &mut Vec<ConsumerBlindSpot>,
) {
    if candidate.content_hash.is_none() {
        return;
    }
    let Ok(text) = std::fs::read_to_string(project.root.join(&candidate.rel)) else {
        return;
    };
    if let Some(construct) = text
        .lines()
        .find_map(crate::map::runtime_generated_code_line)
    {
        push_unsupported_binding_gap(
            candidate,
            construct,
            "symbol consumers behind runtime-generated JavaScript",
            out,
        );
    }
    let code = crate::repo::code_without_comments_or_strings(&text, &candidate.ext);
    let unsupported_module_syntax = [
        "define(",
        "define (",
        "requirejs(",
        "requirejs (",
        "System.register(",
        "System.register (",
        "sap.ui.define(",
        "goog.require(",
        "importScripts(",
    ]
    .iter()
    .any(|probe| code.contains(probe));
    let query_is_unobserved = symbol.is_some_and(|name| {
        if observed_sources.contains(&candidate.rel) {
            has_unscoped_value_reference(candidate, &code, name)
        } else {
            exact_identifier_occurs(&code, name)
        }
    });
    if !unsupported_module_syntax && !query_is_unobserved && !code.contains("\\u") {
        return;
    }
    push_unsupported_binding_gap(
        candidate,
        if unsupported_module_syntax {
            "unsupported_static_module_system"
        } else if !candidate.resolved_imports.contains(anchor_rel) {
            "unresolved_static_import_target"
        } else if code.contains("\\u") {
            "escaped_consumer_identifier_spelling"
        } else {
            "unresolved_query_identifier_consumer_candidate"
        },
        "symbol consumers in a candidate file not joined to the anchor",
        out,
    );
}

fn has_unscoped_value_reference(candidate: &FileInfo, code: &str, query: &str) -> bool {
    let lexical_references = code
        .lines()
        .map(|line| crate::map::line_value_identifier_reference_count(line, query))
        .sum::<usize>();
    if lexical_references > 1 {
        // SymbolInfo owns only line ranges and the structural edge owns only one
        // location. Multiple value references in one observed file can cross a
        // same-line declaration boundary, so that file cannot close the scan.
        return true;
    }
    code.lines().enumerate().any(|(index, line)| {
        let line_number = index + 1;
        let inside_indexed_symbol = candidate
            .symbols
            .iter()
            .any(|symbol| line_number >= symbol.line_start && line_number <= symbol.line_end);
        if !inside_indexed_symbol {
            return crate::map::line_has_value_identifier_reference(line, query);
        }
        line.rsplit_once('}').is_some_and(|(_, suffix)| {
            crate::map::line_has_value_identifier_reference(suffix, query)
        })
    })
}

fn exact_identifier_occurs(code: &str, query: &str) -> bool {
    !query.is_empty()
        && code.match_indices(query).any(|(start, value)| {
            let before = code[..start].chars().next_back();
            let after = code[start + value.len()..].chars().next();
            before.is_none_or(|ch| !is_identifier_char(ch))
                && after.is_none_or(|ch| !is_identifier_char(ch))
        })
}

fn is_identifier_char(ch: char) -> bool {
    ch.is_alphanumeric() || matches!(ch, '_' | '$' | '\u{200c}' | '\u{200d}')
}
