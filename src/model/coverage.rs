// Responsibility: coverage-observation-and-certificate-primitives
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub type CoverageCertificateId = String;

/// Legacy count shape embedded in report kinds outside the S03 propagation boundary.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CountStatus {
    Counted,
    ProvenZero,
    Unknown,
}

/// Compatibility carrier for reports which have not migrated to `ObservedCount`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CountFact {
    pub status: CountStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl CountFact {
    pub fn counted(value: usize) -> Self {
        Self {
            status: CountStatus::Counted,
            value: Some(value),
            reason: None,
        }
    }

    pub fn proven_zero() -> Self {
        Self {
            status: CountStatus::ProvenZero,
            value: Some(0),
            reason: None,
        }
    }

    pub fn unknown(reason: impl Into<String>) -> Self {
        Self {
            status: CountStatus::Unknown,
            value: None,
            reason: Some(reason.into()),
        }
    }

    pub fn display(&self) -> String {
        match self.status {
            CountStatus::Unknown => format!(
                "unknown ({})",
                self.reason.as_deref().unwrap_or("unclassified flow")
            ),
            _ => self.value.unwrap_or(0).to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoverageClosure {
    Closed,
    Open,
    Unavailable,
}

impl CoverageClosure {
    pub(crate) fn from_gaps(has_gaps: bool) -> Self {
        if has_gaps { Self::Open } else { Self::Closed }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CoverageReason {
    AnchorNotIndexed,
    DynamicImportFlow,
    DynamicRuntimeRegistration,
    IncompleteTraversal,
    ReexportFlow,
    RustIncludeFlow,
    UnsupportedConstruct,
    UnsupportedLanguage,
    UnclassifiedCoverageGap,
    VerificationRelationFlow,
}

impl CoverageReason {
    pub fn label(self) -> &'static str {
        match self {
            Self::AnchorNotIndexed => "anchor is not indexed",
            Self::DynamicImportFlow => "dynamic import flow",
            Self::DynamicRuntimeRegistration => "dynamic runtime registration",
            Self::IncompleteTraversal => "incomplete file traversal",
            Self::ReexportFlow => "re-export flow",
            Self::RustIncludeFlow => "rust include! flow",
            Self::UnsupportedConstruct => "unsupported construct",
            Self::UnsupportedLanguage => "unsupported language",
            Self::UnclassifiedCoverageGap => "unclassified coverage gap",
            Self::VerificationRelationFlow => "verification relations partial",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ObservedCount {
    pub observed: u64,
    pub closure: CoverageClosure,
    pub reasons: Vec<CoverageReason>,
    pub certificate_id: CoverageCertificateId,
}

impl ObservedCount {
    pub fn display(&self) -> String {
        let reasons = self
            .reasons
            .iter()
            .map(|reason| reason.label())
            .collect::<Vec<_>>()
            .join(", ");
        match (self.observed, self.closure) {
            (0, CoverageClosure::Closed) => "proven-zero".to_string(),
            (observed, CoverageClosure::Closed) => format!("counted({observed})"),
            (0, CoverageClosure::Open) => format!("unknown lower bound: 0 ({reasons})"),
            (observed, CoverageClosure::Open) => {
                format!("counted-at-least({observed}, {reasons})")
            }
            (_, CoverageClosure::Unavailable) => format!("unknown({reasons})"),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CoverageLocation {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_start: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_end: Option<usize>,
}

impl CoverageLocation {
    pub fn path(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            line_start: None,
            line_end: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExtractorCapability {
    pub extractor_id: String,
    pub version: String,
    pub language: String,
    pub constructs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnsupportedObservation {
    pub file: String,
    pub construct: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<CoverageLocation>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct CoverageStop {
    pub kind: CoverageReason,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<CoverageLocation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub missing_surface: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CoverageCertificate {
    pub id: CoverageCertificateId,
    pub query_kind: String,
    pub scope: String,
    pub snapshot: String,
    pub observed_facts: u64,
    pub eligible_files: u64,
    pub visited_files: u64,
    pub excluded_files_by_reason: BTreeMap<CoverageReason, Vec<String>>,
    pub extractor_capabilities: Vec<ExtractorCapability>,
    pub unsupported: Vec<UnsupportedObservation>,
    pub dynamic_stops: Vec<CoverageStop>,
    pub unresolved_stops: Vec<CoverageStop>,
    pub external_stops: Vec<CoverageStop>,
    pub closure: CoverageClosure,
    pub reasons: Vec<CoverageReason>,
}

impl CoverageCertificate {
    pub fn new(
        query_kind: impl Into<String>,
        scope: impl Into<String>,
        snapshot: impl Into<String>,
        eligible_files: u64,
        visited_files: u64,
        closure: CoverageClosure,
        reasons: Vec<CoverageReason>,
    ) -> Self {
        Self {
            id: String::new(),
            query_kind: query_kind.into(),
            scope: scope.into(),
            snapshot: snapshot.into(),
            observed_facts: 0,
            eligible_files,
            visited_files,
            excluded_files_by_reason: BTreeMap::new(),
            extractor_capabilities: Vec::new(),
            unsupported: Vec::new(),
            dynamic_stops: Vec::new(),
            unresolved_stops: Vec::new(),
            external_stops: Vec::new(),
            closure,
            reasons,
        }
    }

    pub(super) fn seal(mut self) -> Self {
        self.normalize();
        self.id.clear();
        let body = serde_json::to_vec(&self).expect("coverage certificate should serialize");
        self.id = format!("coverage-v1:{:x}", Sha256::digest(body));
        self
    }

    fn normalize(&mut self) {
        self.reasons.sort();
        self.reasons.dedup();
        for files in self.excluded_files_by_reason.values_mut() {
            files.sort();
            files.dedup();
        }
        for capability in &mut self.extractor_capabilities {
            capability.constructs.sort();
            capability.constructs.dedup();
        }
        self.extractor_capabilities.sort();
        self.extractor_capabilities.dedup();
        self.unsupported.sort();
        self.unsupported.dedup();
        self.dynamic_stops.sort();
        self.dynamic_stops.dedup();
        self.unresolved_stops.sort();
        self.unresolved_stops.dedup();
        self.external_stops.sort();
        self.external_stops.dedup();

        if self.closure != CoverageClosure::Unavailable && self.visited_files < self.eligible_files
        {
            self.reasons.push(CoverageReason::IncompleteTraversal);
        }
        self.reasons
            .extend(self.excluded_files_by_reason.keys().copied());
        self.reasons
            .extend(self.dynamic_stops.iter().map(|stop| stop.kind));
        self.reasons
            .extend(self.unresolved_stops.iter().map(|stop| stop.kind));
        self.reasons
            .extend(self.external_stops.iter().map(|stop| stop.kind));
        if !self.unsupported.is_empty()
            && !self.reasons.iter().any(|reason| {
                matches!(
                    reason,
                    CoverageReason::UnsupportedConstruct | CoverageReason::UnsupportedLanguage
                )
            })
        {
            self.reasons.push(CoverageReason::UnsupportedConstruct);
        }
        self.reasons.sort();
        self.reasons.dedup();

        if self.closure == CoverageClosure::Closed && !self.reasons.is_empty() {
            self.closure = CoverageClosure::Open;
        }
        if self.closure != CoverageClosure::Closed && self.reasons.is_empty() {
            self.reasons.push(CoverageReason::UnclassifiedCoverageGap);
        }
    }

    pub(crate) fn visit_accounting_is_valid(&self) -> bool {
        if self.visited_files > self.eligible_files {
            return false;
        }
        let mut excluded = std::collections::BTreeSet::new();
        for files in self.excluded_files_by_reason.values() {
            for file in files {
                if !excluded.insert(file) {
                    return false;
                }
            }
        }
        self.eligible_files - self.visited_files == excluded.len() as u64
    }
}
