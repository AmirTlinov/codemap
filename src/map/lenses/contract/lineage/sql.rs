// Responsibility: exact-sql-table-field-and-reference-lineage
use super::entity_surface;
use crate::map::{structural_edge_with_locations, unknown};
use crate::model::{EvidenceLocation, EvidenceStrength, Project, StructuralEdge, Surface, Unknown};
use crate::repo;

#[derive(Default)]
pub(super) struct SqlLineage {
    pub(super) declarations: Vec<Surface>,
    pub(super) edges: Vec<StructuralEdge>,
    pub(super) consumer_files: Vec<String>,
    pub(super) unknowns: Vec<Unknown>,
}

pub(super) fn sql_lineage(project: &Project, rel: &str) -> SqlLineage {
    let Some(text) = project.read_indexed_text(rel) else {
        return SqlLineage::default();
    };
    let declarations = sql_declarations(rel, &text);
    let tables = declarations
        .iter()
        .filter(|surface| surface.kind == "table")
        .filter_map(|surface| surface.id.strip_prefix("table:"))
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut out = SqlLineage {
        declarations,
        ..SqlLineage::default()
    };
    for table in tables {
        let anchor = format!("table:{table}");
        let line = first_line(&text, &table);
        out.edges.push(structural_edge_with_locations(
            rel.to_string(),
            anchor.clone(),
            "declares",
            "sql_create_table",
            EvidenceStrength::Hard,
            vec![EvidenceLocation::line(rel, line, "table_declaration")],
        ));
        for file in project.files.values().filter(|file| {
            file.rel != rel && repo::is_source_ext(&file.ext) && !file.has_role("test")
        }) {
            let Some(body) = project.read_indexed_text(&file.rel) else {
                continue;
            };
            if !exact_identifier_or_literal(&body, &table) {
                continue;
            }
            let upper = body.to_ascii_uppercase();
            let line = first_line(&body, &table);
            let mut linked = false;
            if ["INSERT INTO", "UPDATE ", "DELETE FROM"]
                .iter()
                .any(|keyword| upper.contains(keyword))
            {
                out.edges
                    .push(table_reference_edge(&file.rel, &anchor, "writes", line));
                linked = true;
            }
            if upper.contains("SELECT ") {
                out.edges
                    .push(table_reference_edge(&file.rel, &anchor, "reads", line));
                linked = true;
            }
            if linked {
                out.consumer_files.push(file.rel.clone());
                if body.contains("'BEGIN'") || body.contains("\"BEGIN\"") {
                    out.edges.push(structural_edge_with_locations(
                        file.rel.clone(),
                        format!("transaction_group:{}", file.rel),
                        "crosses_boundary",
                        "static_sql_statement_grouping",
                        EvidenceStrength::Medium,
                        vec![EvidenceLocation::line(
                            &file.rel,
                            body.lines()
                                .position(|line| line.contains("BEGIN"))
                                .map(|index| index + 1)
                                .unwrap_or(line),
                            "transaction_statement",
                        )],
                    ));
                }
                if let Some(dynamic_line) = body
                    .lines()
                    .position(|line| line.contains("${") && line.contains("table"))
                {
                    out.unknowns.push(unknown(
                        "dynamic_sql_table",
                        Some(&file.rel),
                        Some(dynamic_line + 1),
                        "SQL table identifier can be overridden or interpolated at runtime",
                        "lineage retains the exact default table and stops at the runtime override",
                        Some(format!("codemap cone {}", file.rel)),
                    ));
                }
            }
        }
        if !out.consumer_files.iter().any(|consumer| {
            out.edges
                .iter()
                .any(|edge| edge.from == *consumer && edge.to == anchor)
        }) {
            out.unknowns.push(unknown(
                "table_consumer_missing",
                Some(rel),
                Some(line),
                "declared SQL table has no exact static source consumer",
                "lineage exposes the missing table consumer instead of using name overlap",
                Some(format!("codemap contract {rel} --all")),
            ));
        }
    }
    out
}

fn sql_declarations(rel: &str, text: &str) -> Vec<Surface> {
    let lines = text.lines().collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut index = 0usize;
    while index < lines.len() {
        let upper = lines[index].trim().to_ascii_uppercase();
        if !upper.starts_with("CREATE TABLE ") {
            index += 1;
            continue;
        }
        let table = table_name(lines[index]);
        let Some(table) = table else {
            index += 1;
            continue;
        };
        out.push(entity_surface(
            format!("table:{table}"),
            "table",
            rel,
            index + 1,
            "sql_create_table",
        ));
        index += 1;
        while index < lines.len() && !lines[index].trim_start().starts_with(");") {
            if let Some(field) = sql_field_name(lines[index]) {
                out.push(entity_surface(
                    format!("field:{table}.{field}"),
                    "field",
                    rel,
                    index + 1,
                    "sql_table_field",
                ));
            }
            index += 1;
        }
        index += 1;
    }
    out
}

fn table_name(line: &str) -> Option<String> {
    let tokens = line
        .split_whitespace()
        .map(|token| token.trim_matches(['"', '`', '(', ';']))
        .collect::<Vec<_>>();
    let table_index = tokens
        .iter()
        .position(|token| token.eq_ignore_ascii_case("TABLE"))?;
    let mut index = table_index + 1;
    if tokens
        .get(index)
        .is_some_and(|token| token.eq_ignore_ascii_case("IF"))
    {
        index += 3;
    }
    tokens
        .get(index)
        .filter(|name| identifier(name))
        .map(|name| (*name).to_string())
}

fn sql_field_name(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches(',');
    let name = trimmed.split_whitespace().next()?.trim_matches(['"', '`']);
    let upper = name.to_ascii_uppercase();
    (!matches!(
        upper.as_str(),
        "PRIMARY" | "UNIQUE" | "FOREIGN" | "CONSTRAINT" | "CHECK"
    ) && identifier(name))
    .then(|| name.to_string())
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn exact_identifier_or_literal(text: &str, name: &str) -> bool {
    crate::map::identifier_ranges(text, name).next().is_some()
        || crate::map::quoted_literal_contents(text)
            .iter()
            .any(|literal| literal == name)
}

fn first_line(text: &str, needle: &str) -> usize {
    text.lines()
        .position(|line| line.contains(needle))
        .map(|index| index + 1)
        .unwrap_or(1)
}

fn table_reference_edge(rel: &str, table: &str, relation: &str, line: usize) -> StructuralEdge {
    structural_edge_with_locations(
        rel.to_string(),
        table.to_string(),
        relation,
        "static_sql_table_binding",
        EvidenceStrength::Medium,
        vec![EvidenceLocation::line(rel, line, "table_reference")],
    )
}
