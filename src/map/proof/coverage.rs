// Responsibility: map-proof-coverage
use crate::map::shell_quote;
use crate::model::{ProofCoverageSummary, ProofCoveredPath, ProofGap, ProofSurface};
use std::collections::BTreeMap;
use std::collections::BTreeSet;

pub(crate) fn proof_coverage_summary(
    changed: &[String],
    proofs_by_anchor: &BTreeMap<String, Vec<ProofSurface>>,
) -> ProofCoverageSummary {
    let mut summary = ProofCoverageSummary {
        changed_count: changed.len(),
        runnable_deterministic: Vec::new(),
        evidence_only: Vec::new(),
        setup_support_only: Vec::new(),
        soft_only: Vec::new(),
        missing: Vec::new(),
    };
    for path in changed {
        let proofs = proofs_by_anchor.get(path).map(Vec::as_slice).unwrap_or(&[]);
        let runnable = proofs
            .iter()
            .filter(|proof| {
                crate::proof_classification::proof_surface_is_runnable_validation(proof)
            })
            .collect::<Vec<_>>();
        if !runnable.is_empty() {
            summary
                .runnable_deterministic
                .push(proof_covered_path(path, &runnable));
            continue;
        }
        let evidence_only = proofs
            .iter()
            .filter(|proof| crate::proof_classification::proof_surface_is_evidence_only(proof))
            .collect::<Vec<_>>();
        if !evidence_only.is_empty() {
            summary
                .evidence_only
                .push(proof_covered_path(path, &evidence_only));
            continue;
        }
        let setup = proofs
            .iter()
            .filter(|proof| crate::proof_classification::proof_surface_is_setup_or_support(proof))
            .collect::<Vec<_>>();
        if !setup.is_empty() {
            summary
                .setup_support_only
                .push(proof_covered_path(path, &setup));
            continue;
        }
        let soft = proofs
            .iter()
            .filter(|proof| crate::proof_classification::proof_surface_is_soft_evidence(proof))
            .collect::<Vec<_>>();
        if !soft.is_empty() {
            summary.soft_only.push(proof_covered_path(path, &soft));
            continue;
        }
        summary.missing.push(ProofGap {
            path: path.clone(),
            kind: "direct_deterministic_proof_not_found".to_string(),
            effect: "no runnable command, linked, setup/support, or soft-match verification surface was found for this changed path".to_string(),
            expand: format!("codemap proof-map --files {} --raw-sensors", shell_quote(path)),
        });
    }
    summary
}

fn proof_covered_path(path: &str, proofs: &[&ProofSurface]) -> ProofCoveredPath {
    let evidence = proofs
        .iter()
        .map(|proof| proof.evidence.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let commands = proofs
        .iter()
        .map(|proof| {
            proof
                .command
                .clone()
                .unwrap_or_else(|| proof.evidence.clone())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    ProofCoveredPath {
        path: path.to_string(),
        sensor_count: proofs.len(),
        evidence,
        commands,
    }
}
