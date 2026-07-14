// Responsibility: map-boundary-facts
use crate::map::{
    changed_map_path_file_name, changed_path_is_instruction_file, changed_path_is_protected_looking,
};
use crate::model::{BoundaryFact, BoundaryFacts, Project};
use crate::repo;
use std::collections::BTreeSet;

pub(crate) fn boundary_facts(project: &Project, changed_paths: Option<&[String]>) -> BoundaryFacts {
    let paths = if let Some(changed_paths) = changed_paths {
        changed_paths
            .iter()
            .map(|path| repo::normalize_rel_path(path))
            .collect::<BTreeSet<_>>()
    } else {
        project.files.keys().cloned().collect::<BTreeSet<_>>()
    };
    boundary_facts_from_paths(paths)
}

pub(crate) fn boundary_facts_from_paths(paths: BTreeSet<String>) -> BoundaryFacts {
    let mut instruction_files = Vec::new();
    let mut protected_looking_paths = Vec::new();
    let mut repo_local_guard_files = Vec::new();

    for path in paths {
        if boundary_fact_is_instruction_file(&path) {
            instruction_files.push(BoundaryFact {
                path: path.clone(),
                evidence: "path_pattern".to_string(),
                effect: "repo-local instruction surface exists or changed".to_string(),
            });
        }
        if changed_path_is_protected_looking(&path) {
            protected_looking_paths.push(BoundaryFact {
                path: path.clone(),
                evidence: "path_pattern".to_string(),
                effect: "protected-looking generated/vendor/build/model path observed".to_string(),
            });
        }
        if boundary_fact_is_repo_local_guard_file(&path) {
            repo_local_guard_files.push(BoundaryFact {
                path,
                evidence: "path_pattern".to_string(),
                effect: "repo-local guard or proof rail surface exists or changed".to_string(),
            });
        }
    }

    instruction_files.sort_by(|a, b| a.path.cmp(&b.path));
    protected_looking_paths.sort_by(|a, b| a.path.cmp(&b.path));
    repo_local_guard_files.sort_by(|a, b| a.path.cmp(&b.path));
    BoundaryFacts {
        instruction_files,
        protected_looking_paths,
        repo_local_guard_files,
    }
}

pub(crate) fn boundary_facts_for_ls(project: &Project, rel: &str) -> BoundaryFacts {
    if rel == "." {
        let mut paths = project.files.keys().cloned().collect::<BTreeSet<_>>();
        paths.extend(boundary_fact_walk_paths(project));
        boundary_facts_from_paths(paths)
    } else {
        BoundaryFacts::default()
    }
}

pub(crate) fn boundary_facts_for_changed(
    project: &Project,
    changed_paths: &[String],
) -> BoundaryFacts {
    boundary_facts(project, Some(changed_paths))
}

fn boundary_fact_is_instruction_file(path: &str) -> bool {
    changed_path_is_instruction_file(path)
}

fn boundary_fact_is_repo_local_guard_file(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let name = changed_map_path_file_name(&lower);
    matches!(
        name,
        "makefile" | "justfile" | "package.json" | "pyproject.toml" | "cargo.toml" | "go.mod"
    ) || lower.starts_with(".github/workflows/")
        || boundary_fact_matches_tool_guard(&lower, "doctor.")
        || boundary_fact_matches_tool_guard(&lower, "check.")
        || boundary_fact_matches_tool_guard(&lower, "validate.")
        || boundary_fact_matches_tool_guard(&lower, "next.")
}

fn boundary_fact_matches_tool_guard(path: &str, stem_prefix: &str) -> bool {
    path.strip_prefix("tools/")
        .is_some_and(|rest| rest.starts_with(stem_prefix))
}

fn boundary_fact_walk_paths(project: &Project) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    let mut builder = ignore::WalkBuilder::new(&project.root);
    builder
        .standard_filters(true)
        .hidden(false)
        .follow_links(false);
    for entry in builder.build().filter_map(Result::ok) {
        if paths.len() >= 2048 {
            break;
        }
        if !entry
            .file_type()
            .map(|kind| kind.is_file())
            .unwrap_or(false)
        {
            continue;
        }
        let Some(rel) = entry
            .path()
            .strip_prefix(&project.root)
            .ok()
            .map(|path| repo::normalize_rel_path(&path.to_string_lossy()))
        else {
            continue;
        };
        if boundary_fact_is_instruction_file(&rel)
            || boundary_fact_is_repo_local_guard_file(&rel)
            || changed_path_is_protected_looking(&rel)
        {
            paths.insert(rel);
        }
    }
    paths
}
