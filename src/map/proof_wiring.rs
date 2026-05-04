fn proof_wiring_facts(
    project: &Project,
    anchors: &[String],
    changed: &[String],
    proofs: &[ProofSurface],
    fallback: &[String],
) -> Vec<ProofWiringFact> {
    let mut facts = Vec::new();
    let mut seen = BTreeSet::new();
    let selector = proof_wiring_selector(anchors, changed);
    for proof in proofs {
        push_unique_proof_wiring(
            &mut facts,
            &mut seen,
            proof_surface_wiring_facts(project, proof, &selector),
        );
    }
    for anchor in anchors {
        push_unique_proof_wiring(
            &mut facts,
            &mut seen,
            artifact_contract_wiring_facts(project, anchor, &selector),
        );
    }
    for command in fallback {
        let proof = ProofSurface {
            command: Some(command.clone()),
            path: None,
            evidence: "fallback_command".to_string(),
            strength: EvidenceStrength::Medium,
            reason: "fallback command is broad structural proof, not direct proof".to_string(),
            locations: vec![EvidenceLocation::aggregate("fallback_command")],
        };
        push_unique_proof_wiring(
            &mut facts,
            &mut seen,
            proof_surface_wiring_facts(project, &proof, &selector),
        );
    }
    sort_proof_wiring_facts(&mut facts);
    facts
}

type ProofWiringKey = (String, String, String, Option<String>, String);

fn push_unique_proof_wiring(
    facts: &mut Vec<ProofWiringFact>,
    seen: &mut BTreeSet<ProofWiringKey>,
    incoming: Vec<ProofWiringFact>,
) {
    for fact in incoming {
        let key = (
            fact.stage.clone(),
            fact.status.clone(),
            fact.subject.clone(),
            fact.path.clone(),
            fact.effect.clone(),
        );
        if seen.insert(key) {
            facts.push(fact);
        }
    }
}

fn sort_proof_wiring_facts(facts: &mut [ProofWiringFact]) {
    facts.sort_by(|a, b| {
        proof_wiring_status_rank(&a.status)
            .cmp(&proof_wiring_status_rank(&b.status))
            .then_with(|| a.stage.cmp(&b.stage))
            .then_with(|| a.subject.cmp(&b.subject))
            .then_with(|| a.path.cmp(&b.path))
    });
}

fn proof_wiring_unknowns(facts: &[ProofWiringFact]) -> Vec<Unknown> {
    let mut unknowns = Vec::new();
    for fact in facts {
        if matches!(fact.status.as_str(), "missing" | "unknown") {
            unknowns.push(unknown(
                proof_wiring_unknown_kind(fact),
                fact.path.as_deref(),
                fact.locations.first().and_then(|location| location.line_start),
                &fact.evidence,
                &fact.effect,
                fact.expand.clone(),
            ));
        }
    }
    unknowns
}

fn proof_wiring_selector(anchors: &[String], changed: &[String]) -> String {
    if let Some(anchor) = anchors.first()
        && anchors.len() == 1
        && changed.is_empty()
    {
        return shell_quote(anchor);
    }
    if !changed.is_empty() {
        return "changed".to_string();
    }
    "changed".to_string()
}

fn proof_surface_wiring_facts(
    project: &Project,
    proof: &ProofSurface,
    selector: &str,
) -> Vec<ProofWiringFact> {
    let mut facts = Vec::new();
    let subject = proof
        .command
        .clone()
        .unwrap_or_else(|| proof.evidence.clone());
    if crate::proof_classification::proof_surface_is_soft_evidence(proof) {
        facts.push(proof_wiring_fact(
            ("declared_command", "soft"),
            subject,
            proof.path.clone(),
            "proof surface relation is token/name/path overlap",
            "soft evidence does not close deterministic proof wiring or remove Unknown entries",
            proof.locations.clone(),
            proof_wiring_expand_for_proof(selector, proof),
        ));
        return facts;
    }
    if proof.command.is_none() {
        facts.push(proof_wiring_fact(
            ("declared_command", "missing"),
            subject,
            proof.path.clone(),
            "proof surface has no runnable command",
            "this remains evidence only; it does not close deterministic proof wiring",
            proof.locations.clone(),
            proof_wiring_expand_for_proof(selector, proof),
        ));
        return facts;
    }
    let command = proof.command.as_ref().expect("checked");
    facts.push(proof_wiring_fact(
        ("declared_command", "wired"),
        command.clone(),
        proof.path.clone(),
        "proof surface declares a command",
        "command declaration is visible, but later wiring stages still decide whether it resolves",
        proof.locations.clone(),
        proof_wiring_expand_for_proof(selector, proof),
    ));
    facts.push(resolve_proof_command_fact(project, command, proof, selector));
    if proof_wiring_needs_artifact_chain(project, proof) || command_mentions_artifact(command) {
        facts.extend(artifact_chain_facts_for_command(project, command, proof, selector));
    }
    facts
}

include!("proof_wiring_commands.rs");
include!("proof_wiring_artifacts.rs");
include!("proof_wiring_artifact_paths.rs");
include!("proof_wiring_helpers.rs");
