// Responsibility: changed-diff-structural-events
use crate::map::{
    DiffMapMode, changed_map_path_file_name, changed_map_path_is_config, changed_map_path_is_env,
    changed_map_path_is_manifest, changed_receipt_witness_events, changed_receipt_witness_surface,
    ci_run_lines, config_key_lines, diff_base_file_text, diff_current_file_text, env_key_lines,
    package_script_lines_from_text, schema_decl_lines, schema_owner_path, shell_quote,
};
use crate::model::{EvidenceLocation, Project};
use std::collections::BTreeSet;

pub(crate) fn changed_diff_structural_events(
    project: &Project,
    changed: &[String],
    mode: &DiffMapMode,
) -> Vec<crate::model::ChangedStructuralEvent> {
    let mut events = Vec::new();
    for rel in changed {
        events.extend(changed_env_key_events(project, rel, mode));
        events.extend(changed_receipt_witness_events(project, rel, mode));
        events.extend(changed_config_key_events(project, rel, mode));
        events.extend(changed_package_script_events(project, rel, mode));
        events.extend(changed_schema_decl_events(project, rel, mode));
        events.extend(changed_ci_run_events(project, rel, mode));
    }
    let mut seen = BTreeSet::new();
    events.retain(|event| {
        seen.insert((
            event.kind.clone(),
            event.path.clone(),
            event.effect.clone(),
            event
                .locations
                .first()
                .and_then(|location| location.line_start)
                .unwrap_or_default(),
        ))
    });
    events
}

fn changed_env_key_events(
    project: &Project,
    rel: &str,
    mode: &DiffMapMode,
) -> Vec<crate::model::ChangedStructuralEvent> {
    if !changed_map_path_is_env(rel) {
        return Vec::new();
    }
    let current = diff_current_file_text(project, rel, mode).unwrap_or_default();
    let base = diff_base_file_text(project, rel, mode).unwrap_or_default();
    let current_keys = env_key_lines(&current);
    let base_keys = env_key_lines(&base);
    let mut events = Vec::new();
    for (key, line) in current_keys
        .iter()
        .filter(|(key, _)| !base_keys.contains_key(*key))
    {
        events.push(changed_fact_event(
            "added_env_key",
            rel,
            *line,
            "git_diff_env_declaration",
            format!("env/runtime key `{key}` was added"),
            Some(format!("codemap cone {}", shell_quote(rel))),
            "env_declaration",
        ));
    }
    for (key, line) in base_keys
        .iter()
        .filter(|(key, _)| !current_keys.contains_key(*key))
    {
        events.push(changed_fact_event(
            "removed_env_key",
            rel,
            *line,
            "git_diff_env_declaration",
            format!("env/runtime key `{key}` was removed"),
            Some(format!("codemap runtime {}", shell_quote(rel))),
            "env_declaration",
        ));
    }
    events
}

fn changed_config_key_events(
    project: &Project,
    rel: &str,
    mode: &DiffMapMode,
) -> Vec<crate::model::ChangedStructuralEvent> {
    if changed_map_path_is_env(rel) || !changed_map_path_is_config(rel) {
        return Vec::new();
    }
    if changed_receipt_witness_surface(project, rel) {
        return Vec::new();
    }
    let schema_contract = project
        .files
        .get(rel)
        .is_some_and(|file| file.has_role("schema_contract"));
    let current = diff_current_file_text(project, rel, mode).unwrap_or_default();
    let base = diff_base_file_text(project, rel, mode).unwrap_or_default();
    let current_keys = config_key_lines(rel, &current);
    let base_keys = config_key_lines(rel, &base);
    let mut events = Vec::new();
    for (key, line) in current_keys
        .iter()
        .filter(|(key, _)| !base_keys.contains_key(*key))
    {
        let (kind, evidence, effect, location_kind) = if schema_contract {
            (
                "added_schema_field",
                "git_diff_schema_field",
                format!("schema field/key `{key}` was added"),
                "schema_field",
            )
        } else {
            (
                "added_config_key",
                "git_diff_config_key",
                format!("config key `{key}` was added"),
                "config_key",
            )
        };
        events.push(changed_fact_event(
            kind,
            rel,
            *line,
            evidence,
            effect,
            Some(format!("codemap ls {}", shell_quote(rel))),
            location_kind,
        ));
    }
    for (key, line) in base_keys
        .iter()
        .filter(|(key, _)| !current_keys.contains_key(*key))
    {
        let (kind, evidence, effect, location_kind) = if schema_contract {
            (
                "removed_schema_field",
                "git_diff_schema_field",
                format!("schema field/key `{key}` was removed"),
                "schema_field",
            )
        } else {
            (
                "removed_config_key",
                "git_diff_config_key",
                format!("config key `{key}` was removed"),
                "config_key",
            )
        };
        events.push(changed_fact_event(
            kind,
            rel,
            *line,
            evidence,
            effect,
            Some(format!("codemap diff-map --files {}", shell_quote(rel))),
            location_kind,
        ));
    }
    events
}

fn changed_package_script_events(
    project: &Project,
    rel: &str,
    mode: &DiffMapMode,
) -> Vec<crate::model::ChangedStructuralEvent> {
    if !changed_map_path_is_manifest(rel) || changed_map_path_file_name(rel) != "package.json" {
        return Vec::new();
    }
    let current = diff_current_file_text(project, rel, mode).unwrap_or_default();
    let base = diff_base_file_text(project, rel, mode).unwrap_or_default();
    let current_scripts = package_script_lines_from_text(&current);
    let base_scripts = package_script_lines_from_text(&base);
    let mut events = Vec::new();
    for (script, line) in current_scripts
        .iter()
        .filter(|(script, _)| !base_scripts.contains_key(*script))
    {
        events.push(changed_fact_event(
            "added_manifest_script",
            rel,
            *line,
            "git_diff_manifest_script",
            format!("package script `{script}` was added"),
            Some(format!("codemap cone {}", shell_quote(rel))),
            "package_script",
        ));
    }
    for (script, line) in base_scripts
        .iter()
        .filter(|(script, _)| !current_scripts.contains_key(*script))
    {
        events.push(changed_fact_event(
            "removed_manifest_script",
            rel,
            *line,
            "git_diff_manifest_script",
            format!("package script `{script}` was removed"),
            Some(format!("codemap proof {}", shell_quote(rel))),
            "package_script",
        ));
    }
    events
}

fn changed_schema_decl_events(
    project: &Project,
    rel: &str,
    mode: &DiffMapMode,
) -> Vec<crate::model::ChangedStructuralEvent> {
    if !schema_owner_path(rel) {
        return Vec::new();
    }
    let current = diff_current_file_text(project, rel, mode).unwrap_or_default();
    let base = diff_base_file_text(project, rel, mode).unwrap_or_default();
    let current_decls = schema_decl_lines(&current);
    let base_decls = schema_decl_lines(&base);
    let mut events = Vec::new();
    for (decl, line) in current_decls
        .iter()
        .filter(|(decl, _)| !base_decls.contains_key(*decl))
    {
        events.push(changed_fact_event(
            "added_schema_surface",
            rel,
            *line,
            "git_diff_schema_declaration",
            format!("schema declaration `{decl}` was added"),
            Some(format!("codemap cone {}", shell_quote(rel))),
            "schema_declaration",
        ));
    }
    for (decl, line) in base_decls
        .iter()
        .filter(|(decl, _)| !current_decls.contains_key(*decl))
    {
        events.push(changed_fact_event(
            "removed_schema_surface",
            rel,
            *line,
            "git_diff_schema_declaration",
            format!("schema declaration `{decl}` was removed"),
            Some(format!("codemap proof {}", shell_quote(rel))),
            "schema_declaration",
        ));
    }
    events
}

fn changed_ci_run_events(
    project: &Project,
    rel: &str,
    mode: &DiffMapMode,
) -> Vec<crate::model::ChangedStructuralEvent> {
    let Some(file) = project.files.get(rel) else {
        return Vec::new();
    };
    if !file.has_role("build_ci") {
        return Vec::new();
    }
    let current = diff_current_file_text(project, rel, mode).unwrap_or_default();
    let base = diff_base_file_text(project, rel, mode).unwrap_or_default();
    let current_runs = ci_run_lines(&current);
    let base_runs = ci_run_lines(&base);
    let mut events = Vec::new();
    for (command, line) in current_runs
        .iter()
        .filter(|(command, _)| !base_runs.contains_key(*command))
    {
        events.push(changed_fact_event(
            "added_ci_run_step",
            rel,
            *line,
            "git_diff_ci_run",
            format!("CI run step `{command}` was added"),
            Some(format!("codemap cone {}", shell_quote(rel))),
            "ci_step",
        ));
    }
    for (command, line) in base_runs
        .iter()
        .filter(|(command, _)| !current_runs.contains_key(*command))
    {
        events.push(changed_fact_event(
            "removed_ci_run_step",
            rel,
            *line,
            "git_diff_ci_run",
            format!("CI run step `{command}` was removed"),
            Some(format!("codemap proof {}", shell_quote(rel))),
            "ci_step",
        ));
    }
    events
}

pub(crate) fn changed_fact_event(
    kind: &str,
    rel: &str,
    line: usize,
    evidence: &str,
    effect: String,
    expand: Option<String>,
    location_kind: &str,
) -> crate::model::ChangedStructuralEvent {
    crate::model::ChangedStructuralEvent {
        kind: kind.to_string(),
        path: rel.to_string(),
        old_path: None,
        evidence: evidence.to_string(),
        effect,
        locations: vec![EvidenceLocation::line(rel, line.max(1), location_kind)],
        expand,
    }
}
