// Responsibility: changed-proof-sensor-counts
use crate::model::ChangedReport;

pub(crate) fn changed_proof_sensor_counts(report: &ChangedReport, _compact: bool) {
    let counts = changed_proof_public_sensor_counts(report);
    println!(
        "- sensor counts: runnable_direct=`{}`; soft=`{}`; evidence_only=`{}`; setup_support=`{}`; missing_direct_unknown=`{}`",
        counts.runnable_direct,
        counts.soft,
        counts.evidence_only,
        counts.setup_support,
        counts.missing_direct_unknown
    );
}

struct ChangedProofPublicSensorCounts {
    runnable_direct: usize,
    soft: usize,
    evidence_only: usize,
    setup_support: usize,
    missing_direct_unknown: usize,
}

fn changed_proof_public_sensor_counts(report: &ChangedReport) -> ChangedProofPublicSensorCounts {
    let sensors = report
        .proof
        .hard
        .iter()
        .chain(report.proof.direct_evidence.iter())
        .chain(report.proof.mediated_evidence.iter())
        .chain(report.proof.soft_evidence.iter())
        .chain(report.proof.setup_support.iter())
        .collect::<Vec<_>>();
    let runnable_direct = sensors
        .iter()
        .filter(|sensor| crate::proof_classification::proof_surface_is_runnable_validation(sensor))
        .count();
    let soft = sensors
        .iter()
        .filter(|sensor| crate::proof_classification::proof_surface_is_soft_evidence(sensor))
        .count();
    let evidence_only = sensors
        .iter()
        .filter(|sensor| crate::proof_classification::proof_surface_is_evidence_only(sensor))
        .count();
    let setup_support = sensors
        .iter()
        .filter(|sensor| crate::proof_classification::proof_surface_is_setup_or_support(sensor))
        .count();
    let unknown_direct = report
        .unknowns
        .iter()
        .filter(|unknown| unknown.kind == "direct_test_import_not_found")
        .count();
    ChangedProofPublicSensorCounts {
        runnable_direct,
        soft,
        evidence_only,
        setup_support,
        missing_direct_unknown: unknown_direct.max(report.proof.missing_direct.len()),
    }
}
