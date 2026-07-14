// Responsibility: cli-anchors-details
use crate::cli::{
    AnchorValidationDetail, anchor_pattern_match_count, anchor_pattern_matches_project,
    dedupe_strings, glob_static_prefix, is_glob_like, shell_quote_arg,
};
use crate::{map, repo};

pub(crate) fn semantic_anchor_details(
    project: &crate::model::Project,
    can_run_map_commands: bool,
) -> Vec<AnchorValidationDetail> {
    let mut details = Vec::new();
    for error in &project.config_errors {
        details.push(AnchorValidationDetail {
            kind: "config".to_string(),
            id: error.path.clone(),
            status: "problem".to_string(),
            message: format!("semantic anchor config rejected: {}", error.error),
            next: vec!["codemap anchors validate".to_string()],
        });
    }
    if project.config_path.is_none() {
        if details.is_empty() {
            details.push(AnchorValidationDetail {
                kind: "config".to_string(),
                id: "zero-config".to_string(),
                status: "info".to_string(),
                message:
                    "no .codemap.yml loaded; structural maps use repo files, manifests, imports, and tests"
                        .to_string(),
                next: vec!["codemap ls .".to_string()],
            });
        }
        return details;
    }
    if let Some(config) = &project.config_path {
        details.push(AnchorValidationDetail {
            kind: "config".to_string(),
            id: config.clone(),
            status: "ok".to_string(),
            message: format!("loaded semantic anchor config `{config}`"),
            next: validation_next(can_run_map_commands, vec!["codemap ls .".to_string()]),
        });
    }
    if let Some(domain) = &project.anchors.domain {
        let id = domain.id.as_deref().unwrap_or("repo");
        let path = domain.path.as_deref().unwrap_or(".");
        details.push(domain_anchor_detail(
            project,
            id,
            path,
            can_run_map_commands,
        ));
    }
    for (id, domain) in &project.anchors.domains {
        details.push(domain_anchor_detail(
            project,
            id,
            domain.path.as_deref().unwrap_or("."),
            can_run_map_commands,
        ));
    }
    for (id, concept) in &project.anchors.concepts {
        let mut resolved_files = 0usize;
        let mut missing_exact_files = 0usize;
        let mut glob_patterns = 0usize;
        let mut glob_matches = 0usize;
        for file in &concept.files {
            let rel = map::resolve_anchor_path(project, file);
            if is_glob_like(file) {
                glob_patterns += 1;
                glob_matches += anchor_pattern_match_count(project, file);
            } else if project.files.contains_key(&rel) {
                resolved_files += 1;
            } else {
                missing_exact_files += 1;
            }
        }
        let status = if concept.files.is_empty() || missing_exact_files > 0 {
            "problem"
        } else if concept.invariants.is_empty() || (glob_patterns > 0 && glob_matches == 0) {
            "warning"
        } else {
            "ok"
        };
        details.push(AnchorValidationDetail {
            kind: "concept".to_string(),
            id: id.clone(),
            status: status.to_string(),
            message: format!(
                "role `{}`; exact files resolved: {}; exact files missing: {}; glob matches: {}; invariants: {}",
                concept
                    .role
                    .as_deref()
                    .or(concept.kind.as_deref())
                    .unwrap_or("unspecified"),
                resolved_files,
                missing_exact_files,
                glob_matches,
                concept.invariants.len()
            ),
            next: concept_anchor_next_commands(project, concept, can_run_map_commands),
        });
    }
    for (idx, edge) in project.anchors.boundaries.forbidden.iter().enumerate() {
        let number = idx + 1;
        let from_matches = anchor_pattern_match_count(project, &edge.from);
        let to_matches = anchor_pattern_match_count(project, &edge.to);
        let unsupported_status = edge
            .status
            .as_deref()
            .is_some_and(|status| !matches!(status, "forbidden" | "warn" | "warning"));
        let status = if edge.from.trim().is_empty()
            || edge.to.trim().is_empty()
            || edge.reason.trim().is_empty()
            || unsupported_status
        {
            "problem"
        } else if from_matches == 0 || to_matches == 0 || edge.recovery.is_empty() {
            "warning"
        } else {
            "ok"
        };
        details.push(AnchorValidationDetail {
            kind: "forbidden_boundary".to_string(),
            id: format!("#{number}"),
            status: status.to_string(),
            message: format!(
                "`from` matches {}; `to` matches {}; recovery steps: {}; declared status: {}",
                from_matches,
                to_matches,
                edge.recovery.len(),
                edge.status.as_deref().unwrap_or("forbidden")
            ),
            next: validation_next(
                can_run_map_commands,
                vec![
                    "codemap boundaries".to_string(),
                    "codemap graph --lens boundaries --format mermaid".to_string(),
                ],
            ),
        });
    }
    for (index, command) in project.anchors.verification.default.iter().enumerate() {
        details.push(AnchorValidationDetail {
            kind: "verification_default".to_string(),
            id: format!("#{}", index + 1),
            status: if command.trim().is_empty() {
                "problem"
            } else {
                "ok"
            }
            .to_string(),
            message: command.clone(),
            next: validation_next(
                can_run_map_commands,
                vec!["codemap proof changed".to_string()],
            ),
        });
    }
    details
}

fn domain_anchor_detail(
    project: &crate::model::Project,
    id: &str,
    path: &str,
    can_run_map_commands: bool,
) -> AnchorValidationDetail {
    let rel = repo::normalize_rel_path(path);
    let exists = rel == "." || project.root.join(&rel).is_dir();
    AnchorValidationDetail {
        kind: "domain".to_string(),
        id: id.to_string(),
        status: if exists { "ok" } else { "problem" }.to_string(),
        message: format!(
            "path `{rel}` {}",
            if exists { "exists" } else { "is missing" }
        ),
        next: if exists && can_run_map_commands {
            validation_next(
                true,
                vec![
                    format!("codemap ls {}", shell_quote_arg(&rel)),
                    format!("codemap cone {} --depth 1", shell_quote_arg(&rel)),
                ],
            )
        } else {
            vec!["codemap anchors validate".to_string()]
        },
    }
}

fn concept_anchor_next_commands(
    project: &crate::model::Project,
    concept: &crate::model::AnchorConcept,
    can_run_map_commands: bool,
) -> Vec<String> {
    if !can_run_map_commands {
        return vec!["codemap anchors validate".to_string()];
    }
    let mut next = Vec::new();
    for file in &concept.files {
        let rel = map::resolve_anchor_path(project, file);
        if is_glob_like(file) {
            if anchor_pattern_matches_project(project, file)
                && let Some(prefix) = glob_static_prefix(&rel)
            {
                next.push(format!("codemap files --path {}", shell_quote_arg(&prefix)));
            }
        } else if project.files.contains_key(&rel) {
            next.push(format!("codemap cone {} --depth 1", shell_quote_arg(&rel)));
        }
        if next.len() >= 3 {
            break;
        }
    }
    if next.is_empty() {
        next.push("codemap anchors validate".to_string());
    }
    dedupe_strings(next)
}

fn validation_next(can_run_map_commands: bool, commands: Vec<String>) -> Vec<String> {
    if can_run_map_commands {
        commands
    } else {
        vec!["codemap anchors validate".to_string()]
    }
}
