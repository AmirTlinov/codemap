// Responsibility: diff-map-line-surfaces
use crate::map::{
    code_shape_without_literal_content, env_declaration, find_all, quoted_literal_at,
    static_env_names,
};
use crate::model::{EnvSurface, EvidenceLocation, EvidenceStrength, Project, ProofSurface};

pub(crate) fn env_surfaces_from_diff_line(
    project: &Project,
    rel: &str,
    line_number: usize,
    code: &str,
    evidence: &str,
) -> Vec<EnvSurface> {
    static_env_names(code)
        .into_iter()
        .map(|name| EnvSurface {
            name,
            used_by: rel.to_string(),
            declaration: env_declaration(project, rel),
            evidence: evidence.to_string(),
            strength: EvidenceStrength::High,
            locations: vec![EvidenceLocation::line(rel, line_number, "env_reference")],
        })
        .collect()
}

pub(crate) fn proof_surfaces_from_diff_line(
    rel: &str,
    line_number: usize,
    code: &str,
    evidence: &str,
) -> Vec<ProofSurface> {
    if !diff_path_can_carry_proof(rel) {
        return Vec::new();
    }
    e2e_route_visits_from_line(code)
        .into_iter()
        .map(|route| ProofSurface {
            command: None,
            path: Some(rel.to_string()),
            target_anchor: Some(rel.to_string()),
            evidence: evidence.to_string(),
            strength: EvidenceStrength::High,
            reason: format!("e2e visits runtime route {route}"),
            locations: vec![EvidenceLocation::line(rel, line_number, "e2e_route")],
        })
        .collect()
}

fn diff_path_can_carry_proof(rel: &str) -> bool {
    let lower = rel.to_ascii_lowercase();
    matches!(
        std::path::Path::new(rel)
            .extension()
            .and_then(|ext| ext.to_str()),
        Some("ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs")
    ) && (lower.contains("/test")
        || lower.contains("/spec")
        || lower.contains("/e2e")
        || lower.contains(".test.")
        || lower.contains(".spec.")
        || lower.contains(".e2e."))
}

fn e2e_route_visits_from_line(line: &str) -> Vec<String> {
    let code = code_shape_without_literal_content(line);
    let mut out = Vec::new();
    for start in find_all(&code, ".goto(") {
        let receiver = code[..start].trim_end();
        if !receiver.ends_with("page") {
            continue;
        }
        let arg_start = start + ".goto(".len();
        if let Some(path) = quoted_literal_at(&line[arg_start..])
            && path.starts_with('/')
        {
            out.push(path);
        }
    }
    out.sort();
    out.dedup();
    out
}
