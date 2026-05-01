pub fn impact(report: &ImpactReport) {
    println!("# Structural Impact\n");
    if report.changed.is_empty() && report.clusters.is_empty() {
        println!("No changed anchors detected. Use `--files a,b` or run with a git diff selector.");
        return;
    }
    if !report.changed.is_empty() {
        println!("\n## Changed Anchors\n");
        let rows = report
            .changed
            .iter()
            .map(|file| {
                vec![
                    code(&file.path),
                    file.kind.clone(),
                    file.package.clone().unwrap_or_else(|| "none".to_string()),
                    file.language.clone(),
                ]
            })
            .collect();
        println!("{}", table(&["Path", "Kind", "Package", "Language"], rows));
    }
    for cluster in &report.clusters {
        render_impact_cluster(cluster);
    }
    if !report.hidden.is_empty() {
        println!("\n## Hidden\n");
        let rows = report
            .hidden
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
    section("Unknown", &report.unknowns);
    section("Expand", &report.expand);
}

fn render_impact_cluster(cluster: &ImpactCluster) {
    println!("\n## Cluster `{}`\n", cluster.id);
    println!(
        "{}",
        table(
            &["Field", "Value"],
            vec![
                vec!["Risk".to_string(), cluster.risk.clone()],
                vec!["Changed".to_string(), cluster.changed.join(", ")],
                vec!["Reasons".to_string(), cluster.reasons.join("; ")],
            ],
        )
    );
    cone_section("Direct Consumers", &cluster.direct_consumers);
    cone_section(
        "Cross-Boundary Consumers",
        &cluster.cross_boundary_consumers,
    );
    cone_section("Contract Risks", &cluster.contract_risks);
    cone_section("Proof", &cluster.proof);
}

pub fn proof(report: &ProofReport) {
    println!("# Proof Plan\n");
    if let Some(target) = &report.target {
        println!("Target: `{target}`\n");
    }
    if !report.changed.is_empty() {
        println!("Changed anchors:");
        println!("{}", bullet(&report.changed, true, Some(20)));
        println!();
    }
    println!(
        "{}",
        table(
            &["Field", "Value"],
            vec![vec!["Risk".to_string(), report.risk.clone()]]
        )
    );
    if report.proofs.is_empty() && report.fallback.is_empty() {
        println!("\nNo proof surface found. Use `codemap cone <path>` to inspect edges first.");
        println!("\n{}", report.run_hint);
        return;
    }
    if !report.proofs.is_empty() {
        println!("\n## Proofs\n");
        let rows = report.proofs.iter().map(proof_row).collect::<Vec<_>>();
        println!(
            "{}",
            table(&["Command", "Path", "Evidence", "Strength", "Reason"], rows,)
        );
    }
    if !report.fallback.is_empty() {
        println!("\n## Fallback\n");
        println!("{}", code_block("bash", &report.fallback));
    }
    println!("\n{}", report.run_hint);
}

fn proof_row(proof: &ProofSurface) -> Vec<String> {
    vec![
        proof
            .command
            .as_ref()
            .map(|command| code(command))
            .unwrap_or_else(|| "none".to_string()),
        proof
            .path
            .as_ref()
            .map(|path| code(path))
            .unwrap_or_else(|| "none".to_string()),
        proof.evidence.clone(),
        format!("{:?}", proof.strength).to_ascii_lowercase(),
        proof.reason.clone(),
    ]
}
