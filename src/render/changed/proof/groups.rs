// Responsibility: changed-proof-grouping
use crate::model::{ChangedReport, ProofSurface};
use crate::render::proof_display_command;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChangedProofGroupClass {
    Runnable,
    Setup,
    Soft,
}

pub(crate) fn changed_proof_groups_by_class<'a>(
    grouped: &'a std::collections::BTreeMap<String, (Vec<&'a ProofSurface>, usize)>,
    class: ChangedProofGroupClass,
) -> Vec<(&'a String, &'a (Vec<&'a ProofSurface>, usize))> {
    grouped
        .iter()
        .filter(|(_, (sensors, _))| changed_proof_group_class(sensors) == class)
        .collect()
}

fn changed_proof_group_class(sensors: &[&ProofSurface]) -> ChangedProofGroupClass {
    if sensors.is_empty()
        || sensors
            .iter()
            .any(|sensor| crate::proof_classification::proof_surface_is_runnable_validation(sensor))
    {
        return ChangedProofGroupClass::Runnable;
    }
    if sensors
        .iter()
        .any(|sensor| crate::proof_classification::proof_surface_is_setup_or_support(sensor))
    {
        return ChangedProofGroupClass::Setup;
    }
    ChangedProofGroupClass::Soft
}

pub(crate) fn changed_proof_command_groups(
    report: &ChangedReport,
) -> std::collections::BTreeMap<String, (Vec<&ProofSurface>, usize)> {
    let mut grouped: std::collections::BTreeMap<String, (Vec<&ProofSurface>, usize)> =
        std::collections::BTreeMap::new();
    for command in &report.proof.commands {
        if command.sensors.is_empty() {
            grouped
                .entry(command.command.clone())
                .or_insert_with(|| (Vec::new(), 0))
                .1 += command.hidden_count;
            continue;
        }
        let mut command_groups = std::collections::BTreeSet::new();
        for sensor in &command.sensors {
            let key = proof_display_command(sensor);
            command_groups.insert(key.clone());
            grouped
                .entry(key)
                .or_insert_with(|| (Vec::new(), 0))
                .0
                .push(sensor);
        }
        if command_groups.len() == 1
            && let Some(key) = command_groups.first()
        {
            grouped
                .entry(key.clone())
                .or_insert_with(|| (Vec::new(), 0))
                .1 += command.hidden_count;
        }
    }
    grouped
}

pub(crate) fn changed_proof_surface_groups<'a>(
    surfaces: impl Iterator<Item = &'a ProofSurface>,
) -> std::collections::BTreeMap<String, (Vec<&'a ProofSurface>, usize)> {
    let mut grouped: std::collections::BTreeMap<String, (Vec<&ProofSurface>, usize)> =
        std::collections::BTreeMap::new();
    for sensor in surfaces {
        grouped
            .entry(proof_display_command(sensor))
            .or_insert_with(|| (Vec::new(), 0))
            .0
            .push(sensor);
    }
    grouped
}
