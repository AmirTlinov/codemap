fn changed_proof_section(report: &ChangedReport, compact: bool) {
    println!("\n## Proof");
    let runnable_grouped = changed_proof_command_groups(report);
    let setup_grouped = changed_proof_surface_groups(report.proof.setup_support.iter());
    let soft_grouped = changed_proof_surface_groups(report.proof.soft_evidence.iter());
    let runnable = changed_proof_groups_by_class(&runnable_grouped, ChangedProofGroupClass::Runnable);
    let evidence_only = changed_proof_evidence_only_surfaces(report);
    let setup = changed_proof_groups_by_class(&setup_grouped, ChangedProofGroupClass::Setup);
    let soft = changed_proof_groups_by_class(&soft_grouped, ChangedProofGroupClass::Soft);
    if compact && report.total_changed_count > 20 {
        changed_proof_large_compact_summary(report, &runnable, &evidence_only, &setup, &soft);
        return;
    }
    if runnable.is_empty() && report.proof.fallback.is_empty() {
        println!("No runnable proof command inferred.");
    }
    changed_proof_render_groups(&runnable, compact, &report.selector);
    if !evidence_only.is_empty() {
        println!("\n## Evidence Surfaces");
        changed_proof_render_evidence_surfaces(&evidence_only, compact, &report.selector);
        if !compact {
            println!(
                "\nEvidence surfaces are source-backed links without a runnable command. They do not replace runnable proof commands or remove Unknown entries."
            );
        }
    }
    if !setup.is_empty() {
        println!("\n## Setup / Support Surfaces");
        changed_proof_render_groups(&setup, compact, &report.selector);
        if !compact {
            println!(
                "\nSetup/support surfaces are connected rails such as install, codegen, migration, seed, deploy, release, watch, or dev-server steps. They are not validation proof and are not run by `--run`."
            );
        }
    }
    if !soft.is_empty() {
        println!("\n## Soft Evidence");
        changed_proof_render_groups(&soft, compact, &report.selector);
        if !compact {
            println!(
                "\nSoft evidence is token/name/path surface overlap. It does not replace deterministic proof or remove Unknown entries."
            );
        }
    }
    if !report.proof.fallback.is_empty() {
        println!("\n### Fallback");
        println!("{}", code_block("bash", &report.proof.fallback));
    }
    changed_proof_sensor_counts(report, compact);
}

fn changed_proof_large_compact_summary(
    report: &ChangedReport,
    runnable: &[(&String, &(Vec<&ProofSurface>, usize))],
    evidence_only: &[&ProofSurface],
    setup: &[(&String, &(Vec<&ProofSurface>, usize))],
    soft: &[(&String, &(Vec<&ProofSurface>, usize))],
) {
    println!("- runnable command groups: `{}`", runnable.len());
    println!("- evidence-only sensors: `{}`", evidence_only.len());
    println!("- setup/support groups: `{}`", setup.len());
    println!("- soft evidence groups: `{}`", soft.len());
    println!("- fallback commands: `{}`", report.proof.fallback.len());
    println!(
        "- expand: `{}`",
        root_aware_expand(&format!(
            "codemap changed{} --section proof",
            changed_selector_suffix(&report.selector)
        ))
    );
    changed_proof_sensor_counts(report, true);
}

fn changed_proof_evidence_only_surfaces(report: &ChangedReport) -> Vec<&ProofSurface> {
    let mut seen = std::collections::BTreeSet::new();
    let mut surfaces = Vec::new();
    for sensor in report
        .proof
        .hard
        .iter()
        .chain(report.proof.direct_evidence.iter())
        .chain(report.proof.mediated_evidence.iter())
        .chain(report.proof.soft_evidence.iter())
        .chain(report.proof.setup_support.iter())
    {
        if !crate::proof_classification::proof_surface_is_evidence_only(sensor) {
            continue;
        }
        let location_key = sensor
            .locations
            .iter()
            .map(|location| {
                format!(
                    "{}:{}:{}:{}",
                    location.path,
                    location.line_start.unwrap_or_default(),
                    location.line_end.unwrap_or_default(),
                    location.kind
                )
            })
            .collect::<Vec<_>>()
            .join("|");
        let key = (
            sensor.path.clone(),
            sensor.evidence.clone(),
            sensor.reason.clone(),
            location_key,
        );
        if seen.insert(key) {
            surfaces.push(sensor);
        }
    }
    surfaces.sort_by(|a, b| {
        a.evidence
            .cmp(&b.evidence)
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.reason.cmp(&b.reason))
    });
    surfaces
}

fn changed_proof_render_evidence_surfaces(
    surfaces: &[&ProofSurface],
    compact: bool,
    selector: &str,
) {
    let visible_group_limit = if compact {
        COMPACT_CHANGED_PROOF_COMMAND_LIMIT
    } else {
        usize::MAX
    };
    let mut grouped: std::collections::BTreeMap<String, Vec<&ProofSurface>> =
        std::collections::BTreeMap::new();
    for surface in surfaces {
        grouped
            .entry(public_evidence_label(&surface.evidence))
            .or_default()
            .push(*surface);
    }
    let total_group_count = grouped.len();
    let visible_group_count = total_group_count.min(visible_group_limit);
    for (evidence, sensors) in grouped.iter().take(visible_group_count) {
        println!("\n### `{evidence}`");
        changed_proof_command_group_details(sensors, 0, compact, selector);
    }
    if compact && total_group_count > visible_group_count {
        let hidden_groups = total_group_count - visible_group_count;
        println!("\n- hidden evidence groups: `{hidden_groups}`");
        println!(
            "  expand: `{}`",
            root_aware_expand(&format!(
                "codemap changed{} --section proof",
                changed_selector_suffix(selector)
            ))
        );
    }
}

fn changed_proof_sensor_counts(report: &ChangedReport, compact: bool) {
    println!("\n### Sensor Counts");
    let counts = changed_proof_public_sensor_counts(report);
    if compact {
        println!(
            "- runnable_direct: `{}`; soft: `{}`; evidence_only: `{}`; setup_support: `{}`; missing_direct_unknown: `{}`",
            counts.runnable_direct,
            counts.soft,
            counts.evidence_only,
            counts.setup_support,
            counts.missing_direct_unknown
        );
    } else {
        for (kind, count) in [
            ("runnable_direct", counts.runnable_direct),
            ("soft", counts.soft),
            ("evidence_only", counts.evidence_only),
            ("setup_support", counts.setup_support),
            ("missing_direct_unknown", counts.missing_direct_unknown),
        ] {
            println!("- {kind}: `{count}`");
        }
    }
}

struct ChangedProofPublicSensorCounts {
    runnable_direct: usize,
    soft: usize,
    evidence_only: usize,
    setup_support: usize,
    missing_direct_unknown: usize,
}

fn changed_proof_public_sensor_counts(report: &ChangedReport) -> ChangedProofPublicSensorCounts {
    let sensors = report
        .proof
        .hard
        .iter()
        .chain(report.proof.direct_evidence.iter())
        .chain(report.proof.mediated_evidence.iter())
        .chain(report.proof.soft_evidence.iter())
        .chain(report.proof.setup_support.iter())
        .collect::<Vec<_>>();
    let runnable_direct = sensors
        .iter()
        .filter(|sensor| crate::proof_classification::proof_surface_is_runnable_validation(sensor))
        .count();
    let soft = sensors
        .iter()
        .filter(|sensor| crate::proof_classification::proof_surface_is_soft_evidence(sensor))
        .count();
    let evidence_only = sensors
        .iter()
        .filter(|sensor| crate::proof_classification::proof_surface_is_evidence_only(sensor))
        .count();
    let setup_support = sensors
        .iter()
        .filter(|sensor| crate::proof_classification::proof_surface_is_setup_or_support(sensor))
        .count();
    let unknown_direct = report
        .unknowns
        .iter()
        .filter(|unknown| unknown.kind == "direct_test_import_not_found")
        .count();
    ChangedProofPublicSensorCounts {
        runnable_direct,
        soft,
        evidence_only,
        setup_support,
        missing_direct_unknown: unknown_direct.max(report.proof.missing_direct.len()),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChangedProofGroupClass {
    Runnable,
    Setup,
    Soft,
}

fn changed_proof_groups_by_class<'a>(
    grouped: &'a std::collections::BTreeMap<String, (Vec<&'a ProofSurface>, usize)>,
    class: ChangedProofGroupClass,
) -> Vec<(&'a String, &'a (Vec<&'a ProofSurface>, usize))> {
    grouped
        .iter()
        .filter(|(_, (sensors, _))| changed_proof_group_class(sensors) == class)
        .collect()
}

fn changed_proof_group_class(sensors: &[&ProofSurface]) -> ChangedProofGroupClass {
    if sensors.is_empty()
        || sensors
            .iter()
            .any(|sensor| crate::proof_classification::proof_surface_is_runnable_validation(sensor))
    {
        return ChangedProofGroupClass::Runnable;
    }
    if sensors
        .iter()
        .any(|sensor| crate::proof_classification::proof_surface_is_setup_or_support(sensor))
    {
        return ChangedProofGroupClass::Setup;
    }
    ChangedProofGroupClass::Soft
}

fn changed_proof_render_groups(
    grouped: &[(&String, &(Vec<&ProofSurface>, usize))],
    compact: bool,
    selector: &str,
) {
    let visible_group_limit = if compact {
        COMPACT_CHANGED_PROOF_COMMAND_LIMIT
    } else {
        usize::MAX
    };
    let total_group_count = grouped.len();
    let visible_group_count = total_group_count.min(visible_group_limit);
    for (command, (sensors, hidden_count)) in grouped.iter().take(visible_group_count) {
        println!("\n### `{command}`");
        changed_proof_command_group_details(sensors, *hidden_count, compact, selector);
    }
    if compact && total_group_count > visible_group_count {
        let hidden_command_groups = total_group_count - visible_group_count;
        println!("\n- hidden proof command groups: `{hidden_command_groups}`");
        println!(
            "  expand: `{}`",
            root_aware_expand(&format!(
                "codemap changed{} --section proof",
                changed_selector_suffix(selector)
            ))
        );
    }
}

fn changed_proof_command_groups(
    report: &ChangedReport,
) -> std::collections::BTreeMap<String, (Vec<&ProofSurface>, usize)> {
    let mut grouped: std::collections::BTreeMap<String, (Vec<&ProofSurface>, usize)> =
        std::collections::BTreeMap::new();
    for command in &report.proof.commands {
        if command.sensors.is_empty() {
            grouped
                .entry(command.command.clone())
                .or_insert_with(|| (Vec::new(), 0))
                .1 += command.hidden_count;
            continue;
        }
        let mut command_groups = std::collections::BTreeSet::new();
        for sensor in &command.sensors {
            let key = proof_display_command(sensor);
            command_groups.insert(key.clone());
            grouped
                .entry(key)
                .or_insert_with(|| (Vec::new(), 0))
                .0
                .push(sensor);
        }
        if command_groups.len() == 1
            && let Some(key) = command_groups.first()
        {
            grouped
                .entry(key.clone())
                .or_insert_with(|| (Vec::new(), 0))
                .1 += command.hidden_count;
        }
    }
    grouped
}

fn changed_proof_surface_groups<'a>(
    surfaces: impl Iterator<Item = &'a ProofSurface>,
) -> std::collections::BTreeMap<String, (Vec<&'a ProofSurface>, usize)> {
    let mut grouped: std::collections::BTreeMap<String, (Vec<&ProofSurface>, usize)> =
        std::collections::BTreeMap::new();
    for sensor in surfaces {
        grouped
            .entry(proof_display_command(sensor))
            .or_insert_with(|| (Vec::new(), 0))
            .0
            .push(sensor);
    }
    grouped
}

fn changed_proof_command_group_details(
    sensors: &[&ProofSurface],
    hidden_count: usize,
    compact: bool,
    selector: &str,
) {
    if sensors.is_empty() {
        println!("- no sensor details");
    } else if compact && sensors.iter().all(|sensor| {
        crate::proof_classification::proof_surface_is_soft_evidence(sensor)
    }) {
        changed_proof_soft_group_summary(sensors);
    } else {
        println!("- sensors: `{}`", sensors.len());
        changed_proof_class_line(sensors);
        proof_count_line("evidence", evidence_counts(sensors));
        proof_count_line("strength", strength_counts(sensors));
        changed_proof_samples(sensors, compact);
    }
    if hidden_count > 0 {
        println!("- hidden: {hidden_count} sensors");
        println!(
            "  expand: `{}`",
            root_aware_expand(&format!(
                "codemap proof-map {selector} --raw-sensors --limit {}",
                sensors.len() + hidden_count
            ))
        );
    }
}

fn changed_proof_soft_group_summary(sensors: &[&ProofSurface]) {
    println!(
        "- sensors: `{}`; evidence: {}; strength: {}",
        sensors.len(),
        changed_proof_inline_counts(evidence_counts(sensors)),
        changed_proof_inline_counts(strength_counts(sensors))
    );
    if let Some(sensor) = sensors.first() {
        let path = sensor.path.as_deref().unwrap_or("none");
        println!(
            "- sample: `{path}` [{}; {}] {}",
            public_evidence_label(&sensor.evidence),
            format!("{:?}", sensor.strength).to_ascii_lowercase(),
            proof_location_summary(&sensor.locations)
        );
    }
}

fn changed_proof_inline_counts(counts: Vec<(String, usize)>) -> String {
    counts
        .into_iter()
        .map(|(kind, count)| format!("`{kind}: {count}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn changed_proof_class_line(sensors: &[&ProofSurface]) {
    let runnable = sensors
        .iter()
        .filter(|sensor| crate::proof_classification::proof_surface_is_runnable_validation(sensor))
        .count();
    let evidence = sensors
        .iter()
        .filter(|sensor| crate::proof_classification::proof_surface_is_evidence_only(sensor))
        .count();
    let setup = sensors
        .iter()
        .filter(|sensor| crate::proof_classification::proof_surface_is_setup_or_support(sensor))
        .count();
    let soft = sensors
        .iter()
        .filter(|sensor| crate::proof_classification::proof_surface_is_soft_evidence(sensor))
        .count();
    if evidence > 0 || setup > 0 || soft > 0 {
        println!(
            "- proof class: `runnable: {runnable}`, `evidence: {evidence}`, `setup_support: {setup}`, `soft_evidence: {soft}`"
        );
    }
}

fn changed_proof_samples(sensors: &[&ProofSurface], compact: bool) {
    let sample_limit = if compact {
        sensors.len().min(3)
    } else if sensors.len() <= 6 {
        sensors.len()
    } else {
        5
    };
    println!("- sample:");
    for sensor in sensors.iter().take(sample_limit) {
        let path = sensor.path.as_deref().unwrap_or("none");
        println!(
            "  - `{}` [{}; {}] {}",
            path,
            public_evidence_label(&sensor.evidence),
            format!("{:?}", sensor.strength).to_ascii_lowercase(),
            proof_location_summary(&sensor.locations)
        );
    }
    let hidden_details = sensors.len().saturating_sub(sample_limit);
    if hidden_details > 0 {
        println!("- hidden details: `{hidden_details}` sensors");
    }
}
