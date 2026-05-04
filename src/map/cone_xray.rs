fn empty_xray_card(anchor: &FileSummary, unknowns: &[Unknown]) -> XrayCard {
    XrayCard {
        roles: xray_role_surfaces(anchor).into_iter().take(8).collect(),
        inputs: Vec::new(),
        outputs: xray_output_surfaces(anchor).into_iter().take(8).collect(),
        state: Vec::new(),
        side_effects: Vec::new(),
        direct_consumers: Vec::new(),
        mediated_consumers: Vec::new(),
        flow: Vec::new(),
        nearby: Vec::new(),
        proof_hard: Vec::new(),
        proof_direct: Vec::new(),
        proof_mediated: Vec::new(),
        proof_soft: Vec::new(),
        unknowns: unknowns.iter().take(8).cloned().collect(),
    }
}

struct ConeXrayInput<'a> {
    project: &'a Project,
    anchor: &'a FileSummary,
    seed_files: &'a [String],
    declared_env: &'a [EnvDeclaration],
    outgoing: &'a [StructuralEdge],
    incoming: &'a [StructuralEdge],
    proof: &'a [StructuralEdge],
    unknowns: &'a [Unknown],
    limit: usize,
}

fn cone_xray_card(input: ConeXrayInput<'_>) -> XrayCard {
    let limit = input.limit.clamp(3, 12);
    let mut hard = Vec::new();
    let mut direct = Vec::new();
    let mut mediated = Vec::new();
    let mut soft = Vec::new();
    for edge in input.proof {
        match xray_proof_bucket(edge) {
            XrayEvidenceBucket::Hard => hard.push(edge.clone()),
            XrayEvidenceBucket::Direct => direct.push(edge.clone()),
            XrayEvidenceBucket::Mediated => mediated.push(edge.clone()),
            XrayEvidenceBucket::Soft => soft.push(edge.clone()),
        }
    }

    XrayCard {
        roles: xray_role_surfaces(input.anchor)
            .into_iter()
            .take(limit)
            .collect(),
        inputs: input
            .outgoing
            .iter()
            .filter(|edge| xray_input_edge(edge))
            .take(limit)
            .cloned()
            .collect(),
        outputs: xray_output_surfaces(input.anchor)
            .into_iter()
            .take(limit)
            .collect(),
        state: xray_state_surfaces(
            input.project,
            input.anchor,
            input.seed_files,
            input.declared_env,
        )
            .into_iter()
            .take(limit)
            .collect(),
        side_effects: xray_side_effects(input.project, input.seed_files, limit),
        direct_consumers: input
            .incoming
            .iter()
            .filter(|edge| !xray_edge_is_mediated(edge))
            .take(limit)
            .cloned()
            .collect(),
        mediated_consumers: input
            .incoming
            .iter()
            .filter(|edge| xray_edge_is_mediated(edge))
            .take(limit)
            .cloned()
            .collect(),
        flow: xray_flow_steps(input.project, input.seed_files, limit),
        nearby: xray_nearby_surfaces(input.project, input.seed_files, limit),
        proof_hard: hard.into_iter().take(limit).collect(),
        proof_direct: direct.into_iter().take(limit).collect(),
        proof_mediated: mediated.into_iter().take(limit).collect(),
        proof_soft: soft.into_iter().take(limit).collect(),
        unknowns: input.unknowns.iter().take(limit).cloned().collect(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum XrayEvidenceBucket {
    Hard,
    Direct,
    Mediated,
    Soft,
}

fn xray_proof_bucket(edge: &StructuralEdge) -> XrayEvidenceBucket {
    if edge.edge_type == "setup_support_surface" {
        return XrayEvidenceBucket::Soft;
    }
    if xray_edge_is_mediated(edge) {
        return XrayEvidenceBucket::Mediated;
    }
    let base = crate::proof_classification::proof_base_evidence(&edge.evidence);
    if xray_edge_is_soft(edge) {
        return XrayEvidenceBucket::Soft;
    }
    if edge.strength == EvidenceStrength::Hard {
        return XrayEvidenceBucket::Hard;
    }
    if matches!(
        base,
        "test_import"
            | "test_imported_symbol_reference"
            | "test_reexported_symbol_reference"
            | "test_support_import"
            | "test_symbol_reference"
            | "e2e_route"
            | "manifest_script"
            | "schema_package_script"
            | "ci_run_step"
            | "ci_validation_step"
    ) {
        return XrayEvidenceBucket::Direct;
    }
    XrayEvidenceBucket::Soft
}

fn xray_edge_is_soft(edge: &StructuralEdge) -> bool {
    let base = crate::proof_classification::proof_base_evidence(&edge.evidence);
    matches!(
        base,
        "test_name"
            | "e2e_surface_phrase"
            | "e2e_path_surface"
            | "test_surface_phrase"
            | "test_surface_tokens"
            | "script_path_token"
            | "script_surface_match"
            | "proof_neighbor_token_match"
    ) || (edge.strength < EvidenceStrength::High
        && !matches!(
            base,
            "test_import"
                | "test_imported_symbol_reference"
                | "test_reexported_symbol_reference"
                | "test_support_import"
                | "test_symbol_reference"
                | "e2e_route"
        ))
}

fn xray_edge_is_mediated(edge: &StructuralEdge) -> bool {
    edge.evidence.ends_with("_via_direct_consumer")
        || edge.evidence.ends_with("_via_direct_dependency")
        || edge.evidence.ends_with("_via_local_symbol_consumer")
        || edge.evidence.ends_with("_via_cone_depth")
        || edge.evidence.contains("reexport")
        || edge.evidence.contains("barrel")
        || edge.evidence.contains("module_aggregator")
}

fn xray_input_edge(edge: &StructuralEdge) -> bool {
    matches!(
        edge.edge_type.as_str(),
        "imports" | "direct_dependency" | "env_consumer" | "uses_lockfile" | "contract"
    ) || edge.edge_type.contains("dependency")
}

fn xray_role_surfaces(anchor: &FileSummary) -> Vec<Surface> {
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

fn xray_output_surfaces(anchor: &FileSummary) -> Vec<Surface> {
    let mut surfaces = Vec::new();
    for export in &anchor.exports {
        surfaces.push(Surface {
            id: format!("surface:xray_output:{}#{export}", anchor.path),
            kind: "public_export".to_string(),
            path: Some(format!("{}#{export}", anchor.path)),
            role: Some("output".to_string()),
            evidence: "exported_symbol".to_string(),
            strength: EvidenceStrength::High,
            count: Some(1),
            examples: vec![export.clone()],
            hidden_count: 0,
        });
    }
    let private_count = anchor.symbols.iter().filter(|symbol| !symbol.exported).count();
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

fn xray_state_surfaces(
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
                    surfaces.push(surface_from_path(kind, &file.rel, "file_role_or_extension", EvidenceStrength::Medium));
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

fn xray_side_effects(project: &Project, seed_files: &[String], limit: usize) -> Vec<Surface> {
    let mut surfaces = Vec::new();
    for rel in seed_files {
        if let Some(file) = project.files.get(rel) {
            surfaces.extend(side_effect_surfaces_for_file(project, file));
        }
    }
    surfaces.into_iter().take(limit).collect()
}

fn xray_flow_steps(project: &Project, seed_files: &[String], limit: usize) -> Vec<FlowStep> {
    let Some(rel) = seed_files.first() else {
        return Vec::new();
    };
    if !project.files.contains_key(rel) {
        return Vec::new();
    }
    flow_report(project, rel, false, limit)
        .steps
        .into_iter()
        .take(limit)
        .collect()
}

fn xray_nearby_surfaces(project: &Project, seed_files: &[String], limit: usize) -> Vec<Surface> {
    let Some(rel) = seed_files.first() else {
        return Vec::new();
    };
    let Some(parent) = Path::new(rel).parent() else {
        return Vec::new();
    };
    let parent = parent.to_string_lossy();
    let prefix = if parent.is_empty() || parent == "." {
        String::new()
    } else {
        format!("{}/", parent.trim_end_matches('/'))
    };
    let mut surfaces = Vec::new();
    for file in project.files.values() {
        if file.rel == *rel || !file.rel.starts_with(&prefix) {
            continue;
        }
        let rest = file.rel.trim_start_matches(&prefix);
        if rest.contains('/') {
            continue;
        }
        let Some(kind) = xray_nearby_kind(file) else {
            continue;
        };
        surfaces.push(Surface {
            id: format!("surface:xray_nearby:{kind}:{}", file.rel),
            kind: kind.to_string(),
            path: Some(file.rel.clone()),
            role: Some("existing_nearby_surface".to_string()),
            evidence: "same_directory_surface".to_string(),
            strength: EvidenceStrength::Medium,
            count: Some(1),
            examples: vec![file.rel.clone()],
            hidden_count: 0,
        });
    }
    surfaces.sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.path.cmp(&b.path)));
    surfaces.into_iter().take(limit).collect()
}

fn xray_nearby_kind(file: &FileInfo) -> Option<&'static str> {
    let name = Path::new(&file.rel)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if file.has_role("test") {
        Some("test")
    } else if file.has_role("schema_contract") {
        Some("schema")
    } else if file.has_role("repository") {
        Some("repository")
    } else if file.has_role("parser") {
        Some("parser")
    } else if file.has_role("proof_runner") {
        Some("proof_runner")
    } else if file.has_role("doctor") || name.contains("check") || name.contains("validate") {
        Some("validator")
    } else if name.contains("digest") || name.contains("hash") || name.contains("checksum") {
        Some("digest_helper")
    } else if file.has_role("receipt") || file.has_role("witness") {
        Some("witness")
    } else if file.has_role("manifest") || file.has_role("env_config") || file.has_role("runtime_config") {
        Some("config")
    } else if file.has_role("public_boundary") {
        Some("public_boundary")
    } else {
        None
    }
}
