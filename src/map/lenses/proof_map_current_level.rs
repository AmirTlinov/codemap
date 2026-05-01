fn proof_map_current_level_containers(
    project: &Project,
    scope: Option<&str>,
    raw_sensors: bool,
) -> (Vec<ProofSurface>, Vec<ProofSurface>) {
    if scope != Some(".") || raw_sensors {
        return (Vec::new(), Vec::new());
    }
    let mut direct = Vec::new();
    let mut e2e = Vec::new();
    for dir in immediate_child_dirs(project, ".") {
        let Some(container_role) = directory_container_role_surface(project, &dir) else {
            continue;
        };
        if !matches!(container_role.as_str(), "test" | "e2e_test") {
            continue;
        }
        let files = files_under_directory(project, dir.trim_end_matches('/'));
        let test_files = files
            .into_iter()
            .filter(|file| file.has_role("test"))
            .collect::<Vec<_>>();
        if test_files.is_empty() {
            continue;
        }
        let has_e2e = test_files.iter().any(|file| file.has_role("e2e_test"));
        let surface = proof_container_surface(project, &dir, &test_files, has_e2e);
        if has_e2e || container_role == "e2e_test" {
            e2e.push(surface);
        } else {
            direct.push(surface);
        }
    }
    (direct, e2e)
}

fn proof_container_surface(
    _project: &Project,
    dir: &str,
    test_files: &[&FileInfo],
    has_e2e: bool,
) -> ProofSurface {
    let kind = if has_e2e {
        "e2e test container"
    } else {
        "test container"
    };
    ProofSurface {
        command: None,
        path: Some(dir.to_string()),
        evidence: "current_level_proof_container".to_string(),
        strength: EvidenceStrength::Medium,
        reason: format!("{kind} with {} indexed test files", test_files.len()),
        locations: vec![EvidenceLocation::path(dir, "proof_container")],
    }
}
