// Responsibility: map-cone-owner-schema-edges
use crate::map::{
    edge_with_path_location, owner_line_containing, package_for_rel, prisma_env_names,
    structural_edge_with_locations,
};
use crate::model::{EvidenceLocation, EvidenceStrength, Project, StructuralEdge};
use crate::repo;
use std::path::Path;

pub(crate) fn owner_schema_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    edges.extend(owner_schema_env_edges(project, rel));
    edges.extend(owner_schema_migration_edges(project, rel));
    edges.extend(owner_prisma_generator_edges(project, rel));
    edges
}

fn owner_schema_env_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let Ok(text) = std::fs::read_to_string(project.root.join(rel)) else {
        return Vec::new();
    };
    let mut edges = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if !line.contains("env(") {
            continue;
        }
        for name in prisma_env_names(line) {
            edges.push(structural_edge_with_locations(
                rel.to_string(),
                format!("env:{name}"),
                "reads_env",
                "prisma_env",
                EvidenceStrength::Hard,
                vec![EvidenceLocation::line(rel, index + 1, "env_reference")],
            ));
        }
    }
    edges
}

fn owner_schema_migration_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let path = Path::new(rel);
    let mut edges = Vec::new();
    if path.file_name().and_then(|name| name.to_str()) == Some("schema.prisma") {
        let migration_prefix = path
            .parent()
            .map(|parent| parent.join("migrations"))
            .and_then(|path| path.to_str().map(repo::normalize_rel_path));
        if let Some(prefix) = migration_prefix {
            for file in project.files.values() {
                if file
                    .rel
                    .starts_with(&format!("{}/", prefix.trim_end_matches('/')))
                {
                    edges.push(edge_with_path_location(
                        rel.to_string(),
                        file.rel.clone(),
                        "schema_migration",
                        "prisma_migration_file",
                        EvidenceStrength::High,
                        file.rel.clone(),
                        "migration_file",
                    ));
                }
            }
        }
    } else if rel.contains("/migrations/")
        && let Some(schema) = path
            .parent()
            .and_then(|parent| parent.parent())
            .map(|parent| parent.join("schema.prisma"))
            .and_then(|path| path.to_str().map(repo::normalize_rel_path))
        && project.files.contains_key(&schema)
    {
        edges.push(edge_with_path_location(
            rel.to_string(),
            schema,
            "migration_schema_owner",
            "prisma_schema_file",
            EvidenceStrength::High,
            rel.to_string(),
            "migration_file",
        ));
    }
    edges
}

fn owner_prisma_generator_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let Ok(text) = std::fs::read_to_string(project.root.join(rel)) else {
        return Vec::new();
    };
    text.lines()
        .enumerate()
        .filter(|(_, line)| line.contains("prisma-client-js"))
        .map(|(index, _)| {
            structural_edge_with_locations(
                rel.to_string(),
                "generated:@prisma/client".to_string(),
                "generates_client",
                "prisma_generator",
                EvidenceStrength::High,
                vec![EvidenceLocation::line(rel, index + 1, "prisma_generator")],
            )
        })
        .collect()
}

pub(crate) fn owner_schema_incoming_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let Some(owner_package) = package_for_rel(project, rel) else {
        return Vec::new();
    };
    project
        .files
        .values()
        .filter(|file| file.rel != rel && repo::is_source_ext(&file.ext))
        .filter(|file| file.imports.contains("@prisma/client"))
        .filter(|file| {
            package_for_rel(project, &file.rel)
                .is_some_and(|package| package.path == owner_package.path)
        })
        .map(|file| {
            structural_edge_with_locations(
                file.rel.clone(),
                rel.to_string(),
                "schema_client_consumer",
                "prisma_client_import",
                EvidenceStrength::High,
                first_line_locations_containing(
                    project,
                    &file.rel,
                    &["@prisma/client", "PrismaClient"],
                    "prisma_client_import",
                ),
            )
        })
        .collect()
}

fn first_line_locations_containing(
    project: &Project,
    rel: &str,
    needles: &[&str],
    kind: &str,
) -> Vec<EvidenceLocation> {
    vec![EvidenceLocation::line(
        rel,
        owner_line_containing(project, rel, needles),
        kind,
    )]
}
