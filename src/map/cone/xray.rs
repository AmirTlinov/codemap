// Responsibility: map-cone-xray
mod anchor_surfaces;
mod context_surfaces;

pub(crate) use anchor_surfaces::*;
pub(crate) use context_surfaces::*;

use crate::model::{EnvDeclaration, FileSummary, Project, Unknown, XrayCard};

pub(crate) fn empty_xray_card(anchor: &FileSummary, unknowns: &[Unknown]) -> XrayCard {
    XrayCard {
        roles: xray_role_surfaces(anchor).into_iter().take(8).collect(),
        outputs: xray_output_surfaces(anchor).into_iter().take(8).collect(),
        state: Vec::new(),
        side_effects: Vec::new(),
        flow: Vec::new(),
        nearby: Vec::new(),
        unknowns: unknowns.iter().take(8).cloned().collect(),
    }
}

pub(crate) struct ConeXrayInput<'a> {
    pub(crate) project: &'a Project,
    pub(crate) anchor: &'a FileSummary,
    pub(crate) seed_files: &'a [String],
    pub(crate) declared_env: &'a [EnvDeclaration],
    pub(crate) unknowns: &'a [Unknown],
    pub(crate) limit: usize,
    pub(crate) include_hidden: bool,
}

pub(crate) fn cone_xray_card(input: ConeXrayInput<'_>) -> XrayCard {
    let (limit, compact_limit) = if input.include_hidden {
        (usize::MAX, usize::MAX)
    } else {
        let limit = input.limit.clamp(3, 12);
        (limit, limit.min(5))
    };
    let expand = format!(
        "codemap cone {} --all",
        crate::map::shell_quote(&input.anchor.path)
    );
    XrayCard {
        roles: bounded_xray_group(
            "xray_roles",
            xray_role_surfaces(input.anchor),
            compact_limit,
            &expand,
        ),
        outputs: bounded_xray_group(
            "xray_outputs",
            xray_output_surfaces(input.anchor),
            limit,
            &expand,
        ),
        state: bounded_xray_group(
            "xray_state",
            xray_state_surfaces(
                input.project,
                input.anchor,
                input.seed_files,
                input.declared_env,
            ),
            compact_limit,
            &expand,
        ),
        side_effects: bounded_xray_group(
            "xray_side_effects",
            xray_side_effects(input.project, input.seed_files, input.include_hidden),
            compact_limit,
            &expand,
        ),
        flow: bounded_xray_group(
            "xray_flow",
            xray_flow_steps(input.project, input.seed_files),
            compact_limit,
            &expand,
        ),
        nearby: bounded_xray_group(
            "xray_nearby",
            xray_nearby_surfaces(input.project, input.seed_files),
            compact_limit,
            &expand,
        ),
        unknowns: bounded_xray_group(
            "xray_unknowns",
            input.unknowns.to_vec(),
            compact_limit,
            &expand,
        ),
    }
}

fn bounded_xray_group<T>(group: &str, values: Vec<T>, limit: usize, expand: &str) -> Vec<T> {
    crate::map::BoundedProjection::ordered(group, values, limit, expand).into_shown()
}
