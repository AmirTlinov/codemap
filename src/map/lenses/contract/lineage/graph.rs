// Responsibility: exact-import-topic-and-verification-lineage
use super::entity_surface;
use crate::map::{
    anchor_symbol_reference_names, cone_proof_edges, direct_consumer_edges, env_declared_keys,
    env_surfaces_for_file, owner_line_containing, package_for_rel, quoted_literal_at,
    runtime_code_lines, structural_edge_with_locations, unknown,
};
use crate::model::{EvidenceLocation, EvidenceStrength, Project, StructuralEdge, Surface, Unknown};
use std::collections::{BTreeSet, VecDeque};

#[derive(Default)]
pub(super) struct ConnectedLineage {
    pub(super) declarations: Vec<Surface>,
    pub(super) edges: Vec<StructuralEdge>,
    pub(super) proof: Vec<StructuralEdge>,
    pub(super) unknowns: Vec<Unknown>,
}

pub(super) fn connected_lineage(project: &Project, seeds: &[String]) -> ConnectedLineage {
    let mut out = ConnectedLineage::default();
    let relevant_symbols = seeds
        .iter()
        .filter_map(|seed| project.files.get(seed))
        .flat_map(anchor_symbol_reference_names)
        .collect::<BTreeSet<_>>();
    let mut queue = seeds
        .iter()
        .cloned()
        .map(|seed| (seed, 0usize))
        .collect::<VecDeque<_>>();
    let mut visited = BTreeSet::new();
    while let Some((owner, depth)) = queue.pop_front() {
        if depth > 6 || visited.len() >= 128 || !visited.insert(owner.clone()) {
            continue;
        }
        add_config_references(project, &owner, &mut out);
        add_topics(project, &owner, &mut out);
        let consumers = owner_consumers(project, &owner, &relevant_symbols);
        if depth == 0 || consumers.is_empty() {
            add_direct_proof(project, &owner, &mut out.proof);
        }
        for edge in consumers {
            let Some(consumer) = project.files.get(&edge.from) else {
                continue;
            };
            if consumer.has_role("test") || consumer.has_role("test_support") {
                continue;
            }
            out.edges.push(structural_edge_with_locations(
                edge.from.clone(),
                owner.clone(),
                "consumes",
                if depth == 0 {
                    "direct_static_consumer"
                } else {
                    "mediated_static_consumer"
                },
                edge.strength,
                edge.locations,
            ));
            queue.push_back((edge.from, depth + 1));
        }
    }
    out
}

fn add_config_references(project: &Project, rel: &str, out: &mut ConnectedLineage) {
    let Some(file) = project.files.get(rel) else {
        return;
    };
    for config in env_surfaces_for_file(project, file) {
        let anchor = format!("config:{}", config.name);
        let reference_line = config
            .locations
            .first()
            .and_then(|location| location.line_start)
            .unwrap_or(1);
        let declaration = config.declaration.as_deref().and_then(|path| {
            env_declared_keys(project, path)
                .into_iter()
                .find(|(name, _)| name == &config.name)
                .map(|(_, line)| (path, line))
        });
        let (entity_path, entity_line, evidence) = declaration
            .as_ref()
            .map(|(path, line)| (*path, *line, "env_declaration"))
            .unwrap_or((rel, reference_line, "static_env_reference"));
        out.declarations.push(entity_surface(
            anchor.clone(),
            "config_key",
            entity_path,
            entity_line,
            evidence,
        ));
        out.edges.push(structural_edge_with_locations(
            rel.to_string(),
            anchor.clone(),
            "reads_config",
            config.evidence,
            config.strength,
            config.locations,
        ));
        if let Some((path, line)) = declaration {
            out.edges.push(structural_edge_with_locations(
                path.to_string(),
                anchor,
                "declares",
                "env_declaration",
                EvidenceStrength::Hard,
                vec![EvidenceLocation::line(path, line, "env_declaration")],
            ));
        }
    }
}

fn owner_consumers(
    project: &Project,
    owner: &str,
    relevant_symbols: &BTreeSet<String>,
) -> Vec<StructuralEdge> {
    let mut edges = direct_consumer_edges(project, owner);
    let Some(package) = package_for_rel(project, owner) else {
        return edges;
    };
    let owner_is_public_index = owner
        .strip_prefix(&format!("{}/", package.path.trim_end_matches('/')))
        .is_some_and(|path| {
            matches!(
                path,
                "src/index.ts" | "src/index.js" | "index.ts" | "index.js"
            )
        });
    if !owner_is_public_index {
        return edges;
    }
    if !relevant_symbols.is_empty() {
        edges.retain(|edge| {
            project.files.get(&edge.from).is_some_and(|file| {
                relevant_symbols
                    .iter()
                    .any(|symbol| file.references.contains(symbol))
            })
        });
    }
    for file in project.files.values().filter(|file| {
        file.rel != owner
            && file.imports.iter().any(|spec| {
                spec == &package.name || spec.starts_with(&format!("{}/", package.name))
            })
            && (relevant_symbols.is_empty()
                || relevant_symbols
                    .iter()
                    .any(|symbol| file.references.contains(symbol)))
    }) {
        edges.push(structural_edge_with_locations(
            file.rel.clone(),
            owner.to_string(),
            "consumes",
            "package_public_import",
            EvidenceStrength::High,
            vec![EvidenceLocation::line(
                &file.rel,
                owner_line_containing(project, &file.rel, &[&package.name]),
                "package_import",
            )],
        ));
    }
    crate::map::sort_edges(&mut edges);
    edges
}

fn add_direct_proof(project: &Project, owner: &str, proof: &mut Vec<StructuralEdge>) {
    for mut edge in cone_proof_edges(project, &[owner.to_string()])
        .into_iter()
        .filter(|edge| {
            edge.strength == EvidenceStrength::High
                && matches!(
                    edge.evidence.as_str(),
                    "test_import" | "test_support_import" | "e2e_route"
                )
        })
    {
        edge.edge_type = if edge.evidence == "test_import" {
            "verifies_directly".to_string()
        } else {
            "verifies_through".to_string()
        };
        proof.push(edge);
    }
}

fn add_topics(project: &Project, rel: &str, out: &mut ConnectedLineage) {
    let Some(text) = project.read_indexed_text(rel) else {
        return;
    };
    for (line_number, line) in runtime_code_lines(&text) {
        let Some(start) = line.find("topic:") else {
            continue;
        };
        let tail = line[start + "topic:".len()..].trim_start();
        if let Some(topic) = quoted_literal_at(tail) {
            let anchor = format!("topic:{topic}");
            out.declarations.push(entity_surface(
                anchor.clone(),
                "event_channel",
                rel,
                line_number,
                "static_topic_literal",
            ));
            out.edges.push(structural_edge_with_locations(
                rel.to_string(),
                anchor,
                "emits",
                "static_topic_literal",
                EvidenceStrength::High,
                vec![EvidenceLocation::line(
                    rel,
                    line_number,
                    "topic_declaration",
                )],
            ));
        } else if tail.contains('(')
            || tail.contains("${")
            || tail.contains('[')
            || tail
                .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '}'))
                .next()
                .is_some_and(|value| value.contains('.'))
        {
            out.unknowns.push(unknown(
                "computed_topic",
                Some(rel),
                Some(line_number),
                "event topic is computed instead of a static literal",
                "topic lineage stops instead of inventing an event channel",
                Some(format!("codemap cone {rel}")),
            ));
        }
    }
}
