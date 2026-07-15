// Responsibility: render-helpers
use crate::model::{ProofSurface, StructuralEdge, Surface, Unknown};
use crate::render::{
    AnchorPathDisplay, brief, edge_location_summary_with_paths, proof_location_summary,
    root_aware_expand,
};

// Repeated provenance/boundary disclaimers. Agents internalize these once, so in
// `--brief` mode they are suppressed to save tokens; the default keeps them.
pub(crate) fn disclaimer(text: &str) {
    if !brief() {
        println!("{text}\n");
    }
}

pub(crate) fn section(title: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    println!("\n## {title}\n");
    let values = values
        .iter()
        .map(|value| root_aware_expand(value))
        .collect::<Vec<_>>();
    println!("{}", bullet(&values, true, Some(20)));
}

pub(crate) fn plain_section(title: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    println!("\n## {title}\n");
    println!("{}", bullet(values, false, Some(20)));
}

pub(crate) fn unknown_section(values: &[Unknown]) {
    if values.is_empty() {
        return;
    }
    println!("\n## Unknown\n");
    let mut grouped: std::collections::BTreeMap<&str, Vec<&Unknown>> =
        std::collections::BTreeMap::new();
    for unknown in values {
        grouped
            .entry(unknown.kind.as_str())
            .or_default()
            .push(unknown);
    }
    for (kind, unknowns) in grouped {
        println!("- `{kind}`");
        let visible = unknowns.len().min(4);
        for unknown in unknowns.iter().take(visible) {
            println!("  - where: {}", unknown_where(unknown));
            println!("    reason: {}", unknown.reason);
            println!("    effect: {}", unknown.effect);
            if let Some(expand) = &unknown.expand {
                println!("    expand: `{}`", root_aware_expand(expand));
            }
        }
        let hidden = unknowns.len().saturating_sub(visible);
        if hidden > 0 {
            println!("  - hidden unknowns: `{hidden}`");
        }
    }
}

pub(crate) fn proof_surface_section(title: &str, proofs: &[ProofSurface]) {
    if proofs.is_empty() {
        return;
    }
    println!("\n## {title}");
    let mut grouped: std::collections::BTreeMap<String, Vec<&ProofSurface>> =
        std::collections::BTreeMap::new();
    for proof in proofs {
        grouped
            .entry(
                proof
                    .command
                    .clone()
                    .unwrap_or_else(|| "no command".to_string()),
            )
            .or_default()
            .push(proof);
    }
    for (command, proofs) in grouped {
        println!("\n### `{command}`");
        for proof in proofs {
            let path = proof
                .path
                .as_ref()
                .map(|path| code(path))
                .unwrap_or_else(|| "`none`".to_string());
            println!(
                "- {path}{} [{}; {}] {} - {}",
                proof_target_suffix(proof),
                public_evidence_label(&proof.evidence),
                format!("{:?}", proof.strength).to_ascii_lowercase(),
                proof_location_summary(&proof.locations),
                proof.reason
            );
        }
    }
}

pub(crate) fn proof_command_summary_section(title: &str, proofs: &[ProofSurface]) {
    const COMMAND_SUMMARY_LIMIT: usize = 8;

    if proofs.is_empty() {
        return;
    }
    println!("\n## {title}\n");
    let mut commands = std::collections::BTreeSet::new();
    for proof in proofs {
        let Some(command) = &proof.command else {
            continue;
        };
        commands.insert(command.clone());
    }
    if commands.is_empty() {
        println!("- no command inferred");
        return;
    }
    let total = commands.len();
    for command in commands.iter().take(COMMAND_SUMMARY_LIMIT) {
        println!("- `{command}`");
    }
    let hidden = total.saturating_sub(COMMAND_SUMMARY_LIMIT);
    if hidden > 0 {
        println!("- hidden runnable commands: `{hidden}`");
    }
}

pub(crate) fn proof_target_suffix(proof: &ProofSurface) -> String {
    let Some(target) = proof.target_anchor.as_deref() else {
        return String::new();
    };
    if proof.path.as_deref() == Some(target) {
        String::new()
    } else {
        format!(" -> {}", code(target))
    }
}

pub(crate) fn contract_exports_section(title: &str, surfaces: &[Surface]) {
    if surfaces.is_empty() {
        return;
    }
    println!("\n## {title}");
    let mut grouped: std::collections::BTreeMap<String, Vec<&Surface>> =
        std::collections::BTreeMap::new();
    for surface in surfaces {
        let path = surface.path.as_deref().unwrap_or("aggregate");
        let file = path.split_once('#').map(|(file, _)| file).unwrap_or(path);
        grouped.entry(file.to_string()).or_default().push(surface);
    }
    for (file, surfaces) in grouped {
        println!("\n- `{file}`");
        for surface in surfaces {
            let path = surface.path.as_deref().unwrap_or("aggregate");
            let label = path
                .split_once('#')
                .map(|(_, symbol)| symbol.to_string())
                .unwrap_or_else(|| {
                    surface
                        .examples
                        .first()
                        .cloned()
                        .unwrap_or_else(|| surface.kind.clone())
                });
            println!(
                "  - `{}` [{}; {}]",
                label,
                public_evidence_label(&surface.evidence),
                format!("{:?}", surface.strength).to_ascii_lowercase()
            );
            if surface.hidden_count > 0 {
                println!("    - additional examples: {}", surface.hidden_count);
            }
        }
    }
}

pub(crate) fn hidden_section(hidden: &[crate::model::HiddenGroup]) {
    if hidden.is_empty() {
        return;
    }
    println!("\n## Hidden\n");
    for hidden in hidden {
        println!("- {}: {}", hidden.reason, hidden.count);
        println!("  expand: `{}`", root_aware_expand(&hidden.expand));
    }
}

pub(crate) fn unknown_where(unknown: &Unknown) -> String {
    unknown
        .path
        .as_ref()
        .map(|path| {
            if let Some(line) = unknown.line_start {
                code(&format!("{path}:{line}"))
            } else {
                code(path)
            }
        })
        .unwrap_or_else(|| "none".to_string())
}

pub(crate) fn grouped_edge_list(title: &str, edges: &[StructuralEdge], limit: usize) {
    grouped_edge_list_with_paths(title, edges, limit, &AnchorPathDisplay::new(""));
}

pub(crate) fn grouped_edge_list_with_paths(
    title: &str,
    edges: &[StructuralEdge],
    limit: usize,
    paths: &AnchorPathDisplay<'_>,
) {
    if edges.is_empty() {
        return;
    }
    println!("{title}:");
    let visible_count = edges.len().min(limit);
    let mut grouped: std::collections::BTreeMap<&str, Vec<&StructuralEdge>> =
        std::collections::BTreeMap::new();
    for edge in edges.iter().take(visible_count) {
        grouped.entry(edge.from.as_str()).or_default().push(edge);
    }
    for (from, edges) in grouped {
        println!("- `{}`", paths.path(from));
        for edge in edges {
            println!(
                "  - {} -> `{}` [{}; {}] {}",
                edge.edge_type,
                paths.path(&edge.to),
                public_evidence_label(&edge.evidence),
                format!("{:?}", edge.strength).to_ascii_lowercase(),
                edge_location_summary_with_paths(edge, paths)
            );
        }
    }
    let hidden_count = edges.len().saturating_sub(visible_count);
    if hidden_count > 0 {
        println!("- hidden: {hidden_count} {title} edges");
    }
}

pub(crate) fn cone_section(title: &str, edges: &[StructuralEdge]) {
    if edges.is_empty() {
        return;
    }
    println!("\n## {title}\n");
    grouped_edge_list(&title.to_ascii_lowercase(), edges, 10);
}

pub(crate) fn table(headers: &[&str], rows: Vec<Vec<String>>) -> String {
    let mut out = Vec::new();
    out.push(format!("| {} |", headers.join(" | ")));
    out.push(format!("|{}|", vec!["---"; headers.len()].join("|")));
    for row in rows {
        out.push(format!(
            "| {} |",
            row.into_iter()
                .map(|cell| cell.replace('\n', "<br>"))
                .collect::<Vec<_>>()
                .join(" | ")
        ));
    }
    out.join("\n")
}

pub(crate) fn bullet(values: &[String], code_style: bool, limit: Option<usize>) -> String {
    let mut items: Vec<String> = values.to_vec();
    if let Some(limit) = limit
        && items.len() > limit
    {
        let extra = items.len() - limit;
        items.truncate(limit);
        items.push(format!("... +{extra} more"));
    }
    if items.is_empty() {
        return "- none".to_string();
    }
    items
        .into_iter()
        .map(|item| {
            if code_style {
                format!("- `{item}`")
            } else {
                format!("- {item}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn code(value: &str) -> String {
    format!("`{value}`")
}

pub(crate) fn code_block(lang: &str, commands: &[String]) -> String {
    if commands.is_empty() {
        return format!("```{lang}\n# no command inferred\n```");
    }
    format!("```{lang}\n{}\n```", commands.join("\n"))
}

pub(crate) fn public_evidence_label(evidence: &str) -> String {
    if evidence == "role_script_target" {
        return "script_surface_match".to_string();
    }
    if evidence == "no_structural_proof_surface" {
        return "no_structural_verification_surface".to_string();
    }
    evidence
        .strip_prefix("role:")
        .map(|rest| format!("surface_hint:{rest}"))
        .unwrap_or_else(|| evidence.to_string())
}

pub(crate) fn public_surface_kind_label(kind: &str) -> String {
    match kind {
        "missing_direct_proof" => "missing_direct_linked_surface".to_string(),
        "no_structural_proof_surface" => "no_structural_verification_surface".to_string(),
        other => other.to_string(),
    }
}

pub(crate) fn mermaid_id(value: &str) -> String {
    let body: String = value
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("n_{body}")
}

pub(crate) fn escape_mermaid(value: &str) -> String {
    value.replace('"', "'").replace('\n', " ")
}
