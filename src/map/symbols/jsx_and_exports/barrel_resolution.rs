// Responsibility: map-symbols-barrel-resolution
use crate::map::{
    default_export_symbol_name, file_has_inline_named_export, imported_symbol_binding_matches,
    local_named_export_bindings, matching_symbols,
};
use crate::model::{FileInfo, Project};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
enum PublicResolution {
    Missing,
    Ambiguous,
    Resolved {
        owner_rel: String,
        public_name: String,
    },
}

#[derive(Debug, Default)]
pub(crate) struct BarrelResolutionCache {
    resolutions: BTreeMap<(String, String, String), PublicResolution>,
    reachability: BTreeMap<(String, String), bool>,
    exposures: BTreeMap<(String, String), bool>,
}

impl BarrelResolutionCache {
    pub(crate) fn may_reexport_from(
        &mut self,
        project: &Project,
        barrel_rel: &str,
        target_rel: &str,
    ) -> bool {
        let key = (barrel_rel.to_string(), target_rel.to_string());
        if let Some(reachable) = self.reachability.get(&key) {
            return *reachable;
        }
        let mut visiting = BTreeSet::new();
        let reachable = reexport_path_reaches(project, barrel_rel, target_rel, &mut visiting);
        self.reachability.insert(key, reachable);
        reachable
    }
}

fn reexport_path_reaches(
    project: &Project,
    from_rel: &str,
    target_rel: &str,
    visiting: &mut BTreeSet<String>,
) -> bool {
    if from_rel == target_rel {
        return true;
    }
    if !visiting.insert(from_rel.to_string()) {
        return false;
    }
    let reachable = project.files.get(from_rel).is_some_and(|file| {
        file.resolved_import_bindings
            .iter()
            .filter(|(_, bindings)| bindings.keys().any(|local| local.starts_with("export:")))
            .any(|(next_rel, _)| reexport_path_reaches(project, next_rel, target_rel, visiting))
    });
    visiting.remove(from_rel);
    reachable
}

pub(crate) fn barrel_reexports_symbol_from_file(
    project: &Project,
    barrel: &FileInfo,
    file_rel: &str,
    symbol_name: &str,
    imported_public_name: &str,
    cache: &mut BarrelResolutionCache,
) -> bool {
    let mut visiting = BTreeSet::new();
    match resolve_public_name(
        project,
        &barrel.rel,
        imported_public_name,
        file_rel,
        cache,
        &mut visiting,
    ) {
        PublicResolution::Resolved {
            owner_rel,
            public_name,
        } => {
            owner_rel == file_rel
                && imported_symbol_binding_matches(project, file_rel, symbol_name, &public_name)
        }
        PublicResolution::Missing | PublicResolution::Ambiguous => false,
    }
}

fn resolve_public_name(
    project: &Project,
    file_rel: &str,
    public_name: &str,
    target_rel: &str,
    cache: &mut BarrelResolutionCache,
    visiting: &mut BTreeSet<(String, String, String)>,
) -> PublicResolution {
    let key = (
        file_rel.to_string(),
        public_name.to_string(),
        target_rel.to_string(),
    );
    if let Some(resolution) = cache.resolutions.get(&key) {
        return resolution.clone();
    }
    if !visiting.insert(key.clone()) {
        return PublicResolution::Missing;
    }
    let resolution = project
        .files
        .get(file_rel)
        .map(|file| resolve_from_file(project, file, public_name, target_rel, cache, visiting))
        .unwrap_or(PublicResolution::Missing);
    visiting.remove(&key);
    cache.resolutions.insert(key, resolution.clone());
    resolution
}

fn resolve_from_file(
    project: &Project,
    file: &FileInfo,
    public_name: &str,
    target_rel: &str,
    cache: &mut BarrelResolutionCache,
    visiting: &mut BTreeSet<(String, String, String)>,
) -> PublicResolution {
    let mut explicit = Vec::new();
    if public_name == "default" && default_export_symbol_name(project, &file.rel).is_some() {
        explicit.push(local_resolution(file, public_name, target_rel));
    }
    if file_has_inline_named_export(project, &file.rel, public_name) {
        explicit.push(local_resolution(file, public_name, target_rel));
    }
    if let Some(locals) = local_named_export_bindings(project, &file.rel).get(public_name) {
        for local in locals {
            explicit.push(if !matching_symbols(file, local).is_empty() {
                local_resolution(file, local, target_rel)
            } else {
                PublicResolution::Missing
            });
        }
    }
    for (owner_target, bindings) in &file.resolved_import_bindings {
        for (exported, imported_name) in bindings {
            if exported.strip_prefix("export:") != Some(public_name) {
                continue;
            }
            explicit.push(
                if owner_target == target_rel
                    || cache.may_reexport_from(project, owner_target, target_rel)
                {
                    resolve_public_name(
                        project,
                        owner_target,
                        imported_name,
                        target_rel,
                        cache,
                        visiting,
                    )
                } else {
                    PublicResolution::Missing
                },
            );
        }
    }
    if !explicit.is_empty() {
        return one_resolution(explicit);
    }
    if public_name == "default" {
        return PublicResolution::Missing;
    }

    let mut star_resolutions = Vec::new();
    let mut unrelated_stars = Vec::new();
    for (owner_target, bindings) in &file.resolved_import_bindings {
        if bindings.get("export:*").is_none_or(|value| value != "*") {
            continue;
        }
        if owner_target == target_rel || cache.may_reexport_from(project, owner_target, target_rel)
        {
            let resolution = resolve_public_name(
                project,
                owner_target,
                public_name,
                target_rel,
                cache,
                visiting,
            );
            if !matches!(resolution, PublicResolution::Missing) {
                star_resolutions.push(resolution);
            }
        } else {
            unrelated_stars.push(owner_target);
        }
    }
    if star_resolutions.len() != 1 {
        return if star_resolutions.is_empty() {
            PublicResolution::Missing
        } else {
            PublicResolution::Ambiguous
        };
    }
    let resolution = star_resolutions
        .into_iter()
        .next()
        .unwrap_or(PublicResolution::Missing);
    if matches!(resolution, PublicResolution::Ambiguous)
        || unrelated_stars
            .into_iter()
            .any(|owner| public_name_is_exposed(project, owner, public_name, cache))
    {
        PublicResolution::Ambiguous
    } else {
        resolution
    }
}

fn public_name_is_exposed(
    project: &Project,
    file_rel: &str,
    public_name: &str,
    cache: &mut BarrelResolutionCache,
) -> bool {
    let mut visiting = BTreeSet::new();
    public_name_is_exposed_inner(project, file_rel, public_name, cache, &mut visiting)
}

fn public_name_is_exposed_inner(
    project: &Project,
    file_rel: &str,
    public_name: &str,
    cache: &mut BarrelResolutionCache,
    visiting: &mut BTreeSet<(String, String)>,
) -> bool {
    let key = (file_rel.to_string(), public_name.to_string());
    if let Some(exposed) = cache.exposures.get(&key) {
        return *exposed;
    }
    if !visiting.insert(key.clone()) {
        return false;
    }
    let exposed = project.files.get(file_rel).is_some_and(|file| {
        let local_list = local_named_export_bindings(project, file_rel);
        let has_default =
            public_name == "default" && default_export_symbol_name(project, file_rel).is_some();
        let has_inline = file_has_inline_named_export(project, file_rel, public_name);
        let has_local = local_list.get(public_name).is_some_and(|locals| {
            locals
                .iter()
                .any(|local| !matching_symbols(file, local).is_empty())
        });
        let explicit_reexports = file
            .resolved_import_bindings
            .iter()
            .flat_map(|(owner, bindings)| {
                bindings.iter().filter_map(move |(exported, imported)| {
                    (exported.strip_prefix("export:") == Some(public_name))
                        .then_some((owner, imported))
                })
            })
            .collect::<Vec<_>>();
        if has_default || has_inline || has_local {
            return true;
        }
        if !explicit_reexports.is_empty() {
            return explicit_reexports.into_iter().any(|(owner, imported)| {
                public_name_is_exposed_inner(project, owner, imported, cache, visiting)
            });
        }
        if public_name == "default" {
            return false;
        }
        file.resolved_import_bindings
            .iter()
            .filter(|(_, bindings)| bindings.get("export:*").is_some_and(|value| value == "*"))
            .any(|(owner, _)| {
                public_name_is_exposed_inner(project, owner, public_name, cache, visiting)
            })
    });
    visiting.remove(&key);
    cache.exposures.insert(key, exposed);
    exposed
}

fn local_resolution(file: &FileInfo, public_name: &str, target_rel: &str) -> PublicResolution {
    if file.rel == target_rel {
        PublicResolution::Resolved {
            owner_rel: file.rel.clone(),
            public_name: public_name.to_string(),
        }
    } else {
        PublicResolution::Missing
    }
}

fn one_resolution(resolutions: Vec<PublicResolution>) -> PublicResolution {
    if resolutions.len() != 1 {
        return PublicResolution::Ambiguous;
    }
    resolutions
        .into_iter()
        .next()
        .unwrap_or(PublicResolution::Missing)
}
