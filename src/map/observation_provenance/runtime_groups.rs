// Responsibility: runtime-group-observation-provenance
use crate::model::{
    CoverageCertificate, CoverageClosure, CoverageLocation, CoverageReason, CoverageStop,
    ExtractorCapability, FileInfo, ObservationLedger, Project, Unknown,
};

use super::runtime_group_gaps::{env_certificate, scripts_certificate, unknowns_certificate};

/// Visibility already decided by runtime report assembly for one fact group.
pub(crate) struct RuntimeGroupVisibility {
    pub observed: usize,
    pub shown: usize,
    pub expand: Option<String>,
}

/// Exact inputs owned by runtime report assembly. This projection certifies
/// what each remaining runtime group observed and chose to show; it never
/// rediscovers scope or recomputes group facts.
pub(crate) struct RuntimeGroupObservationInput<'a> {
    pub scope: &'a str,
    pub scope_indexed: bool,
    pub scope_files: &'a [&'a FileInfo],
    pub hidden_scope_count: usize,
    pub scope_unknowns: &'a [Unknown],
    pub entrypoints: RuntimeGroupVisibility,
    pub scripts: RuntimeGroupVisibility,
    pub env: RuntimeGroupVisibility,
    pub workers: RuntimeGroupVisibility,
    pub ci: RuntimeGroupVisibility,
    pub proof: RuntimeGroupVisibility,
    pub unknowns: RuntimeGroupVisibility,
}

pub(crate) fn record_runtime_group_observations(
    project: &Project,
    input: RuntimeGroupObservationInput<'_>,
    ledger: &mut ObservationLedger,
) {
    if !input.scope_indexed {
        for group in RUNTIME_OBSERVATION_GROUPS {
            let certificate = CoverageCertificate::new(
                query_kind_for(group),
                input.scope,
                crate::cache::fingerprint(project, None),
                0,
                0,
                CoverageClosure::Unavailable,
                vec![CoverageReason::AnchorNotIndexed],
            );
            record_group(&input, ledger, group, certificate);
        }
        return;
    }

    record_group(
        &input,
        ledger,
        "entrypoints",
        entrypoints_certificate(project, &input),
    );
    record_group(
        &input,
        ledger,
        "scripts",
        scripts_certificate(project, &input),
    );
    record_group(&input, ledger, "env", env_certificate(project, &input));
    record_group(
        &input,
        ledger,
        "workers",
        inventory_certificate(project, &input, "workers"),
    );
    record_group(
        &input,
        ledger,
        "ci",
        inventory_certificate(project, &input, "ci"),
    );
    record_group(&input, ledger, "proof", proof_certificate(project, &input));
    record_group(
        &input,
        ledger,
        "unknowns",
        unknowns_certificate(project, &input),
    );
}

const RUNTIME_OBSERVATION_GROUPS: [&str; 7] = [
    "entrypoints",
    "scripts",
    "env",
    "workers",
    "ci",
    "proof",
    "unknowns",
];

fn record_group(
    input: &RuntimeGroupObservationInput<'_>,
    ledger: &mut ObservationLedger,
    group: &str,
    certificate: CoverageCertificate,
) {
    let visibility = match group {
        "entrypoints" => &input.entrypoints,
        "scripts" => &input.scripts,
        "env" => &input.env,
        "workers" => &input.workers,
        "ci" => &input.ci,
        "proof" => &input.proof,
        _ => &input.unknowns,
    };
    ledger.record(
        group,
        input.scope,
        visibility.observed as u64,
        visibility.shown as u64,
        certificate,
        visibility.expand.clone(),
    );
}

pub(super) fn query_kind_for(group: &str) -> &'static str {
    match group {
        "entrypoints" => "runtime_entrypoint_surfaces",
        "scripts" => "runtime_script_catalog",
        "env" => "runtime_env_surfaces",
        "workers" => "runtime_worker_job_surfaces",
        "ci" => "runtime_ci_surfaces",
        "proof" => "runtime_verification_edges",
        _ => "runtime_unknown_boundaries",
    }
}

pub(super) fn base_certificate(
    project: &Project,
    input: &RuntimeGroupObservationInput<'_>,
    group: &str,
    eligible: u64,
    visited: u64,
    reasons: Vec<CoverageReason>,
) -> CoverageCertificate {
    CoverageCertificate::new(
        query_kind_for(group),
        input.scope,
        crate::cache::fingerprint(project, None),
        eligible,
        visited,
        if reasons.is_empty() {
            CoverageClosure::Closed
        } else {
            CoverageClosure::Open
        },
        reasons,
    )
}

pub(super) fn capability(id: &str, language: &str, constructs: &[&str]) -> ExtractorCapability {
    ExtractorCapability {
        extractor_id: id.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        language: language.to_string(),
        constructs: constructs.iter().map(|item| item.to_string()).collect(),
    }
}

pub(super) fn root_scope_trim_stop(
    input: &RuntimeGroupObservationInput<'_>,
) -> Option<CoverageStop> {
    (input.hidden_scope_count > 0).then(|| CoverageStop {
        kind: CoverageReason::IncompleteTraversal,
        location: Some(CoverageLocation::path(input.scope)),
        missing_surface: Some(format!(
            "{} recursive runtime files hidden at root scope",
            input.hidden_scope_count
        )),
    })
}

/// Code-level entrypoint extraction only understands Rust Clap subcommands;
/// entrypoints in other languages stay outside the declared grammar, so the
/// horizon is a typed open lower bound, never a proven-complete set.
fn entrypoints_certificate(
    project: &Project,
    input: &RuntimeGroupObservationInput<'_>,
) -> CoverageCertificate {
    let files = input.scope_files.len() as u64;
    let mut certificate = base_certificate(
        project,
        input,
        "entrypoints",
        files,
        files,
        vec![CoverageReason::UnsupportedConstruct],
    );
    certificate.extractor_capabilities = vec![capability(
        "codemap.runtime-entrypoints",
        "multi",
        &[
            "manifest_cli_entrypoint",
            "runtime_container_manifest",
            "runtime_entrypoint_file_convention",
            "rust_clap_subcommand_enum",
        ],
    )];
    certificate
        .unresolved_stops
        .extend(root_scope_trim_stop(input));
    certificate
}

/// Workers and CI groups are exact path/role projections over the indexed
/// candidate inventory, so their zero closes only when every eligible file was
/// actually visited and nothing was trimmed away by the root scope.
fn inventory_certificate(
    project: &Project,
    input: &RuntimeGroupObservationInput<'_>,
    group: &str,
) -> CoverageCertificate {
    let files = input.scope_files.len() as u64;
    let mut certificate = base_certificate(project, input, group, files, files, Vec::new());
    certificate.extractor_capabilities = vec![if group == "workers" {
        capability(
            "codemap.runtime-worker-jobs",
            "path",
            &["worker_job_path_convention"],
        )
    } else {
        capability("codemap.runtime-ci-roles", "path", &["build_ci_role"])
    }];
    certificate
        .unresolved_stops
        .extend(root_scope_trim_stop(input));
    certificate
}

/// Verification edges tie observed routes to e2e sensors through a partial
/// relation extractor; unresolved route/verification relations keep this
/// horizon a typed open lower bound.
fn proof_certificate(
    project: &Project,
    input: &RuntimeGroupObservationInput<'_>,
) -> CoverageCertificate {
    let mut certificate = base_certificate(
        project,
        input,
        "proof",
        0,
        0,
        vec![CoverageReason::VerificationRelationFlow],
    );
    certificate.extractor_capabilities = vec![capability(
        "codemap.runtime-verification",
        "multi",
        &["e2e_visited_route"],
    )];
    certificate
}
