fn proof_plan_surface_sections(report: &ProofReport, force: bool) {
    let runnable = report
        .proofs
        .iter()
        .filter(|proof| crate::proof_classification::proof_surface_is_runnable_validation(proof))
        .collect::<Vec<_>>();
    let evidence_only = report
        .proofs
        .iter()
        .filter(|proof| crate::proof_classification::proof_surface_is_evidence_only(proof))
        .collect::<Vec<_>>();
    let setup_or_support = report
        .proofs
        .iter()
        .filter(|proof| crate::proof_classification::proof_surface_is_setup_or_support(proof))
        .collect::<Vec<_>>();
    let soft = report
        .proofs
        .iter()
        .filter(|proof| crate::proof_classification::proof_surface_is_soft_evidence(proof))
        .collect::<Vec<_>>();
    if runnable.is_empty()
        && force
        && (!evidence_only.is_empty() || !setup_or_support.is_empty() || !soft.is_empty())
    {
        proof_empty_section(
            "Proof",
            "No runnable deterministic proof commands were emitted; evidence, setup/support, and soft surfaces are shown separately.",
        );
    } else if !runnable.is_empty() {
        proof_plan_surface_section("Proof", report, &runnable);
    }
    if !evidence_only.is_empty() {
        proof_plan_evidence_surface_section("Evidence Surfaces", report, &evidence_only);
        println!(
            "\nEvidence surfaces are source-backed links without a runnable command. They do not replace runnable proof commands or remove Unknown entries."
        );
    }
    if !setup_or_support.is_empty() {
        proof_plan_surface_section("Setup / Support Surfaces", report, &setup_or_support);
        println!(
            "\nSetup/support surfaces are connected rails such as install, codegen, migration, seed, deploy, release, watch, or dev-server steps. They are not treated as validation proof and are not run by `--run`."
        );
    }
    if !soft.is_empty() {
        proof_plan_surface_section("Soft Evidence", report, &soft);
        println!(
            "\nSoft evidence is token/name/path surface overlap. It does not replace deterministic proof or remove Unknown entries."
        );
    }
}

fn proof_plan_evidence_surface_section(
    title: &str,
    report: &ProofReport,
    proofs: &[&ProofSurface],
) {
    println!("\n## {title}");
    let mut grouped: std::collections::BTreeMap<String, Vec<&ProofSurface>> =
        std::collections::BTreeMap::new();
    for proof in proofs {
        grouped
            .entry(public_evidence_label(&proof.evidence))
            .or_default()
            .push(*proof);
    }
    for (evidence, proofs) in grouped {
        println!("\n### `{evidence}`");
        println!("- sensors: `{}`", proofs.len());
        proof_count_line("strength", strength_counts(&proofs));
        proof_plan_surface_samples(report, &proofs);
    }
}

fn proof_plan_surface_section(title: &str, report: &ProofReport, proofs: &[&ProofSurface]) {
    println!("\n## {title}");
    let mut grouped: std::collections::BTreeMap<String, Vec<&ProofSurface>> =
        std::collections::BTreeMap::new();
    for proof in proofs {
        grouped
            .entry(proof_display_command(proof))
            .or_default()
            .push(*proof);
    }
    for (command, proofs) in grouped {
        println!("\n### `{command}`");
        println!("- sensors: `{}`", proofs.len());
        proof_count_line("evidence", evidence_counts(&proofs));
        proof_count_line("strength", strength_counts(&proofs));
        proof_plan_surface_samples(report, &proofs);
    }
}

fn proof_plan_surface_samples(report: &ProofReport, proofs: &[&ProofSurface]) {
    let sample_limit = if proofs.len() <= 6 { proofs.len() } else { 5 };
    if sample_limit > 0 {
        println!("- sample:");
        for proof in proofs.iter().take(sample_limit) {
            let path = proof
                .path
                .as_ref()
                .map(|path| code(path))
                .unwrap_or_else(|| "`none`".to_string());
            println!(
                "  - {path}{} [{}; {}] {} - {}",
                proof_target_suffix(proof),
                public_evidence_label(&proof.evidence),
                format!("{:?}", proof.strength).to_ascii_lowercase(),
                proof_location_summary(&proof.locations),
                proof.reason
            );
        }
    }
    let hidden_details = proofs.len().saturating_sub(sample_limit);
    if hidden_details > 0 {
        println!("- hidden details: `{hidden_details}` sensors");
        if let Some(expand) = proof_detail_expand(report, proofs.len()) {
            println!("  expand: `{}`", root_aware_expand(&expand));
        }
    }
}

fn proof_display_command(proof: &ProofSurface) -> String {
    let Some(command) = &proof.command else {
        return public_evidence_label(&proof.evidence);
    };
    let Some(path) = proof.path.as_deref() else {
        return command.clone();
    };
    let mut candidates = Vec::new();
    candidates.push(path.to_string());
    let parts = path.split('/').collect::<Vec<_>>();
    for index in 1..parts.len() {
        candidates.push(parts[index..].join("/"));
    }
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.len()));
    for candidate in candidates {
        for suffix in [candidate.clone(), shell_quote_for_markdown(&candidate)] {
            let suffix = format!(" {suffix}");
            if let Some(stripped) = command.strip_suffix(&suffix) {
                return stripped.trim_end_matches(" --").trim_end().to_string();
            }
        }
    }
    command.clone()
}

fn evidence_counts(proofs: &[&ProofSurface]) -> Vec<(String, usize)> {
    let mut counts = std::collections::BTreeMap::new();
    for proof in proofs {
        *counts
            .entry(public_evidence_label(&proof.evidence))
            .or_insert(0) += 1;
    }
    counts.into_iter().collect()
}

fn strength_counts(proofs: &[&ProofSurface]) -> Vec<(String, usize)> {
    let mut counts = std::collections::BTreeMap::new();
    for proof in proofs {
        *counts
            .entry(format!("{:?}", proof.strength).to_ascii_lowercase())
            .or_insert(0) += 1;
    }
    counts.into_iter().collect()
}

fn proof_count_line(label: &str, counts: Vec<(String, usize)>) {
    if counts.is_empty() {
        return;
    }
    let values = counts
        .into_iter()
        .map(|(kind, count)| format!("`{kind}: {count}`"))
        .collect::<Vec<_>>()
        .join(", ");
    println!("- {label}: {values}");
}

fn proof_detail_expand(report: &ProofReport, limit: usize) -> Option<String> {
    if let Some(target) = &report.target {
        return Some(format!(
            "codemap proof-map {} --raw-sensors --limit {limit}",
            shell_quote_for_markdown(target)
        ));
    }
    if !report.changed.is_empty() {
        let selector = proof_map_changed_selector(report);
        return Some(format!(
            "codemap proof-map {selector} --raw-sensors --limit {limit}"
        ));
    }
    None
}

fn proof_changed_command_selector_suffix(report: &ProofReport) -> String {
    match report.selector.as_str() {
        "" | "changed" | "--changed" => String::new(),
        selector => format!(" {selector}"),
    }
}

fn proof_map_changed_selector(report: &ProofReport) -> String {
    match report.selector.as_str() {
        "" | "changed" | "--changed" => "--changed".to_string(),
        selector => selector.to_string(),
    }
}

fn shell_quote_for_markdown(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-' | '#'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn proof_location_summary(locations: &[EvidenceLocation]) -> String {
    let Some(first) = locations.first() else {
        return "unknown".to_string();
    };
    let suffix = if locations.len() > 1 {
        format!(" +{}", locations.len() - 1)
    } else {
        String::new()
    };
    let base = if first.path == "aggregate" {
        "aggregate".to_string()
    } else if let Some(line) = first.line_start {
        format!("{}:{line}", first.path)
    } else {
        first.path.clone()
    };
    format!("{}{}", code(&base), suffix)
}
