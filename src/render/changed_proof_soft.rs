fn changed_proof_render_soft_summary(
    grouped: &[(&String, &(Vec<&ProofSurface>, usize))],
    selector: &str,
) {
    let total_sensors = grouped
        .iter()
        .map(|(_, (sensors, hidden_count))| sensors.len() + *hidden_count)
        .sum::<usize>();
    println!("- groups: `{}`", grouped.len());
    println!("- sensors: `{total_sensors}`");
    if let Some((_, (sensors, _))) = grouped.first()
        && let Some(sensor) = sensors.first()
    {
        println!(
            "- sample: `{}` [{}; {}] {}",
            sensor.path.as_deref().unwrap_or("none"),
            public_evidence_label(&sensor.evidence),
            format!("{:?}", sensor.strength).to_ascii_lowercase(),
            proof_location_summary(&sensor.locations)
        );
    }
    println!(
        "- expand: `{}`",
        root_aware_expand(&format!(
            "codemap proof-map {selector} --raw-sensors --limit {total_sensors}"
        ))
    );
}
