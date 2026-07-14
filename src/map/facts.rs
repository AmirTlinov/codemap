// Responsibility: map-facts
use crate::map::{directory_surface_role, symbol_anchor_path};
use crate::model::{EvidenceLocation, EvidenceStrength, ProofSurface, Surface};

pub(crate) struct SurfaceFact {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) path: Option<String>,
    pub(crate) role: Option<String>,
    pub(crate) evidence: String,
    pub(crate) strength: EvidenceStrength,
    pub(crate) count: Option<usize>,
    pub(crate) examples: Vec<String>,
    pub(crate) hidden_count: usize,
}

pub(crate) fn surface(fact: SurfaceFact) -> Surface {
    Surface {
        id: fact.id,
        kind: fact.kind,
        path: fact.path,
        role: fact.role,
        evidence: fact.evidence,
        strength: fact.strength,
        count: fact.count,
        examples: fact.examples,
        hidden_count: fact.hidden_count,
    }
}

pub(crate) fn surface_from_path(
    kind: &str,
    path: &str,
    evidence: &str,
    strength: EvidenceStrength,
) -> Surface {
    surface(SurfaceFact {
        id: format!("surface:{kind}:{path}"),
        kind: kind.to_string(),
        path: Some(path.to_string()),
        role: directory_surface_role(kind),
        evidence: evidence.to_string(),
        strength,
        count: Some(1),
        examples: vec![path.to_string()],
        hidden_count: 0,
    })
}

pub(crate) fn exported_symbol_surface(file_rel: &str, export: &str) -> Surface {
    let anchor = symbol_anchor_path(file_rel, export);
    surface(SurfaceFact {
        id: format!("surface:export:{file_rel}#{export}"),
        kind: "exported_symbol".to_string(),
        path: Some(anchor.clone()),
        role: Some("contract".to_string()),
        evidence: "exported_symbol".to_string(),
        strength: EvidenceStrength::High,
        count: Some(1),
        examples: vec![anchor],
        hidden_count: 0,
    })
}

pub(crate) fn proof_surface(
    command: Option<String>,
    path: Option<String>,
    evidence: &str,
    strength: EvidenceStrength,
    reason: String,
    locations: Vec<EvidenceLocation>,
) -> ProofSurface {
    ProofSurface {
        command,
        path: path.clone(),
        target_anchor: path,
        evidence: evidence.to_string(),
        strength,
        reason,
        locations,
    }
}
