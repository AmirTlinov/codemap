// Responsibility: schema-codegen-export-consumer-verification-lineage
mod package;

use super::entity_surface;
use crate::map::{quoted_literal_contents, structural_edge_with_locations, unknown};
use crate::model::{EvidenceLocation, EvidenceStrength, Project, StructuralEdge, Surface, Unknown};
use crate::repo;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Default)]
pub(super) struct CodegenLineage {
    pub(super) declarations: Vec<Surface>,
    pub(super) edges: Vec<StructuralEdge>,
    pub(super) proof: Vec<StructuralEdge>,
    pub(super) unknowns: Vec<Unknown>,
}

pub(super) fn supported_contract_source(project: &Project, rel: &str) -> bool {
    let extension = Path::new(rel).extension().and_then(|ext| ext.to_str());
    if matches!(
        extension,
        Some("graphql" | "gql" | "proto" | "avsc" | "prisma")
    ) {
        return true;
    }
    if !matches!(extension, Some("yaml" | "yml" | "json")) {
        return false;
    }
    let contract_path = rel
        .split('/')
        .any(|part| matches!(part, "contracts" | "openapi" | "schemas"));
    let contract_role = project
        .files
        .get(rel)
        .is_some_and(|file| file.has_role("schema_contract"));
    let contract_header = project.read_indexed_text(rel).is_some_and(|text| {
        text.lines().take(30).any(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("openapi:")
                || trimmed.starts_with("asyncapi:")
                || trimmed.starts_with("\"$schema\"")
        })
    });
    contract_path || contract_role || contract_header
}

pub(super) fn codegen_lineage(project: &Project, rel: &str) -> CodegenLineage {
    let mut out = CodegenLineage::default();
    let mut found_consumer = false;
    super::declarations::contract_declarations(project, rel, &mut out.declarations);
    for file in project.files.values().filter(|file| {
        !file.has_role("test")
            && !file.has_role("test_support")
            && (repo::is_source_ext(&file.ext) || file.has_role("manifest"))
    }) {
        let Some(text) = project.read_indexed_text(&file.rel) else {
            continue;
        };
        if !quoted_literal_contents(&text)
            .iter()
            .any(|literal| repo::normalize_rel_path(literal.trim_start_matches("./")) == rel)
        {
            continue;
        }
        out.edges.push(structural_edge_with_locations(
            file.rel.clone(),
            rel.to_string(),
            "consumes",
            "exact_codegen_input_path",
            EvidenceStrength::High,
            vec![EvidenceLocation::line(
                &file.rel,
                line_containing(&text, rel),
                "codegen_input",
            )],
        ));
        found_consumer = true;
        let generated_paths = generated_paths(project, &text, rel);
        if generated_paths.is_empty() {
            out.unknowns.push(unknown(
                "runtime_generated_schema",
                Some(&file.rel),
                None,
                "code generation input is static but its output artifact is computed or absent",
                "generated lineage stops instead of inventing an output path",
                Some(format!("codemap cone {}", file.rel)),
            ));
        }
        for generated in generated_paths {
            let anchor = format!("generated:{generated}");
            out.declarations.push(entity_surface(
                anchor.clone(),
                "generated_artifact",
                &generated,
                1,
                "static_codegen_output",
            ));
            out.edges.push(structural_edge_with_locations(
                file.rel.clone(),
                anchor.clone(),
                "generates",
                "exact_codegen_output_path",
                EvidenceStrength::High,
                vec![EvidenceLocation::line(
                    &file.rel,
                    line_containing(
                        &text,
                        Path::new(&generated)
                            .file_name()
                            .and_then(|v| v.to_str())
                            .unwrap_or(&generated),
                    ),
                    "codegen_output",
                )],
            ));
            package::add_package_export_and_consumers(project, &generated, &anchor, &mut out);
            add_generation_verification(project, &file.rel, &generated, &anchor, &mut out);
        }
    }
    if !found_consumer {
        out.unknowns.push(unknown(
            "contract_codegen_consumer_missing",
            Some(rel),
            None,
            "contract source has no exact static code generation consumer",
            "lineage remains open instead of attaching lexically similar generated files",
            Some(format!("codemap contract {rel} --all")),
        ));
    }
    out
}

fn generated_paths(project: &Project, text: &str, source: &str) -> Vec<String> {
    let literals = quoted_literal_contents(text);
    let mut candidates = BTreeSet::new();
    for literal in &literals {
        let normalized = repo::normalize_rel_path(literal.trim_start_matches("./"));
        if normalized != source && project.files.contains_key(&normalized) {
            candidates.insert(normalized);
        }
    }
    let dirs = literals
        .iter()
        .filter(|literal| literal.contains('/') && Path::new(literal).extension().is_none());
    let files = literals
        .iter()
        .filter(|literal| generated_extension(literal));
    for dir in dirs {
        for file in files.clone() {
            let joined =
                repo::normalize_rel_path(&format!("{}/{}", dir.trim_end_matches('/'), file));
            if project.files.contains_key(&joined) {
                candidates.insert(joined.clone());
            }
            if let Some(ts) = joined
                .strip_suffix(".d.ts")
                .map(|base| format!("{base}.ts"))
                && project.files.contains_key(&ts)
            {
                candidates.insert(ts);
            }
        }
    }
    candidates
        .into_iter()
        .filter(|path| generated_extension(path))
        .collect()
}

fn generated_extension(path: &str) -> bool {
    matches!(
        Path::new(path).extension().and_then(|ext| ext.to_str()),
        Some("ts" | "js" | "rs" | "go" | "py" | "java" | "kt" | "swift")
    )
}

fn add_generation_verification(
    project: &Project,
    generator: &str,
    generated: &str,
    anchor: &str,
    out: &mut CodegenLineage,
) {
    let mut found = false;
    for file in project
        .files
        .values()
        .filter(|file| file.has_role("manifest"))
    {
        let Some(text) = project.read_indexed_text(&file.rel) else {
            continue;
        };
        if text.contains(generated)
            && (text.contains("git diff") || text.contains("cmp ") || text.contains("diff --"))
        {
            let edge = structural_edge_with_locations(
                format!("verification:{}:generation-diff", file.rel),
                anchor.to_string(),
                "verifies_directly",
                "generation_diff_manifest_command",
                EvidenceStrength::Hard,
                vec![EvidenceLocation::line(
                    &file.rel,
                    line_containing(&text, generated),
                    "generation_check",
                )],
            );
            out.proof.push(edge.clone());
            out.edges.push(edge);
            found = true;
        }
    }
    for script in &project.scripts {
        if script.command.contains(generated)
            && (script.command.contains("git diff")
                || script.command.contains("cmp ")
                || script.name.contains("check"))
        {
            let owner = format!(
                "verification:{}:{}",
                script.path.as_deref().unwrap_or("manifest"),
                script.name
            );
            let edge = structural_edge_with_locations(
                owner,
                anchor.to_string(),
                "verifies_directly",
                "generation_diff_command",
                EvidenceStrength::Hard,
                script
                    .path
                    .as_deref()
                    .map(|path| {
                        vec![EvidenceLocation::line(
                            path,
                            script.line_start.unwrap_or(1),
                            "generation_check",
                        )]
                    })
                    .unwrap_or_default(),
            );
            out.proof.push(edge.clone());
            out.edges.push(edge);
            found = true;
        }
    }
    if !found {
        out.unknowns.push(unknown(
            "generation_verification_missing",
            Some(generator),
            None,
            "generated contract artifact has no exact regeneration-diff verification command",
            "lineage exposes the missing generation check instead of attaching neighboring tests",
            Some(format!("codemap proof {generator}")),
        ));
    }
}

fn line_containing(text: &str, needle: &str) -> usize {
    text.lines()
        .position(|line| line.contains(needle))
        .map(|index| index + 1)
        .unwrap_or(1)
}
