// Responsibility: map-cone-env-surfaces
use crate::map::{
    env_ci_reference_proof_surfaces, env_declared_keys, line_may_contain_static_env_reference,
    owner_proof_surface_edge, prisma_env_names, shell_quote, static_env_names,
    structural_edge_with_locations, unique_proof_surfaces, unknown,
};
use crate::model::{
    EnvDeclaration, EvidenceLocation, EvidenceStrength, Project, ProofSurface, StructuralEdge,
    Unknown,
};
use std::collections::BTreeSet;

pub(crate) fn cone_declared_env(project: &Project, rel: &str) -> Vec<EnvDeclaration> {
    let Some(file) = project.files.get(rel) else {
        return Vec::new();
    };
    if !file.has_role("env_config") {
        return Vec::new();
    }
    env_declared_keys(project, rel)
        .into_iter()
        .map(|(key, line_start)| EnvDeclaration {
            key,
            path: rel.to_string(),
            line_start,
        })
        .collect()
}

#[derive(Debug, Clone)]
pub(crate) struct OwnerEnvFacts {
    pub(crate) keys: Vec<(String, usize)>,
    pub(crate) consumers: Vec<EnvConsumerReference>,
}

#[derive(Debug, Clone)]
pub(crate) struct EnvConsumerReference {
    key: String,
    path: String,
    line_start: usize,
}

pub(crate) fn file_is_env_config(project: &Project, rel: &str) -> bool {
    project
        .files
        .get(rel)
        .is_some_and(|file| file.has_role("env_config"))
}

pub(crate) fn owner_env_facts(project: &Project, rel: &str) -> OwnerEnvFacts {
    let keys = env_declared_keys(project, rel);
    let key_set = keys
        .iter()
        .map(|(key, _)| key.clone())
        .collect::<BTreeSet<_>>();
    if key_set.is_empty() {
        return OwnerEnvFacts {
            keys,
            consumers: Vec::new(),
        };
    }
    let mut consumers = Vec::new();
    for file in project.files.values() {
        if file.rel == rel
            || file.has_role("generated")
            || file.has_role("fixture")
            || file.has_role("archive")
        {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(project.root.join(&file.rel)) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            if !line_may_contain_static_env_reference(line) {
                continue;
            }
            let mut names = static_env_names(line);
            names.extend(prisma_env_names(line));
            names.sort();
            names.dedup();
            for name in names {
                if key_set.contains(&name) {
                    consumers.push(EnvConsumerReference {
                        key: name,
                        path: file.rel.clone(),
                        line_start: index + 1,
                    });
                }
            }
        }
    }
    consumers.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then_with(|| a.line_start.cmp(&b.line_start))
            .then_with(|| a.key.cmp(&b.key))
    });
    consumers.dedup_by(|a, b| a.path == b.path && a.line_start == b.line_start && a.key == b.key);
    OwnerEnvFacts { keys, consumers }
}

pub(crate) fn owner_env_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    owner_env_edges_from_facts(rel, &owner_env_facts(project, rel))
}

pub(crate) fn owner_env_edges_from_facts(rel: &str, facts: &OwnerEnvFacts) -> Vec<StructuralEdge> {
    let mut edges = facts
        .keys
        .iter()
        .map(|(key, line)| {
            structural_edge_with_locations(
                rel.to_string(),
                format!("env:{key}"),
                "declares_env",
                "env_file",
                EvidenceStrength::Hard,
                vec![EvidenceLocation::line(rel, *line, "env_declaration")],
            )
        })
        .collect::<Vec<_>>();
    for consumer in &facts.consumers {
        edges.push(structural_edge_with_locations(
            rel.to_string(),
            consumer.path.clone(),
            "env_consumer",
            "static_env_reference",
            EvidenceStrength::High,
            vec![EvidenceLocation::line(
                &consumer.path,
                consumer.line_start,
                "env_reference",
            )],
        ));
    }
    edges
}

pub(crate) fn owner_env_unknowns(project: &Project, rel: &str) -> Vec<Unknown> {
    owner_env_unknowns_from_facts(rel, &owner_env_facts(project, rel))
}

pub(crate) fn owner_env_unknowns_from_facts(rel: &str, facts: &OwnerEnvFacts) -> Vec<Unknown> {
    if facts.keys.is_empty() {
        return Vec::new();
    }
    let consumed = facts
        .consumers
        .iter()
        .map(|consumer| consumer.key.as_str())
        .collect::<BTreeSet<_>>();
    facts
        .keys
        .iter()
        .filter(|(key, _)| !consumed.contains(key.as_str()))
        .map(|(key, line)| {
            unknown(
                "env_consumer_not_found",
                Some(rel),
                Some(*line),
                format!("no static reader found for env key `{key}`"),
                "runtime config key is declared but no deterministic consumer edge was found",
                Some(format!("codemap runtime {}", shell_quote(rel))),
            )
        })
        .collect()
}

pub(crate) fn cone_owner_env_proof_edges_from_facts(
    project: &Project,
    rel: &str,
    facts: &OwnerEnvFacts,
) -> Vec<StructuralEdge> {
    let Some(file) = project.files.get(rel) else {
        return Vec::new();
    };
    let mut proofs = facts
        .consumers
        .iter()
        .map(|consumer| ProofSurface {
            command: None,
            path: Some(consumer.path.clone()),
            target_anchor: Some(file.rel.clone()),
            evidence: "env_consumer_reference".to_string(),
            strength: EvidenceStrength::High,
            reason: format!(
                "source reads env key `{}` declared in {}",
                consumer.key, file.rel
            ),
            locations: vec![EvidenceLocation::line(
                &consumer.path,
                consumer.line_start,
                "env_reference",
            )],
        })
        .collect::<Vec<_>>();
    proofs.extend(env_ci_reference_proof_surfaces(project, file));
    unique_proof_surfaces(proofs)
        .into_iter()
        .map(|proof| owner_proof_surface_edge(rel, proof))
        .collect()
}
