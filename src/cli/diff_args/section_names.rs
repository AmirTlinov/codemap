// Responsibility: cli-section-names
use crate::cli::{ChangedSection, ConeSection, LsSection, ProofSection};

pub(crate) fn changed_section_name(section: Option<ChangedSection>) -> Option<&'static str> {
    match section {
        Some(ChangedSection::Observed) => Some("observed"),
        Some(ChangedSection::Links) => Some("links"),
        Some(ChangedSection::Roles) => Some("roles"),
        Some(ChangedSection::Proof) => Some("proof"),
        Some(ChangedSection::Unknown) => Some("unknown"),
        Some(ChangedSection::Hidden) => Some("hidden"),
        None => None,
    }
}

pub(crate) fn ls_section_name(section: Option<LsSection>) -> Option<&'static str> {
    match section {
        Some(LsSection::Observed) => Some("observed"),
        Some(LsSection::Links) => Some("links"),
        Some(LsSection::Roles) => Some("roles"),
        Some(LsSection::Proof) => Some("proof"),
        Some(LsSection::Unknown) => Some("unknown"),
        Some(LsSection::Hidden) => Some("hidden"),
        None => None,
    }
}

pub(crate) fn cone_section_name(section: Option<ConeSection>) -> Option<&'static str> {
    match section {
        Some(ConeSection::Observed) => Some("observed"),
        Some(ConeSection::Links) => Some("links"),
        Some(ConeSection::Roles) => Some("roles"),
        Some(ConeSection::Proof) => Some("proof"),
        Some(ConeSection::Unknown) => Some("unknown"),
        Some(ConeSection::Hidden) => Some("hidden"),
        None => None,
    }
}

pub(crate) fn proof_section_name(section: Option<ProofSection>) -> Option<&'static str> {
    match section {
        Some(ProofSection::Observed) => Some("observed"),
        Some(ProofSection::Links) => Some("links"),
        Some(ProofSection::Roles) => Some("roles"),
        Some(ProofSection::Proof) => Some("proof"),
        Some(ProofSection::Unknown) => Some("unknown"),
        Some(ProofSection::Hidden) => Some("hidden"),
        None => None,
    }
}
