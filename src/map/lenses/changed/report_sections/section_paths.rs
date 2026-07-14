// Responsibility: changed-section-paths
use crate::map::{
    changed_lens_path_looks_like_source, changed_manifest_for_lockfile, changed_map_path_is_config,
    changed_map_path_is_manifest, changed_path_is_generated, changed_path_is_large_binary,
    changed_path_is_model_weight_like, changed_path_is_protected_looking,
    changed_path_is_runner_like,
};
use crate::model::Project;

pub(crate) fn changed_section_paths(
    project: &Project,
    changed_paths: &[String],
    limit: usize,
) -> Vec<String> {
    let mut ranked = changed_paths
        .iter()
        .enumerate()
        .map(|(index, path)| {
            (
                changed_section_path_cost(project, path),
                index,
                path.clone(),
            )
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    ranked
        .into_iter()
        .take(limit)
        .map(|(_, _, path)| path)
        .collect()
}

fn changed_section_path_cost(project: &Project, path: &str) -> usize {
    if changed_manifest_for_lockfile(path).is_some()
        || changed_path_is_generated(project, path)
        || changed_path_is_large_binary(project, path)
        || changed_path_is_model_weight_like(path)
    {
        return 4;
    }
    if project.files.get(path).is_some_and(|file| {
        file.has_role("schema_contract")
            || file.has_role("manifest")
            || file.has_role("env_config")
            || file.has_role("build_ci")
    }) || changed_map_path_is_manifest(path)
        || changed_map_path_is_config(path)
        || changed_path_is_runner_like(path)
        || changed_lens_path_looks_like_source(&path.to_ascii_lowercase())
    {
        return 0;
    }
    if changed_path_is_protected_looking(path) {
        return 3;
    }
    1
}
