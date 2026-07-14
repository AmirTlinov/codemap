fn boundary_facts_section(facts: &crate::model::BoundaryFacts, force: bool, compact: bool) {
    if boundary_facts_empty(facts) {
        if force {
            println!("\n## Boundary Facts\n");
            println!("No instruction, guard, or protected-looking path facts found.");
        }
        return;
    }
    println!("\n## Boundary Facts\n");
    if !compact {
        disclaimer("Path facts only. Not permissions or next actions.");
    }
    boundary_fact_group(
        "instruction files",
        &facts.instruction_files,
        "none",
        compact,
    );
    boundary_fact_group(
        "protected-looking paths",
        &facts.protected_looking_paths,
        "none",
        compact,
    );
    boundary_fact_group(
        "repo-local guard files",
        &facts.repo_local_guard_files,
        "none",
        compact,
    );
}

fn boundary_facts_empty(facts: &crate::model::BoundaryFacts) -> bool {
    facts.instruction_files.is_empty()
        && facts.protected_looking_paths.is_empty()
        && facts.repo_local_guard_files.is_empty()
}

fn boundary_fact_group(
    title: &str,
    facts: &[crate::model::BoundaryFact],
    empty: &str,
    compact: bool,
) {
    if facts.is_empty() {
        if !compact {
            println!("{title}: {empty}");
        }
        return;
    }
    if compact {
        let sample = facts
            .iter()
            .take(3)
            .map(|fact| code(&fact.path))
            .collect::<Vec<_>>()
            .join(", ");
        let hidden = facts.len().saturating_sub(3);
        if hidden == 0 {
            println!("- {title}: `{}` ({sample})", facts.len());
        } else {
            println!("- {title}: `{}` ({sample} +{hidden} hidden)", facts.len());
        }
        return;
    }
    println!("{title}:");
    for fact in facts.iter().take(8) {
        println!("- `{}` [{}]", fact.path, public_evidence_label(&fact.evidence));
        println!("  effect: {}", fact.effect);
    }
    let hidden = facts.len().saturating_sub(8);
    if hidden > 0 {
        println!("- additional {title}: `{hidden}`");
    }
}
