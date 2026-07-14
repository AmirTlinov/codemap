// Responsibility: map-cone-xray-anchor-surfaces
use crate::map::surface_from_path;
use crate::model::{EnvDeclaration, EvidenceStrength, FileSummary, Project, Surface};

pub(crate) fn xray_role_surfaces(anchor: &FileSummary) -> Vec<Surface> {
    let mut roles = std::collections::BTreeSet::new();
    let path = anchor.path.to_ascii_lowercase();
    for role in &anchor.roles {
        match role.as_str() {
            "entrypoint" | "runtime_surface" => {
                roles.insert("entrypoint");
            }
            "adapter" | "controller" | "cli_surface" => {
                roles.insert("adapter");
            }
            "domain" | "state_model" => {
                roles.insert("domain");
            }
            "runtime_state" | "env_config" | "runtime_config" => {
                roles.insert("state");
            }
            "repository" | "persistence" => {
                roles.insert("persistence");
            }
            "renderer_ui" => {
                roles.insert("renderer");
            }
            "render_surface" => {
                roles.insert("render_surface");
            }
            "helper_surface" => {
                roles.insert("helper_surface");
            }
            "proof_surface" => {
                roles.insert("proof_surface");
            }
            "contract_surface" => {
                roles.insert("contract_surface");
            }
            "analysis_surface" => {
                roles.insert("analysis_surface");
            }
            "teach_surface" => {
                roles.insert("teach_surface");
            }
            "proof_runner" | "receipt" | "witness" | "doctor" => {
                roles.insert("proof");
            }
            "manifest" | "schema_contract" | "schema" | "build_ci" => {
                roles.insert("config");
            }
            "test" | "e2e_test" => {
                roles.insert("proof");
            }
            _ => {}
        }
    }
    if roles.is_empty() {
        if anchor.kind == "directory" {
            roles.insert("directory");
        } else if anchor.kind == "source" || !anchor.symbols.is_empty() {
            roles.insert("source");
        } else if anchor.kind == "missing" || path.contains('#') {
            roles.insert("unknown");
        } else {
            roles.insert(anchor.kind.as_str());
        }
    }
    roles
        .into_iter()
        .map(|role| Surface {
            id: format!("surface:xray_role:{}:{role}", anchor.path),
            kind: role.to_string(),
            path: Some(anchor.path.clone()),
            role: Some("anchor_role".to_string()),
            evidence: "surface_hint".to_string(),
            strength: EvidenceStrength::Medium,
            count: Some(1),
            examples: vec![anchor.path.clone()],
            hidden_count: 0,
        })
        .collect()
}

pub(crate) fn xray_output_surfaces(anchor: &FileSummary) -> Vec<Surface> {
    let mut surfaces = Vec::new();
    for export in &anchor.exports {
        // A symbol anchor (`file#sym`) is already the final export path; only file
        // anchors need the `#export` suffix. Without this guard, `cone file#run`
        // produced `file#run#run`.
        let export_path = if anchor.path.contains('#') {
            anchor.path.clone()
        } else {
            format!("{}#{export}", anchor.path)
        };
        surfaces.push(Surface {
            id: format!("surface:xray_output:{export_path}"),
            kind: "public_export".to_string(),
            path: Some(export_path),
            role: Some("output".to_string()),
            evidence: "exported_symbol".to_string(),
            strength: EvidenceStrength::High,
            count: Some(1),
            examples: vec![export.clone()],
            hidden_count: 0,
        });
    }
    let private_count = anchor
        .symbols
        .iter()
        .filter(|symbol| !symbol.exported)
        .count();
    if private_count > 0 {
        surfaces.push(Surface {
            id: format!("surface:xray_output:{}:private_symbols", anchor.path),
            kind: "defined_private_symbols".to_string(),
            path: Some(anchor.path.clone()),
            role: Some("output".to_string()),
            evidence: "symbol_definition".to_string(),
            strength: EvidenceStrength::Hard,
            count: Some(private_count),
            examples: anchor
                .symbols
                .iter()
                .filter(|symbol| !symbol.exported)
                .take(5)
                .map(|symbol| symbol.name.clone())
                .collect(),
            hidden_count: private_count.saturating_sub(5),
        });
    }
    surfaces
}

pub(crate) fn xray_state_surfaces(
    project: &Project,
    anchor: &FileSummary,
    seed_files: &[String],
    declared_env: &[EnvDeclaration],
) -> Vec<Surface> {
    let mut surfaces = Vec::new();
    for env in declared_env {
        surfaces.push(Surface {
            id: format!("surface:xray_state:{}:{}", env.path, env.key),
            kind: "env_key".to_string(),
            path: Some(env.path.clone()),
            role: Some("state".to_string()),
            evidence: "env_file".to_string(),
            strength: EvidenceStrength::Hard,
            count: Some(1),
            examples: vec![format!("{}:{}", env.key, env.line_start)],
            hidden_count: 0,
        });
    }
    for rel in seed_files {
        if let Some(file) = project.files.get(rel) {
            for (role, kind) in [
                ("schema_contract", "schema_state"),
                ("manifest", "manifest_config"),
                ("env_config", "env_config"),
                ("runtime_config", "runtime_config"),
                ("receipt", "receipt_artifact"),
                ("witness", "witness_artifact"),
            ] {
                if file.has_role(role) {
                    surfaces.push(surface_from_path(
                        kind,
                        &file.rel,
                        "file_role_or_extension",
                        EvidenceStrength::Medium,
                    ));
                }
            }
        }
    }
    if surfaces.is_empty() && anchor.kind == "config" {
        surfaces.push(surface_from_path(
            "config_state",
            &anchor.path,
            "file_kind",
            EvidenceStrength::Medium,
        ));
    }
    surfaces
}
