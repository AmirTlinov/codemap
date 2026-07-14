fn proof_wiring_section(facts: &[ProofWiringFact], compact: bool, expand: Option<&str>) {
    if facts.is_empty() {
        return;
    }
    println!("\n## Verification Wiring");
    proof_wiring_status_counts(facts);
    let limit = if compact { 6 } else { 14 };
    for fact in facts.iter().take(limit) {
        let path = fact
            .path
            .as_ref()
            .map(|path| format!(" path={}", code(path)))
            .unwrap_or_default();
        println!(
            "- [{}] `{}` `{}`{} — {}",
            fact.status, fact.stage, fact.subject, path, fact.effect
        );
        if let Some(location) = fact.locations.first() {
            println!(
                "  evidence: {} at {}",
                proof_wiring_inline_code(&fact.evidence),
                proof_wiring_location(location)
            );
        } else {
            println!("  evidence: {}", proof_wiring_inline_code(&fact.evidence));
        }
        if let Some(expand) = &fact.expand {
            println!("  expand: `{}`", root_aware_expand(expand));
        }
    }
    let hidden = facts.len().saturating_sub(limit);
    if hidden > 0 {
        println!("- hidden wiring facts: `{hidden}`");
        if let Some(expand) = expand {
            println!("  expand: `{}`", root_aware_expand(expand));
        }
    }
}

fn proof_wiring_summary_section(facts: &[ProofWiringFact], expand: Option<&str>) {
    if facts.is_empty() || !proof_wiring_has_material_summary(facts) {
        return;
    }
    println!("\n## Verification Wiring");
    let status = proof_wiring_status_summary(facts);
    match (status, expand) {
        (Some(status), Some(expand)) => {
            println!("- status: {status}; expand: `{}`", root_aware_expand(expand));
        }
        (Some(status), None) => println!("- status: {status}"),
        (None, Some(expand)) => println!("- expand: `{}`", root_aware_expand(expand)),
        (None, None) => {}
    }
    if let Some(fact) = facts
        .iter()
        .find(|fact| matches!(fact.status.as_str(), "missing" | "unknown"))
    {
        let path = fact
            .path
            .as_ref()
            .map(|path| format!(" path={}", code(path)))
            .unwrap_or_default();
        println!(
            "- sample: [{}] `{}` `{}`{} — {}",
            fact.status, fact.stage, fact.subject, path, fact.effect
        );
        if let Some(expand) = &fact.expand {
            println!("  expand: `{}`", root_aware_expand(expand));
        }
    }
}

fn proof_wiring_has_material_summary(facts: &[ProofWiringFact]) -> bool {
    facts.iter().any(|fact| {
        matches!(
            fact.status.as_str(),
            "missing" | "soft" | "validated" | "load_bearing" | "executed"
        ) || matches!(
            fact.stage.as_str(),
            "artifact" | "evidence_consumption" | "contract_field"
        )
    })
}

fn proof_wiring_status_counts(facts: &[ProofWiringFact]) {
    let Some(values) = proof_wiring_status_summary(facts) else {
        return;
    };
    println!("- status: {values}");
}

fn proof_wiring_status_summary(facts: &[ProofWiringFact]) -> Option<String> {
    let mut counts = std::collections::BTreeMap::new();
    for fact in facts {
        *counts.entry(fact.status.as_str()).or_insert(0usize) += 1;
    }
    if counts.is_empty() {
        return None;
    }
    Some(
        counts
            .into_iter()
            .map(|(status, count)| format!("`{status}: {count}`"))
            .collect::<Vec<_>>()
            .join(", "),
    )
}

fn proof_wiring_inline_code(value: &str) -> String {
    code(&value.replace('`', "'"))
}

fn proof_wiring_location(location: &EvidenceLocation) -> String {
    if location.path == "aggregate" {
        return "aggregate".to_string();
    }
    if let Some(line) = location.line_start {
        code(&format!("{}:{line}", location.path))
    } else {
        code(&location.path)
    }
}
