// Responsibility: runtime-lens-proof-map-selection
use crate::map::{
    changed_section_paths, changed_should_check_direct_proof, direct_files_under_directory,
    directory_has_files, files_under_directory, is_generic_noise, proof_missing_should_surface,
};
use crate::model::{EnvSurface, Project};
use std::collections::BTreeMap;

pub(crate) fn proof_map_seed_selection(
    project: &Project,
    scope: Option<&str>,
    changed: &[String],
    raw_sensors: bool,
    limit: usize,
) -> (Vec<String>, usize) {
    let Some(scope) = scope else {
        if raw_sensors || changed.len() <= limit {
            return (changed.to_vec(), 0);
        }
        return (
            changed_section_paths(project, changed, limit),
            changed.len().saturating_sub(limit),
        );
    };
    if !directory_has_files(project, scope) {
        return (vec![scope.to_string()], 0);
    }
    let all = files_under_directory(project, scope)
        .into_iter()
        .map(|file| file.rel.clone())
        .collect::<Vec<_>>();
    if scope != "." || raw_sensors {
        return (all, 0);
    }
    let mut seeds = direct_files_under_directory(project, scope)
        .into_iter()
        .filter(|file| !file.has_role("generated") && !is_generic_noise(file))
        .map(|file| file.rel.clone())
        .collect::<Vec<_>>();
    seeds.sort();
    let hidden = all.len().saturating_sub(seeds.len());
    (seeds, hidden)
}

pub(crate) fn proof_map_missing_should_surface(
    project: &Project,
    seed: &str,
    scope: Option<&str>,
    changed: &[String],
) -> bool {
    let exact_requested =
        changed.iter().any(|path| path == seed) || scope.is_some_and(|scope| scope == seed);
    if exact_requested
        && project
            .files
            .get(seed)
            .is_some_and(changed_should_check_direct_proof)
    {
        return true;
    }
    if !proof_missing_should_surface(project, seed) {
        return false;
    }
    if exact_requested {
        return true;
    }
    !project
        .packages
        .iter()
        .any(|package| package.manifest == seed)
}

pub(crate) fn group_env_surfaces(values: Vec<EnvSurface>) -> Vec<EnvSurface> {
    let mut seen: BTreeMap<(String, String, String, String), usize> = BTreeMap::new();
    let mut out: Vec<EnvSurface> = Vec::new();
    for value in values {
        let key = (
            value.name.clone(),
            value.used_by.clone(),
            value.declaration.clone().unwrap_or_default(),
            value.evidence.clone(),
        );
        if let Some(index) = seen.get(&key).copied() {
            if value.strength > out[index].strength {
                out[index].strength = value.strength;
            }
            out[index].locations.extend(value.locations);
        } else {
            seen.insert(key, out.len());
            out.push(value);
        }
    }
    out
}
