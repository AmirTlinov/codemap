fn section(title: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    println!("\n## {title}\n");
    println!("{}", bullet(values, true, Some(20)));
}

fn unknown_section(values: &[Unknown]) {
    if values.is_empty() {
        return;
    }
    println!("\n## Unknown\n");
    let rows = values
        .iter()
        .map(|unknown| {
            vec![
                unknown.kind.clone(),
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
                    .unwrap_or_else(|| "none".to_string()),
                unknown.reason.clone(),
                unknown.effect.clone(),
                unknown
                    .expand
                    .as_ref()
                    .map(|expand| code(expand))
                    .unwrap_or_else(|| "none".to_string()),
            ]
        })
        .collect();
    println!(
        "{}",
        table(&["Kind", "Where", "Reason", "Effect", "Expand"], rows)
    );
}

fn proof_surface_section(title: &str, proofs: &[ProofSurface]) {
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
                "- {path} [{}; {}] {} - {}",
                proof.evidence,
                format!("{:?}", proof.strength).to_ascii_lowercase(),
                proof_location_summary(&proof.locations),
                proof.reason
            );
        }
    }
}

fn proof_command_summary_section(title: &str, proofs: &[ProofSurface]) {
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
    for command in commands {
        println!("- `{command}`");
    }
}

fn hidden_section(hidden: &[crate::model::HiddenGroup]) {
    if hidden.is_empty() {
        return;
    }
    println!("\n## Hidden\n");
    let rows = hidden
        .iter()
        .map(|hidden| {
            vec![
                hidden.reason.clone(),
                hidden.count.to_string(),
                code(&hidden.expand),
            ]
        })
        .collect();
    println!("{}", table(&["Reason", "Count", "Expand"], rows));
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

fn bullet(values: &[String], code_style: bool, limit: Option<usize>) -> String {
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

fn code(value: &str) -> String {
    format!("`{value}`")
}

fn code_block(lang: &str, commands: &[String]) -> String {
    if commands.is_empty() {
        return format!("```{lang}\n# no command inferred\n```");
    }
    format!("```{lang}\n{}\n```", commands.join("\n"))
}

fn mermaid_id(value: &str) -> String {
    let body: String = value
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("n_{body}")
}

fn escape_mermaid(value: &str) -> String {
    value.replace('"', "'").replace('\n', " ")
}
