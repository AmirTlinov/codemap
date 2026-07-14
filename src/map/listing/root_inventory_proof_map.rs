// Responsibility: map-listing-root-inventory-proof-map
use crate::map::{
    ancestor_paths, expand_with_concrete_limit, inventory_root_script_edges,
    inventory_top_level_dirs, javascript_script_command, json_key_line, manifest_command_prefix,
    manifest_dir_for_rel, manifest_file_name, manifest_script_evidence,
    manifest_script_is_proof_or_support_relevant, proof_surface, truncate_with_hidden,
    unique_proof_commands, unique_proof_surfaces,
};
use crate::model::{
    EvidenceLocation, EvidenceStrength, HiddenGroup, ProofMapReport, ProofSurface, Unknown,
};
use crate::repo;
use std::path::Path;

pub(crate) fn root_inventory_proof_map_report(
    root: &Path,
    files: &[String],
    limit: usize,
) -> ProofMapReport {
    let limit = limit.max(1);
    let expand_raw = "codemap proof-map . --raw-sensors";
    let expand_larger = "codemap proof-map . --limit <larger-number>";
    let mut hidden = Vec::new();

    let direct_root_files = files.iter().filter(|rel| !rel.contains('/')).count();
    if files.len() > direct_root_files {
        hidden.push(HiddenGroup {
            reason: "recursive proof seeds hidden at root scope".to_string(),
            count: files.len() - direct_root_files,
            expand: expand_with_concrete_limit(expand_raw, files.len()),
        });
    }

    let mut hard = Vec::new();
    let mut setup_support = Vec::new();
    for surface in inventory_proof_script_surfaces(root, files) {
        match crate::proof_classification::proof_surface_class(&surface) {
            crate::proof_classification::ProofSurfaceClass::SetupSupport => {
                setup_support.push(surface)
            }
            crate::proof_classification::ProofSurfaceClass::Hard => hard.push(surface),
            _ => hard.push(surface),
        }
    }
    let mut direct_evidence = inventory_current_level_proof_containers(files);

    hard = unique_proof_surfaces(hard);
    setup_support = unique_proof_surfaces(setup_support);
    direct_evidence = unique_proof_surfaces(direct_evidence);
    sort_proof_surfaces_for_inventory(&mut hard);
    sort_proof_surfaces_for_inventory(&mut setup_support);
    sort_proof_surfaces_for_inventory(&mut direct_evidence);

    let commands = unique_proof_commands(hard.to_vec());

    truncate_with_hidden(
        &mut hard,
        limit,
        &mut hidden,
        "runnable verification surfaces hidden by limit",
        expand_larger,
    );
    truncate_with_hidden(
        &mut direct_evidence,
        limit,
        &mut hidden,
        "direct linked surfaces hidden by limit",
        expand_larger,
    );
    truncate_with_hidden(
        &mut setup_support,
        limit,
        &mut hidden,
        "setup/support verification surfaces hidden by limit",
        expand_larger,
    );

    let unknowns = vec![Unknown {
        kind: "bounded_root_inventory".to_string(),
        path: Some(".".to_string()),
        line_start: None,
        reason: "cold root proof-map used path, manifest, script, and CI inventory only; recursive file-level verification sensors were not expanded".to_string(),
        effect: "file-level direct imports, route ownership, and semantic verification wiring remain hidden until raw-sensors expansion".to_string(),
        expand: Some(expand_raw.to_string()),
    }];

    ProofMapReport {
        kind: "proof_map_report",
        schema_version: "5",
        selector: ".".to_string(),
        scope: Some(".".to_string()),
        changed: Vec::new(),
        hard,
        direct_evidence,
        mediated_evidence: Vec::new(),
        soft_evidence: Vec::new(),
        setup_support,
        missing_direct: Vec::new(),
        commands,
        wiring: Vec::new(),
        fallback: Vec::new(),
        unknowns,
        hidden,
        expand: vec!["codemap proof .".to_string()],
    }
}

fn inventory_proof_script_surfaces(root: &Path, files: &[String]) -> Vec<ProofSurface> {
    let mut out = Vec::new();
    out.extend(inventory_root_command_surfaces(root, files));
    for rel in files {
        match manifest_file_name(rel) {
            "package.json" => out.extend(inventory_package_json_proof_surfaces(root, rel)),
            "Cargo.toml" => out.push(inventory_builtin_command_surface(
                rel,
                format!("{}cargo test", manifest_command_prefix(rel)),
                "cargo_manifest_command",
                "Cargo.toml detected",
            )),
            "go.mod" => out.push(inventory_builtin_command_surface(
                rel,
                format!("{}go test ./...", manifest_command_prefix(rel)),
                "go_manifest_command",
                "go.mod detected",
            )),
            "pyproject.toml" => out.push(inventory_builtin_command_surface(
                rel,
                format!("{}pytest", manifest_command_prefix(rel)),
                "python_manifest_command",
                "pyproject.toml detected",
            )),
            "Package.swift" => out.push(inventory_builtin_command_surface(
                rel,
                format!("{}swift test", manifest_command_prefix(rel)),
                "swift_manifest_command",
                "Package.swift detected",
            )),
            _ => {}
        }
    }
    unique_proof_surfaces(out)
}

fn inventory_root_command_surfaces(root: &Path, files: &[String]) -> Vec<ProofSurface> {
    let (_, edges) = inventory_root_script_edges(root, files);
    edges
        .into_iter()
        .filter(|edge| edge.edge_type == "runs_command")
        .filter(|edge| !edge.to.starts_with("ci_run_block:"))
        .filter_map(|edge| {
            let command = edge.to.strip_prefix("command:")?.trim();
            if command.is_empty() {
                return None;
            }
            let path = edge
                .locations
                .first()
                .map(|location| location.path.clone())
                .filter(|path| path != "aggregate")
                .or_else(|| Some(edge.from.clone()));
            if inventory_command_edge_is_manifest_body(&path) {
                return None;
            }
            Some(proof_surface(
                Some(command.to_string()),
                path,
                edge.evidence.as_str(),
                edge.strength,
                "declared script or CI run command from root inventory".to_string(),
                edge.locations,
            ))
        })
        .collect()
}

fn inventory_command_edge_is_manifest_body(path: &Option<String>) -> bool {
    path.as_deref().is_some_and(|path| {
        matches!(
            manifest_file_name(path),
            "package.json" | "Cargo.toml" | "go.mod" | "pyproject.toml" | "Package.swift"
        )
    })
}

fn inventory_package_json_proof_surfaces(root: &Path, rel: &str) -> Vec<ProofSurface> {
    let Ok(text) = std::fs::read_to_string(root.join(rel)) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let Some(scripts) = value.get("scripts").and_then(|scripts| scripts.as_object()) else {
        return Vec::new();
    };
    let runner = inventory_javascript_runner_for_manifest(root, rel);
    let mut entries = scripts
        .iter()
        .filter_map(|(name, value)| {
            let body = value.as_str()?.trim();
            if body.is_empty() || !manifest_script_is_proof_or_support_relevant(name, body) {
                return None;
            }
            let command = format!(
                "{}{}",
                manifest_command_prefix(rel),
                javascript_script_command(&runner, name)
            );
            let evidence = manifest_script_evidence(name, body);
            Some(proof_surface(
                Some(command),
                Some(rel.to_string()),
                evidence,
                EvidenceStrength::Hard,
                format!("package manifest defines `{name}` script: {body}"),
                vec![EvidenceLocation::line(
                    rel,
                    json_key_line(&text, name).unwrap_or(1),
                    "package_script",
                )],
            ))
        })
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| {
        a.command
            .cmp(&b.command)
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.evidence.cmp(&b.evidence))
    });
    entries
}

fn inventory_builtin_command_surface(
    rel: &str,
    command: String,
    evidence: &str,
    reason: &str,
) -> ProofSurface {
    proof_surface(
        Some(command),
        Some(rel.to_string()),
        evidence,
        EvidenceStrength::Hard,
        reason.to_string(),
        vec![EvidenceLocation::line(rel, 1, evidence)],
    )
}

fn inventory_javascript_runner_for_manifest(root: &Path, rel: &str) -> String {
    for ancestor in ancestor_paths(&manifest_dir_for_rel(rel)) {
        let dir = if ancestor == "." {
            root.to_path_buf()
        } else {
            root.join(&ancestor)
        };
        if dir.join("pnpm-workspace.yaml").exists() || dir.join("pnpm-lock.yaml").exists() {
            return "pnpm".to_string();
        }
        if dir.join("yarn.lock").exists() {
            return "yarn".to_string();
        }
        if dir.join("bun.lockb").exists() {
            return "bun".to_string();
        }
        if dir.join("package-lock.json").exists() {
            return "npm".to_string();
        }
    }
    "npm".to_string()
}

fn inventory_current_level_proof_containers(files: &[String]) -> Vec<ProofSurface> {
    let mut out = Vec::new();
    for dir in inventory_top_level_dirs(files) {
        let Some(role) = inventory_proof_container_role(files, &dir) else {
            continue;
        };
        let test_count = files
            .iter()
            .filter(|rel| rel.starts_with(&dir) && inventory_path_is_test_file(rel))
            .count();
        if test_count == 0 {
            continue;
        }
        let has_e2e = role == "e2e_test"
            || files
                .iter()
                .any(|rel| rel.starts_with(&dir) && inventory_path_is_e2e_test(rel));
        let kind = if has_e2e {
            "e2e test container"
        } else {
            "test container"
        };
        out.push(proof_surface(
            None,
            Some(dir.clone()),
            "current_level_proof_container",
            EvidenceStrength::Medium,
            format!("{kind} with {test_count} candidate test files"),
            vec![EvidenceLocation::path(&dir, "proof_container")],
        ));
    }
    out
}

fn inventory_proof_container_role(files: &[String], dir: &str) -> Option<&'static str> {
    let name = dir.trim_end_matches('/').to_ascii_lowercase();
    if matches!(name.as_str(), "e2e" | "e2e-tests" | "playwright") {
        return Some("e2e_test");
    }
    if matches!(
        name.as_str(),
        "test" | "tests" | "__tests__" | "spec" | "specs"
    ) {
        if files
            .iter()
            .any(|rel| rel.starts_with(dir) && inventory_path_is_e2e_test(rel))
        {
            return Some("e2e_test");
        }
        return Some("test");
    }
    None
}

fn inventory_path_is_test_file(rel: &str) -> bool {
    let lower = rel.to_ascii_lowercase();
    let name = Path::new(&lower)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(lower.as_str());
    let ext = Path::new(&lower)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");
    let inside_test_dir = lower.starts_with("test/")
        || lower.starts_with("tests/")
        || lower.starts_with("__tests__/")
        || lower.contains("/tests/")
        || lower.contains("/__tests__/");
    matches!(
        name,
        "test.rs" | "tests.rs" | "conftest.py" | "pytest.ini" | "vitest.config.ts"
    ) || name.ends_with(".test.ts")
        || name.ends_with(".test.tsx")
        || name.ends_with(".test.js")
        || name.ends_with(".test.jsx")
        || name.ends_with(".spec.ts")
        || name.ends_with(".spec.tsx")
        || name.ends_with(".spec.js")
        || name.ends_with(".spec.jsx")
        || name.ends_with("_test.py")
        || name.ends_with("_test.go")
        || (inside_test_dir && repo::is_source_ext(ext))
}

fn inventory_path_is_e2e_test(rel: &str) -> bool {
    let lower = rel.to_ascii_lowercase();
    lower.starts_with("e2e/")
        || lower.contains("/e2e/")
        || lower.contains("e2e.")
        || lower.contains(".e2e.")
        || lower.contains("playwright")
        || lower.contains("cypress")
}

fn sort_proof_surfaces_for_inventory(values: &mut [ProofSurface]) {
    values.sort_by(|a, b| {
        a.command
            .cmp(&b.command)
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.evidence.cmp(&b.evidence))
            .then_with(|| a.reason.cmp(&b.reason))
    });
}
