// Responsibility: map-proof-wiring-helpers-facts
use crate::map::shell_quote;
use crate::model::{EvidenceLocation, ProofSurface, ProofWiringFact};

pub(crate) fn proof_wiring_fact(
    state: (&str, &str),
    subject: impl Into<String>,
    path: Option<String>,
    evidence: impl Into<String>,
    effect: impl Into<String>,
    locations: Vec<EvidenceLocation>,
    expand: Option<String>,
) -> ProofWiringFact {
    let (stage, status) = state;
    ProofWiringFact {
        stage: stage.to_string(),
        status: status.to_string(),
        subject: subject.into(),
        path,
        evidence: evidence.into(),
        effect: effect.into(),
        locations,
        expand,
    }
}

pub(crate) fn proof_wiring_expand_for_proof(
    selector: &str,
    proof: &ProofSurface,
) -> Option<String> {
    proof
        .path
        .as_ref()
        .map(|path| format!("codemap cone {} --depth 2", shell_quote(path)))
        .or_else(|| Some(format!("codemap proof {selector} --section proof")))
}

pub(crate) fn proof_wiring_unknown_kind(fact: &ProofWiringFact) -> &str {
    match fact.stage.as_str() {
        "declared_command" => "proof_command_missing",
        "runner" => "proof_runner_unresolved",
        "artifact" => "artifact_write_not_found",
        "evidence_consumption" => "consumer_not_found",
        "contract_field" => "predicate_not_found",
        _ => "proof_wiring_unknown",
    }
}

pub(crate) fn proof_wiring_status_rank(status: &str) -> usize {
    match status {
        "missing" => 0,
        "unknown" => 1,
        "validated" => 2,
        "load_bearing" => 3,
        "executed" => 4,
        "wired" => 5,
        "soft" => 6,
        _ => 7,
    }
}
