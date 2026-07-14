// Responsibility: siblings-lens-report
use crate::map::{
    direct_files_under_directory, directory_contract_edges_at_depth, directory_edges,
    directory_seed_file_paths, file_kind_for_ls, files_under_directory,
    group_duplicate_proof_surfaces, proof_surfaces_for_file_paths, route_service_test_triplets,
    runtime_fact_index_for_paths, shell_quote, truncate_with_hidden,
};
use crate::model::{EvidenceStrength, FileInfo, HiddenGroup, Project, SiblingsReport, Surface};
use crate::repo;
use std::collections::BTreeMap;

pub fn siblings_report(
    project: &Project,
    scope: &str,
    include_hidden: bool,
    limit: usize,
) -> SiblingsReport {
    let limit = limit.max(1);
    let scope = repo::normalize_rel_path(scope);
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let (scope_files, recursive_hidden_count) =
        siblings_scope_files(project, &scope, include_hidden);
    for file in &scope_files {
        grouped
            .entry(file_kind_for_ls(file))
            .or_default()
            .push(file.rel.clone());
    }
    let mut same_kind = grouped
        .into_iter()
        .map(|(kind, mut examples)| {
            examples.sort();
            let count = examples.len();
            let hidden_count = count.saturating_sub(5);
            examples.truncate(5);
            Surface {
                id: format!("surface:siblings:{scope}:{kind}"),
                kind,
                path: None,
                role: Some("sibling_group".to_string()),
                evidence: "same_directory_and_kind".to_string(),
                strength: EvidenceStrength::Medium,
                count: Some(count),
                examples,
                hidden_count,
            }
        })
        .collect::<Vec<_>>();
    same_kind.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.kind.cmp(&b.kind)));
    let mut hidden = Vec::new();
    let include_hidden_expand = format!("codemap siblings {} --all", shell_quote(&scope));
    if recursive_hidden_count > 0 {
        hidden.push(HiddenGroup {
            reason: "recursive sibling files hidden at root scope".to_string(),
            count: recursive_hidden_count,
            expand: include_hidden_expand.clone(),
        });
    }
    truncate_with_hidden(
        &mut same_kind,
        limit,
        &mut hidden,
        "sibling groups hidden by limit",
        &include_hidden_expand,
    );
    let mut shared_helpers = directory_edges(project, &scope, include_hidden)
        .into_iter()
        .filter(|edge| edge.edge_type.contains("import"))
        .collect::<Vec<_>>();
    truncate_with_hidden(
        &mut shared_helpers,
        limit,
        &mut hidden,
        "shared helper edges hidden by limit",
        &include_hidden_expand,
    );
    let mut shared_contracts =
        directory_contract_edges_at_depth(project, &scope, include_hidden, 1);
    truncate_with_hidden(
        &mut shared_contracts,
        limit,
        &mut hidden,
        "shared contract edges hidden by limit",
        &include_hidden_expand,
    );
    let proof_map_raw_expand = format!(
        "codemap proof-map {} --raw-sensors --limit <larger-number>",
        shell_quote(&scope)
    );
    let proof_seed_files = if scope == "." && !include_hidden {
        Vec::new()
    } else {
        directory_seed_file_paths(project, &scope, false)
    };
    let mut proof_pattern = proof_surfaces_for_file_paths(project, &proof_seed_files, 1, limit);
    group_duplicate_proof_surfaces(
        &mut proof_pattern,
        &mut hidden,
        "duplicate proof pattern sensors grouped by structural key",
        &proof_map_raw_expand,
    );
    if !include_hidden {
        truncate_with_hidden(
            &mut proof_pattern,
            limit,
            &mut hidden,
            "proof pattern surfaces hidden by limit",
            &format!(
                "codemap proof-map {} --limit <larger-number>",
                shell_quote(&scope)
            ),
        );
    }
    let runtime_fact_paths = scope_files
        .iter()
        .map(|file| file.rel.clone())
        .collect::<Vec<_>>();
    let runtime_facts = runtime_fact_index_for_paths(project, &runtime_fact_paths);
    let mut route_service_test_triplets =
        route_service_test_triplets(&scope, &runtime_facts, &scope_files);
    truncate_with_hidden(
        &mut route_service_test_triplets,
        limit,
        &mut hidden,
        "route/service/test triplets hidden by limit",
        &include_hidden_expand,
    );
    SiblingsReport {
        kind: "siblings_report",
        schema_version: "2",
        scope: scope.clone(),
        same_kind,
        route_service_test_triplets,
        shared_helpers,
        shared_contracts,
        proof_pattern,
        unknowns: Vec::new(),
        hidden,
        expand: vec![format!("codemap ls {} --all", shell_quote(&scope))],
    }
}

fn siblings_scope_files<'a>(
    project: &'a Project,
    scope: &str,
    include_hidden: bool,
) -> (Vec<&'a FileInfo>, usize) {
    if scope == "." && !include_hidden {
        let direct = direct_files_under_directory(project, scope);
        let recursive_count = files_under_directory(project, scope).len();
        let hidden_count = recursive_count.saturating_sub(direct.len());
        return (direct, hidden_count);
    }
    (files_under_directory(project, scope), 0)
}
