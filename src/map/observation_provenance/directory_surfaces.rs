// Responsibility: nested-directory-ls-surface-observation-provenance
use crate::map::{ObservationProjection, path_under_scope};
use crate::model::{
    CoverageCertificate, CoverageClosure, CoverageLocation, CoverageReason, ExtractorCapability,
    ObservationLedger, Project, UnsupportedObservation,
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn directory_surface_observations(
    project: &Project,
    groups: ObservationProjection<'_>,
    members: ObservationProjection<'_>,
) -> ObservationLedger {
    let basis = surface_candidate_basis(project, groups.scope);
    let mut ledger = ObservationLedger::default();
    ledger.record(
        groups.group,
        groups.scope,
        groups.observed as u64,
        groups.shown as u64,
        basis.certificate("nested_directory_surface_groups"),
        groups.expand,
    );
    ledger.record(
        members.group,
        members.scope,
        members.observed as u64,
        members.shown as u64,
        basis.certificate("nested_directory_surface_members"),
        members.expand,
    );
    ledger
}

struct SurfaceCandidateBasis {
    scope: String,
    snapshot: String,
    eligible_files: u64,
    visited_files: u64,
    closure: CoverageClosure,
    reasons: Vec<CoverageReason>,
    excluded: BTreeMap<CoverageReason, Vec<String>>,
    unsupported: Vec<UnsupportedObservation>,
    capabilities: Vec<ExtractorCapability>,
}

impl SurfaceCandidateBasis {
    fn certificate(&self, query_kind: &str) -> CoverageCertificate {
        let mut certificate = CoverageCertificate::new(
            query_kind,
            &self.scope,
            &self.snapshot,
            self.eligible_files,
            self.visited_files,
            self.closure,
            self.reasons.clone(),
        );
        certificate.excluded_files_by_reason = self.excluded.clone();
        certificate.unsupported = self.unsupported.clone();
        certificate.extractor_capabilities = self.capabilities.clone();
        certificate
    }
}

fn surface_candidate_basis(project: &Project, scope: &str) -> SurfaceCandidateBasis {
    let package_audit = crate::repo::audit_package_discovery(&project.root, &project.files);
    let package_gaps = package_audit
        .unsupported
        .iter()
        .filter(|gap| path_under_scope(&gap.manifest, scope))
        .map(|gap| (gap.manifest.as_str(), gap.construct))
        .collect::<BTreeMap<_, _>>();
    let candidates = project
        .files
        .values()
        .filter(|file| path_under_scope(&file.rel, scope))
        .collect::<Vec<_>>();
    let mut visited_files = 0_u64;
    let mut reasons = Vec::new();
    let mut excluded = BTreeMap::<CoverageReason, Vec<String>>::new();
    let mut unsupported = Vec::new();
    let mut capabilities = BTreeSet::new();
    for file in &candidates {
        let unavailable = file
            .content_hash
            .is_none()
            .then_some("indexed file body is unavailable");
        let gap = package_gaps.get(file.rel.as_str()).copied().or(unavailable);
        if let Some(construct) = gap {
            reasons.push(CoverageReason::UnsupportedConstruct);
            excluded
                .entry(CoverageReason::UnsupportedConstruct)
                .or_default()
                .push(file.rel.clone());
            unsupported.push(UnsupportedObservation {
                file: file.rel.clone(),
                construct: construct.to_string(),
                location: Some(CoverageLocation::path(&file.rel)),
            });
            continue;
        }
        visited_files += 1;
        capabilities.insert(ExtractorCapability {
            extractor_id: "codemap.directory-surface-classification".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            language: file.language.clone(),
            constructs: vec![
                "current_level_directory".to_string(),
                "path_role".to_string(),
                "recursive_role_group".to_string(),
            ],
        });
    }
    SurfaceCandidateBasis {
        scope: scope.to_string(),
        snapshot: crate::cache::fingerprint(project, None),
        eligible_files: candidates.len() as u64,
        visited_files,
        closure: CoverageClosure::from_gaps(!reasons.is_empty()),
        reasons,
        excluded,
        unsupported,
        capabilities: capabilities.into_iter().collect(),
    }
}
