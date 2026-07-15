// Responsibility: map-cone-xray
mod anchor_surfaces;
mod context_surfaces;
mod edge_classes;

pub(crate) use anchor_surfaces::*;
pub(crate) use context_surfaces::*;
pub(crate) use edge_classes::*;

use crate::model::{EnvDeclaration, FileSummary, Project, StructuralEdge, Unknown, XrayCard};

pub(crate) fn empty_xray_card(anchor: &FileSummary, unknowns: &[Unknown]) -> XrayCard {
    XrayCard {
        roles: xray_role_surfaces(anchor).into_iter().take(8).collect(),
        inputs: Vec::new(),
        outputs: xray_output_surfaces(anchor).into_iter().take(8).collect(),
        state: Vec::new(),
        side_effects: Vec::new(),
        direct_consumers: Vec::new(),
        mediated_consumers: Vec::new(),
        flow: Vec::new(),
        nearby: Vec::new(),
        proof_hard: Vec::new(),
        proof_direct: Vec::new(),
        proof_mediated: Vec::new(),
        proof_soft: Vec::new(),
        unknowns: unknowns.iter().take(8).cloned().collect(),
    }
}

pub(crate) struct ConeXrayInput<'a> {
    pub(crate) project: &'a Project,
    pub(crate) anchor: &'a FileSummary,
    pub(crate) seed_files: &'a [String],
    pub(crate) declared_env: &'a [EnvDeclaration],
    pub(crate) outgoing: &'a [StructuralEdge],
    pub(crate) incoming: &'a [StructuralEdge],
    pub(crate) proof: &'a [StructuralEdge],
    pub(crate) unknowns: &'a [Unknown],
    pub(crate) limit: usize,
    pub(crate) include_hidden: bool,
}

pub(crate) fn cone_xray_card(input: ConeXrayInput<'_>) -> XrayCard {
    let limit = if input.include_hidden {
        usize::MAX
    } else {
        input.limit.clamp(3, 12)
    };
    let mut hard = Vec::new();
    let mut direct = Vec::new();
    let mut mediated = Vec::new();
    let mut soft = Vec::new();
    for edge in input.proof {
        match xray_proof_bucket(edge) {
            XrayEvidenceBucket::Hard => hard.push(edge.clone()),
            XrayEvidenceBucket::Direct => direct.push(edge.clone()),
            XrayEvidenceBucket::Mediated => mediated.push(edge.clone()),
            XrayEvidenceBucket::Soft => soft.push(edge.clone()),
        }
    }

    XrayCard {
        roles: xray_role_surfaces(input.anchor)
            .into_iter()
            .take(limit)
            .collect(),
        inputs: input
            .outgoing
            .iter()
            .filter(|edge| xray_input_edge(edge))
            .take(limit)
            .cloned()
            .collect(),
        outputs: xray_output_surfaces(input.anchor)
            .into_iter()
            .take(limit)
            .collect(),
        state: xray_state_surfaces(
            input.project,
            input.anchor,
            input.seed_files,
            input.declared_env,
        )
        .into_iter()
        .take(limit)
        .collect(),
        side_effects: xray_side_effects(
            input.project,
            input.seed_files,
            limit,
            input.include_hidden,
        ),
        direct_consumers: input
            .incoming
            .iter()
            .filter(|edge| !xray_edge_is_mediated(edge))
            .take(limit)
            .cloned()
            .collect(),
        mediated_consumers: input
            .incoming
            .iter()
            .filter(|edge| xray_edge_is_mediated(edge))
            .take(limit)
            .cloned()
            .collect(),
        flow: xray_flow_steps(input.project, input.seed_files, limit),
        nearby: xray_nearby_surfaces(input.project, input.seed_files, limit),
        proof_hard: hard.into_iter().take(limit).collect(),
        proof_direct: direct.into_iter().take(limit).collect(),
        proof_mediated: mediated.into_iter().take(limit).collect(),
        proof_soft: soft.into_iter().take(limit).collect(),
        unknowns: input.unknowns.iter().take(limit).cloned().collect(),
    }
}
