// Responsibility: observed-verification-topology
use crate::model::{
    EvidenceLocation, EvidenceStrength, HiddenGroup, Project, ProofSurface, ProofWiringFact,
    Surface, Unknown, VerificationHorizon, VerificationRelation, VerificationTopology,
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, VecDeque};

pub(crate) struct VerificationTopologyInput<'a> {
    pub project: &'a Project,
    pub proofs: &'a [ProofSurface],
    pub missing: &'a [Surface],
    pub wiring: &'a [ProofWiringFact],
    pub unknowns: &'a [Unknown],
    pub hidden: &'a [HiddenGroup],
    pub expand: &'a [String],
}

pub(crate) fn verification_topology(input: VerificationTopologyInput<'_>) -> VerificationTopology {
    let mut topology = VerificationTopology::default();
    for proof in input.proofs {
        push_proof_relation(input.project, proof, &mut topology);
        if let Some(command) = proof.command.as_ref()
            && !crate::proof_classification::proof_surface_is_soft_evidence(proof)
            && !crate::proof_classification::proof_surface_is_setup_or_support(proof)
        {
            topology.runnable.push(relation(
                "contains_sensor",
                command,
                proof.path.as_deref().unwrap_or(&proof.evidence),
                command_path(command, proof.path.as_deref()),
                &proof.evidence,
                proof.strength,
                proof.locations.clone(),
            ));
        }
    }
    for surface in input.missing {
        let object = surface.path.as_deref().unwrap_or(&surface.id);
        topology.missing_link.push(relation(
            "missing_link",
            object,
            object,
            vec![object.to_string()],
            &surface.evidence,
            surface.strength,
            Vec::new(),
        ));
    }
    push_unknown_relations(input.unknowns, input.missing.is_empty(), &mut topology);
    for fact in input.wiring {
        push_wiring_relation(fact, &mut topology);
    }
    sort_and_dedupe(&mut topology);
    topology.horizon = topology_horizon(&topology, input.hidden, input.expand);
    topology
}

fn push_unknown_relations(
    unknowns: &[Unknown],
    include_missing: bool,
    out: &mut VerificationTopology,
) {
    for unknown in unknowns {
        let subject = unknown.path.as_deref().unwrap_or("verification_scope");
        let locations = unknown
            .line_start
            .map(|line| EvidenceLocation::line(subject, line, "unknown_boundary"))
            .into_iter()
            .collect();
        if include_missing
            && matches!(
                unknown.kind.as_str(),
                "direct_test_import_not_found" | "missing_deterministic_proof"
            )
        {
            out.missing_link.push(relation(
                "missing_link",
                subject,
                subject,
                vec![subject.to_string()],
                &unknown.kind,
                EvidenceStrength::Medium,
                locations,
            ));
        } else if unknown.kind.contains("dynamic")
            || unknown.kind.contains("external")
            || unknown.kind.contains("unresolved")
        {
            out.unknown_external.push(relation(
                "unknown_external",
                subject,
                subject,
                vec![subject.to_string()],
                &unknown.kind,
                EvidenceStrength::Medium,
                locations,
            ));
        }
    }
}

pub(crate) fn unavailable_verification_topology(
    reason: &str,
    expand: Vec<String>,
) -> VerificationTopology {
    let certificate_id = format!("verification-v1:{:x}", Sha256::digest(reason.as_bytes()));
    VerificationTopology {
        horizon: VerificationHorizon {
            status: "open".to_string(),
            reasons: vec![reason.to_string()],
            certificate_id,
            expand,
            ..VerificationHorizon::default()
        },
        ..VerificationTopology::default()
    }
}

fn push_proof_relation(project: &Project, proof: &ProofSurface, out: &mut VerificationTopology) {
    let subject = proof
        .target_anchor
        .as_deref()
        .unwrap_or("verification_scope");
    let object = proof.path.as_deref().unwrap_or(&proof.evidence);
    let path = observed_proof_path(project, proof, subject, object);
    if crate::proof_classification::proof_surface_is_setup_or_support(proof) {
        out.support.push(relation(
            "supports_verification",
            subject,
            object,
            path,
            &proof.evidence,
            proof.strength,
            proof.locations.clone(),
        ));
    } else if crate::proof_classification::proof_surface_is_soft_evidence(proof) {
        out.soft_related.push(relation(
            "related_soft",
            subject,
            object,
            path,
            &proof.evidence,
            proof.strength,
            proof.locations.clone(),
        ));
    } else if crate::proof_classification::proof_surface_is_mediated_evidence(proof) {
        out.mediated.push(relation(
            "verifies_via",
            subject,
            object,
            path,
            &proof.evidence,
            proof.strength,
            proof.locations.clone(),
        ));
    } else if topology_evidence_is_direct(&proof.evidence) {
        out.direct.push(relation(
            "verifies_directly",
            subject,
            object,
            path,
            &proof.evidence,
            proof.strength,
            proof.locations.clone(),
        ));
    } else {
        out.soft_related.push(relation(
            "related_soft",
            subject,
            object,
            path,
            &proof.evidence,
            proof.strength,
            proof.locations.clone(),
        ));
    }
}

fn topology_evidence_is_direct(evidence: &str) -> bool {
    matches!(
        crate::proof_classification::proof_base_evidence(evidence),
        "test_import"
            | "test_imported_symbol_reference"
            | "test_reexported_symbol_reference"
            | "e2e_route"
            | "current_level_proof_container"
    )
}

fn push_wiring_relation(fact: &ProofWiringFact, out: &mut VerificationTopology) {
    if fact.stage == "invokes_process" && fact.status == "wired" {
        let object = fact.path.as_deref().unwrap_or("external_process");
        out.runnable.push(relation(
            "invokes_process",
            &fact.subject,
            object,
            command_path(&fact.subject, fact.path.as_deref()),
            &fact.evidence,
            EvidenceStrength::High,
            fact.locations.clone(),
        ));
    } else if fact.stage == "invokes_process" || fact.status == "unknown" {
        let object = fact
            .path
            .as_deref()
            .unwrap_or("unresolved_external_process");
        out.unknown_external.push(relation(
            "unknown_external",
            &fact.subject,
            object,
            command_path(&fact.subject, fact.path.as_deref()),
            &fact.evidence,
            EvidenceStrength::Medium,
            fact.locations.clone(),
        ));
    }
}

fn observed_proof_path(
    project: &Project,
    proof: &ProofSurface,
    subject: &str,
    object: &str,
) -> Vec<String> {
    let anchor = subject.split_once('#').map_or(subject, |(path, _)| path);
    if crate::proof_classification::proof_surface_is_mediated_evidence(proof)
        && let Some(path) = import_path(project, anchor, object, 4)
    {
        return path;
    }
    if subject == object {
        vec![subject.to_string()]
    } else {
        vec![subject.to_string(), object.to_string()]
    }
}

fn import_path(
    project: &Project,
    anchor: &str,
    test: &str,
    max_hops: usize,
) -> Option<Vec<String>> {
    let mut queue = VecDeque::from([(test.to_string(), vec![test.to_string()])]);
    let mut seen = BTreeSet::from([test.to_string()]);
    while let Some((current, path)) = queue.pop_front() {
        if path.len() > max_hops + 1 {
            continue;
        }
        let file = project.files.get(&current)?;
        for dependency in &file.resolved_imports {
            let mut next_path = path.clone();
            next_path.push(dependency.clone());
            if dependency == anchor {
                next_path.reverse();
                return Some(next_path);
            }
            if seen.insert(dependency.clone()) {
                queue.push_back((dependency.clone(), next_path));
            }
        }
    }
    None
}

fn command_path(command: &str, object: Option<&str>) -> Vec<String> {
    let mut path = vec![command.to_string()];
    if let Some(object) = object
        && object != command
    {
        path.push(object.to_string());
    }
    path
}

fn relation(
    kind: &str,
    subject: &str,
    object: &str,
    path: Vec<String>,
    evidence: &str,
    strength: EvidenceStrength,
    locations: Vec<EvidenceLocation>,
) -> VerificationRelation {
    VerificationRelation {
        relation: kind.to_string(),
        subject: subject.to_string(),
        object: object.to_string(),
        path,
        evidence: evidence.to_string(),
        strength,
        locations,
    }
}

fn sort_and_dedupe(topology: &mut VerificationTopology) {
    for relations in [
        &mut topology.direct,
        &mut topology.mediated,
        &mut topology.runnable,
        &mut topology.soft_related,
        &mut topology.support,
        &mut topology.missing_link,
        &mut topology.unknown_external,
    ] {
        relations.sort_by(|a, b| {
            a.relation
                .cmp(&b.relation)
                .then_with(|| a.subject.cmp(&b.subject))
                .then_with(|| a.object.cmp(&b.object))
                .then_with(|| a.path.cmp(&b.path))
        });
        relations.dedup_by(|a, b| {
            a.relation == b.relation
                && a.subject == b.subject
                && a.object == b.object
                && a.path == b.path
        });
    }
}

fn topology_horizon(
    topology: &VerificationTopology,
    hidden: &[HiddenGroup],
    expand: &[String],
) -> VerificationHorizon {
    let shown = topology_relation_count(topology);
    let hidden_count = hidden
        .iter()
        .filter(|group| group.reason.contains("verification") || group.reason.contains("proof"))
        .map(|group| group.count)
        .sum::<usize>();
    let mut reasons = Vec::new();
    if hidden_count > 0 {
        reasons.push("bounded_visible_topology".to_string());
    }
    if !topology.unknown_external.is_empty() {
        reasons.push("external_runtime_boundary".to_string());
    }
    if reasons.is_empty() {
        reasons.push("observed_topology_closed_within_indexed_scope".to_string());
    }
    let status = if hidden_count == 0 && topology.unknown_external.is_empty() {
        "closed"
    } else {
        "open"
    };
    let body = serde_json::to_vec(&(
        status,
        shown,
        hidden_count,
        &reasons,
        topology
            .direct
            .iter()
            .chain(&topology.mediated)
            .chain(&topology.runnable)
            .chain(&topology.soft_related)
            .chain(&topology.support)
            .chain(&topology.missing_link)
            .chain(&topology.unknown_external)
            .map(|item| (&item.relation, &item.subject, &item.object, &item.path))
            .collect::<Vec<_>>(),
    ))
    .unwrap_or_default();
    VerificationHorizon {
        status: status.to_string(),
        observed: shown + hidden_count,
        shown,
        hidden: hidden_count,
        unknown_external: topology.unknown_external.len(),
        reasons,
        certificate_id: format!("verification-v1:{:x}", Sha256::digest(body)),
        expand: expand.to_vec(),
    }
}

fn topology_relation_count(topology: &VerificationTopology) -> usize {
    topology.direct.len()
        + topology.mediated.len()
        + topology.runnable.len()
        + topology.soft_related.len()
        + topology.support.len()
        + topology.missing_link.len()
        + topology.unknown_external.len()
}
