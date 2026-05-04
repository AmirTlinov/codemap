pub fn teach(report: &TeachReport) {
    println!("# Repo Dialect Draft\n");
    println!("No repository files were written.\n");
    println!(
        "{}",
        table(
            &["Field", "Value"],
            vec![
                vec![
                    "Config".to_string(),
                    report
                        .config
                        .as_ref()
                        .map(|value| code(value))
                        .unwrap_or_else(|| "zero-config".to_string()),
                ],
                vec![
                    "Surface hint patterns".to_string(),
                    report.role_patterns.len().to_string(),
                ],
                vec![
                    "Proof changed commands".to_string(),
                    report.proof_changed.len().to_string(),
                ],
            ],
        )
    );
    if !report.role_patterns.is_empty() {
        println!("\n## Surface Hints\n");
        println!("Derived from deterministic configured patterns. Not intent, correctness, or ownership truth.\n");
        for role in &report.role_patterns {
            println!(
                "- `{}` -> `{}` [{}; matched: `{}`]",
                role.pattern, role.role, role.evidence, role.matched
            );
            if !role.examples.is_empty() {
                println!("  examples: {}", role.examples.join(", "));
            }
        }
    }
    if !report.proof_changed.is_empty() {
        println!("\n## Proof Changed\n");
        for command in &report.proof_changed {
            let source = command
                .source
                .as_ref()
                .map(|path| match command.line_start {
                    Some(line) => format!("{path}:{line}"),
                    None => path.clone(),
                })
                .unwrap_or_else(|| "script catalog".to_string());
            println!(
                "- `{}` [{}; source: `{}`]",
                command.command, command.evidence, source
            );
        }
    }
    if !report.codemap_yml.is_empty() {
        println!("\n## codemap.yml\n");
        println!("{}", code_block("yaml", &report.codemap_yml));
    }
    section("Expand", &report.expand);
}
