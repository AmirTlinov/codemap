// Responsibility: runtime-script-observation-provenance
use std::collections::BTreeSet;

use crate::model::{
    CoverageCertificate, CoverageClosure, CoverageLocation, CoverageReason, CoverageStop, FileInfo,
    Project,
};

use super::capabilities::{script_candidate, script_capability};
use super::{RuntimeGroupObservationInput, file_certificate};

pub(super) fn script_certificate(
    project: &Project,
    input: &RuntimeGroupObservationInput<'_>,
    visited: &BTreeSet<&str>,
) -> CoverageCertificate {
    let active_root_carriers = active_root_script_carriers(input.candidate_files);
    let mut certificate = file_certificate(
        input,
        visited,
        "scripts",
        |file| active_script_candidate(file, &active_root_carriers),
        |file| script_capability(input.scope, file),
    );
    if !input.scope_logically_empty {
        certificate.closure = CoverageClosure::Open;
        certificate.unresolved_stops.push(CoverageStop {
            kind: CoverageReason::UnsupportedConstruct,
            location: Some(CoverageLocation::path(input.scope)),
            missing_surface: Some(
                "script catalog covers declared root carriers and is not an exhaustive script universe"
                    .to_string(),
            ),
        });
    }
    if input.scope == "." {
        let candidates = input
            .candidate_files
            .iter()
            .filter(|file| active_script_candidate(file, &active_root_carriers))
            .map(|file| file.rel.as_str())
            .collect::<BTreeSet<_>>();
        for script in &project.scripts {
            if script
                .path
                .as_deref()
                .is_some_and(|path| candidates.contains(path))
            {
                continue;
            }
            certificate.closure = CoverageClosure::Open;
            certificate.unresolved_stops.push(CoverageStop {
                kind: CoverageReason::IncompleteTraversal,
                location: script.path.as_deref().map(CoverageLocation::path),
                missing_surface: Some(format!(
                    "observed script `{}` is not bound to an indexed carrier",
                    script.name
                )),
            });
        }
    }
    certificate
}

fn active_root_script_carriers<'a>(files: &[&'a FileInfo]) -> BTreeSet<&'a str> {
    let mut active = BTreeSet::new();
    for precedence in [
        &["GNUmakefile", "makefile", "Makefile"][..],
        &["justfile", "Justfile"][..],
    ] {
        if let Some(file) = precedence.iter().find_map(|name| {
            files
                .iter()
                .find(|file| file.rel == *name)
                .map(|file| file.rel.as_str())
        }) {
            active.insert(file);
        }
    }
    active
}

fn active_script_candidate(file: &FileInfo, active_root_carriers: &BTreeSet<&str>) -> bool {
    if !script_candidate(file) {
        return false;
    }
    if file.rel.contains('/') {
        return true;
    }
    if matches!(
        file.rel.as_str(),
        "GNUmakefile" | "makefile" | "Makefile" | "justfile" | "Justfile"
    ) {
        return active_root_carriers.contains(file.rel.as_str());
    }
    true
}
