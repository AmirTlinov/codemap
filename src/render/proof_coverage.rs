fn proof_coverage_section(coverage: &crate::model::ProofCoverageSummary) {
    println!("\n## Coverage\n");
    println!("- changed files: `{}`", coverage.changed_count);
    println!(
        "- runnable deterministic: `{}`",
        coverage.runnable_deterministic.len()
    );
    println!("- evidence only: `{}`", coverage.evidence_only.len());
    println!(
        "- setup/support only: `{}`",
        coverage.setup_support_only.len()
    );
    println!("- soft only: `{}`", coverage.soft_only.len());
    println!("- missing direct proof: `{}`", coverage.missing.len());
    proof_coverage_path_group("Runnable Deterministic", &coverage.runnable_deterministic);
    proof_coverage_path_group("Evidence Only", &coverage.evidence_only);
    proof_coverage_path_group("Setup / Support Only", &coverage.setup_support_only);
    proof_coverage_path_group("Soft Only", &coverage.soft_only);
    proof_coverage_gaps(&coverage.missing);
}

fn proof_coverage_path_group(title: &str, paths: &[crate::model::ProofCoveredPath]) {
    if paths.is_empty() {
        return;
    }
    let limit = if paths.len() > 5 { 3 } else { 8 };
    println!("\n### {title}");
    for path in paths.iter().take(limit) {
        println!("- `{}` [sensors={}]", path.path, path.sensor_count);
        if !path.evidence.is_empty() {
            println!("  evidence: {}", path.evidence.join(", "));
        }
    }
    let hidden = paths.len().saturating_sub(limit);
    if hidden > 0 {
        println!("- hidden paths: `{hidden}`");
    }
}

fn proof_coverage_gaps(gaps: &[crate::model::ProofGap]) {
    if gaps.is_empty() {
        return;
    }
    let limit = if gaps.len() > 5 { 3 } else { 8 };
    println!("\n### Gaps");
    for gap in gaps.iter().take(limit) {
        println!("- `{}` [{}]", gap.path, gap.kind);
        println!("  effect: {}", gap.effect);
        println!("  expand: `{}`", root_aware_expand(&gap.expand));
    }
    let hidden = gaps.len().saturating_sub(limit);
    if hidden > 0 {
        println!("- hidden gaps: `{hidden}`");
    }
}
