// Responsibility: atomic-coverage-horizon-registry
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{
    CoverageCertificate, CoverageCertificateId, CoverageStop, ObservedCount, UnsupportedObservation,
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CoverageHorizon {
    pub group: String,
    pub scope: String,
    pub count: ObservedCount,
    pub shown: u64,
    pub hidden: u64,
    pub expand: Option<String>,
    pub unsupported: Vec<UnsupportedObservation>,
    pub dynamic: Vec<CoverageStop>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct ObservationLedger {
    pub certificates: BTreeMap<CoverageCertificateId, CoverageCertificate>,
    pub horizons: Vec<CoverageHorizon>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationLedgerError {
    CertificateKeyMismatch,
    NonCanonicalCertificate,
    VisitedExceedsEligible,
    ExcludedFileAccountingMismatch,
    DanglingCertificate,
    DuplicateHorizon,
    DuplicateVisibilityAccounting,
    MissingRequiredHorizon,
    ShownFactCountMismatch,
    ObservedCountMismatch,
    ScopeMismatch,
    ClosureMismatch,
    ReasonsMismatch,
    VisibilityMismatch,
    UnsupportedMismatch,
    DynamicMismatch,
    ExpansionMismatch,
    UnreferencedCertificate,
}

impl ObservationLedger {
    /// Validates persisted ledger semantics before a report can be served.
    ///
    /// Deserialization alone only proves that the JSON has the right shape;
    /// this check restores the invariants established by `record` and `merge`.
    pub fn validate(&self) -> Result<(), ObservationLedgerError> {
        for (key, certificate) in &self.certificates {
            if key != &certificate.id {
                return Err(ObservationLedgerError::CertificateKeyMismatch);
            }
            if certificate.clone().seal() != *certificate {
                return Err(ObservationLedgerError::NonCanonicalCertificate);
            }
            if certificate.visited_files > certificate.eligible_files {
                return Err(ObservationLedgerError::VisitedExceedsEligible);
            }
            if !certificate.visit_accounting_is_valid() {
                return Err(ObservationLedgerError::ExcludedFileAccountingMismatch);
            }
        }

        let mut referenced = BTreeSet::new();
        let mut horizon_keys = BTreeSet::new();
        for horizon in &self.horizons {
            if !horizon_keys.insert((&horizon.group, &horizon.scope)) {
                return Err(ObservationLedgerError::DuplicateHorizon);
            }
            let certificate = self
                .certificates
                .get(&horizon.count.certificate_id)
                .ok_or(ObservationLedgerError::DanglingCertificate)?;
            referenced.insert(horizon.count.certificate_id.as_str());
            if horizon.scope != certificate.scope {
                return Err(ObservationLedgerError::ScopeMismatch);
            }
            if horizon.count.closure != certificate.closure {
                return Err(ObservationLedgerError::ClosureMismatch);
            }
            if horizon.count.observed != certificate.observed_facts {
                return Err(ObservationLedgerError::ObservedCountMismatch);
            }
            if horizon.count.reasons != certificate.reasons {
                return Err(ObservationLedgerError::ReasonsMismatch);
            }
            if horizon.shown.checked_add(horizon.hidden) != Some(horizon.count.observed) {
                return Err(ObservationLedgerError::VisibilityMismatch);
            }
            if horizon.unsupported != certificate.unsupported {
                return Err(ObservationLedgerError::UnsupportedMismatch);
            }
            if horizon.dynamic != certificate.dynamic_stops {
                return Err(ObservationLedgerError::DynamicMismatch);
            }
            if (horizon.hidden > 0) != horizon.expand.is_some() {
                return Err(ObservationLedgerError::ExpansionMismatch);
            }
        }
        if referenced.len() != self.certificates.len() {
            return Err(ObservationLedgerError::UnreferencedCertificate);
        }
        Ok(())
    }

    /// Records visibility and its machine basis together, so neither count nor
    /// horizon can carry a dangling certificate id.
    pub fn record(
        &mut self,
        group: impl Into<String>,
        scope: impl Into<String>,
        observed: u64,
        shown: u64,
        mut certificate: CoverageCertificate,
        expand: Option<String>,
    ) -> ObservedCount {
        assert!(shown <= observed, "shown coverage cannot exceed observed");
        let hidden = observed - shown;
        assert!(
            (hidden > 0) == expand.is_some(),
            "coverage expand must exist exactly when facts are hidden"
        );
        let group = group.into();
        let scope = scope.into();
        assert_eq!(
            certificate.scope, scope,
            "horizon and certificate scopes differ"
        );
        assert!(
            certificate.visited_files <= certificate.eligible_files,
            "coverage visited files cannot exceed eligible files"
        );
        assert!(
            certificate.visit_accounting_is_valid(),
            "coverage exclusions must account for every unvisited eligible file"
        );
        certificate.observed_facts = observed;
        let certificate = certificate.seal();
        let count = ObservedCount {
            observed,
            closure: certificate.closure,
            reasons: certificate.reasons.clone(),
            certificate_id: certificate.id.clone(),
        };
        let horizon = CoverageHorizon {
            group,
            scope,
            count: count.clone(),
            shown,
            hidden,
            expand,
            unsupported: certificate.unsupported.clone(),
            dynamic: certificate.dynamic_stops.clone(),
        };
        self.insert_certificate(certificate);
        self.insert_horizon(horizon);
        self.prune_unreferenced_certificates();
        count
    }

    #[cfg(test)]
    pub fn certificate(&self, id: &str) -> Option<&CoverageCertificate> {
        self.certificates.get(id)
    }

    #[cfg(test)]
    pub fn horizon(&self, group: &str) -> Option<&CoverageHorizon> {
        self.horizons.iter().find(|horizon| horizon.group == group)
    }

    pub fn extend(&mut self, other: &Self) {
        for horizon in &other.horizons {
            assert!(
                other
                    .certificates
                    .contains_key(&horizon.count.certificate_id),
                "cannot extend a ledger with a dangling horizon"
            );
        }
        for certificate in other.certificates.values() {
            if let Some(existing) = self.certificates.get(&certificate.id) {
                assert_eq!(existing, certificate, "coverage certificate id collision");
            }
        }
        for certificate in other.certificates.values().cloned() {
            self.insert_certificate(certificate);
        }
        for horizon in other.horizons.iter().cloned() {
            self.insert_horizon(horizon);
        }
        self.prune_unreferenced_certificates();
    }

    pub fn merge(&mut self, other: &Self) {
        self.extend(other);
    }

    fn insert_certificate(&mut self, certificate: CoverageCertificate) {
        self.certificates
            .entry(certificate.id.clone())
            .or_insert(certificate);
    }

    fn insert_horizon(&mut self, horizon: CoverageHorizon) {
        self.horizons
            .retain(|existing| existing.group != horizon.group || existing.scope != horizon.scope);
        self.horizons.push(horizon);
        self.horizons.sort_by(|left, right| {
            (&left.group, &left.scope, &left.count.certificate_id).cmp(&(
                &right.group,
                &right.scope,
                &right.count.certificate_id,
            ))
        });
    }

    fn prune_unreferenced_certificates(&mut self) {
        let referenced = self
            .horizons
            .iter()
            .map(|horizon| horizon.count.certificate_id.as_str())
            .collect::<BTreeSet<_>>();
        self.certificates
            .retain(|id, _| referenced.contains(id.as_str()));
    }
}
