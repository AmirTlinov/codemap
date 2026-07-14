// Responsibility: side-effect-surface-detection
use crate::map::{code_shape_without_literal_content, quoted_literal_contents, runtime_code_lines};
use crate::model::{EvidenceStrength, FileInfo, Project, Surface};

pub(crate) fn side_effect_surfaces_for_file(project: &Project, file: &FileInfo) -> Vec<Surface> {
    let Ok(text) = std::fs::read_to_string(project.root.join(&file.rel)) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (line_number, line) in runtime_code_lines(&text) {
        let Some((kind, evidence)) = side_effect_kind(&line) else {
            continue;
        };
        out.push(Surface {
            id: format!("surface:side_effect:{kind}:{}:{line_number}", file.rel),
            kind: kind.to_string(),
            path: Some(file.rel.clone()),
            role: Some("side_effect".to_string()),
            evidence: evidence.to_string(),
            strength: EvidenceStrength::Medium,
            count: Some(1),
            examples: vec![format!("{}:{line_number}", file.rel)],
            hidden_count: 0,
        });
    }
    out
}

pub(crate) fn raw_sql_literal_line(line: &str) -> bool {
    raw_sql_literal_kind(line).is_some()
}

fn side_effect_kind(line: &str) -> Option<(&'static str, &'static str)> {
    let code = code_shape_without_literal_content(line);
    if code.contains("fetch(") || code.contains("axios.") {
        Some(("network_call", "static_network_call"))
    } else if code.contains("localStorage.setItem")
        || code.contains("sessionStorage.setItem")
        || code.contains("fs.writeFile")
        || code.contains("std::fs::write")
        || code.contains("os.WriteFile")
    {
        Some(("storage_write", "static_storage_write"))
    } else if matches!(
        raw_sql_literal_kind(line),
        Some("INSERT INTO " | "UPDATE " | "DELETE FROM ")
    ) {
        Some(("database_write", "raw_sql_mutation"))
    } else {
        None
    }
}

fn raw_sql_literal_kind(line: &str) -> Option<&'static str> {
    if !has_raw_sql_execution_context(line) {
        return None;
    }
    let literals = quoted_literal_contents(line)
        .into_iter()
        .map(|literal| literal.to_ascii_uppercase())
        .collect::<Vec<_>>();
    ["SELECT ", "INSERT INTO ", "UPDATE ", "DELETE FROM "]
        .into_iter()
        .find(|needle| literals.iter().any(|literal| literal.contains(needle)))
}

fn has_raw_sql_execution_context(line: &str) -> bool {
    let code = code_shape_without_literal_content(line).to_ascii_lowercase();
    [
        ".query(",
        "query(",
        ".execute(",
        "execute(",
        ".exec(",
        "exec(",
        "sqlx::query",
        "sql!",
        "$queryraw",
        "rawquery",
        "prepare(",
    ]
    .iter()
    .any(|needle| code.contains(needle))
}
