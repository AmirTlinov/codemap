// Responsibility: map-cone-owner-manifest-edges
use crate::map::{
    command_target, first_line_containing, lockfiles_for_package, manifest_file_name,
    owner_workspace_manifest_edges, package_json_scripts, package_public_targets,
    script_target_for_path, structural_edge_with_locations, workspace_manifest_file,
};
use crate::model::{EvidenceLocation, EvidenceStrength, Project, StructuralEdge};

pub(crate) fn owner_manifest_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    let Some(package) = project
        .packages
        .iter()
        .find(|package| package.manifest == rel)
    else {
        if workspace_manifest_file(rel) {
            edges.extend(owner_workspace_manifest_edges(project, rel));
        }
        return edges;
    };
    let package_line = first_line_containing(project, rel, &["\"name\"", "name ="]).unwrap_or(1);
    edges.push(structural_edge_with_locations(
        rel.to_string(),
        format!("package:{}", package.name),
        "declares_package",
        "package_manifest",
        EvidenceStrength::Hard,
        vec![EvidenceLocation::line(
            rel,
            package_line,
            "package_manifest",
        )],
    ));
    for script in project
        .scripts
        .iter()
        .filter(|script| script.path.as_deref() == Some(rel))
        .filter(|_| manifest_file_name(rel) != "package.json")
    {
        let line = script.line_start.unwrap_or(1);
        let script_id = script_target_for_path(rel, &script.name);
        edges.push(structural_edge_with_locations(
            rel.to_string(),
            script_id.clone(),
            "declares_script",
            "script_manifest",
            EvidenceStrength::Hard,
            vec![EvidenceLocation::line(rel, line, "script_manifest")],
        ));
        edges.push(structural_edge_with_locations(
            script_id,
            command_target(&script.command),
            "runs_command",
            "script_manifest",
            EvidenceStrength::Hard,
            vec![EvidenceLocation::line(rel, line, "script_manifest")],
        ));
    }
    for lockfile in lockfiles_for_package(project, package) {
        edges.push(structural_edge_with_locations(
            rel.to_string(),
            lockfile.rel.clone(),
            "uses_lockfile",
            "lockfile",
            EvidenceStrength::High,
            vec![
                EvidenceLocation::path(rel, "package_manifest"),
                EvidenceLocation::path(&lockfile.rel, "lockfile"),
            ],
        ));
    }
    for (name, command, line) in package_json_scripts(project, rel) {
        let script_id = format!("script:{name}");
        edges.push(structural_edge_with_locations(
            rel.to_string(),
            script_id.clone(),
            "declares_script",
            "manifest_script",
            EvidenceStrength::Hard,
            vec![EvidenceLocation::line(rel, line, "package_script")],
        ));
        edges.push(structural_edge_with_locations(
            script_id,
            command_target(&command),
            "runs_command",
            "manifest_script",
            EvidenceStrength::Hard,
            vec![EvidenceLocation::line(rel, line, "package_script")],
        ));
    }
    for edge in project
        .package_edges
        .iter()
        .filter(|edge| edge.from_manifest == rel)
    {
        let target = edge.to_manifest.as_deref().unwrap_or(edge.to.as_str());
        edges.push(structural_edge_with_locations(
            rel.to_string(),
            target.to_string(),
            "package_dependency",
            edge.source.clone(),
            EvidenceStrength::Hard,
            vec![EvidenceLocation::line(
                rel,
                owner_line_containing(
                    project,
                    rel,
                    &[&format!("\"{}\"", edge.dependency), &edge.dependency],
                ),
                "package_dependency",
            )],
        ));
    }
    for target in package_public_targets(project, package) {
        edges.push(structural_edge_with_locations(
            rel.to_string(),
            target,
            "package_export",
            "package_manifest_export",
            EvidenceStrength::Hard,
            vec![EvidenceLocation::line(
                rel,
                first_line_containing(project, rel, &["\"exports\"", "\"bin\""]).unwrap_or(1),
                "package_export",
            )],
        ));
    }
    edges
}

pub(crate) fn owner_manifest_incoming_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    project
        .package_edges
        .iter()
        .filter(|edge| edge.to_manifest.as_deref() == Some(rel))
        .map(|edge| {
            structural_edge_with_locations(
                edge.from_manifest.clone(),
                rel.to_string(),
                "package_consumer",
                edge.source.clone(),
                EvidenceStrength::Hard,
                vec![EvidenceLocation::line(
                    &edge.from_manifest,
                    owner_line_containing(
                        project,
                        &edge.from_manifest,
                        &[&format!("\"{}\"", edge.dependency), &edge.dependency],
                    ),
                    "package_dependency",
                )],
            )
        })
        .collect()
}

pub(crate) fn owner_line_containing(project: &Project, rel: &str, needles: &[&str]) -> usize {
    first_line_containing(project, rel, needles).unwrap_or(1)
}
