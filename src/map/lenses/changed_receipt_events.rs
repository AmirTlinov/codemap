fn changed_receipt_witness_events(
    project: &Project,
    rel: &str,
    mode: &DiffMapMode,
) -> Vec<crate::model::ChangedStructuralEvent> {
    if !changed_receipt_witness_surface(project, rel) || !receipt_json_path(rel) {
        return Vec::new();
    }
    let current = diff_current_file_text(project, rel, mode).unwrap_or_default();
    let base = diff_base_file_text(project, rel, mode).unwrap_or_default();
    let Ok(current_value) = serde_json::from_str::<serde_json::Value>(&current) else {
        return receipt_payload_fallback_event(rel);
    };
    let Ok(base_value) = serde_json::from_str::<serde_json::Value>(&base) else {
        return receipt_payload_fallback_event(rel);
    };
    let current_buckets = receipt_bucket_values(&current_value, &current);
    let base_buckets = receipt_bucket_values(&base_value, &base);
    let mut buckets = current_buckets.keys().cloned().collect::<BTreeSet<_>>();
    buckets.extend(base_buckets.keys().cloned());
    let mut events = Vec::new();
    for bucket in buckets {
        let current_bucket = current_buckets.get(&bucket);
        let base_bucket = base_buckets.get(&bucket);
        let current_value = current_bucket.map(|bucket| &bucket.value);
        let base_value = base_bucket.map(|bucket| &bucket.value);
        if current_value == base_value {
            continue;
        }
        let line = current_bucket
            .or(base_bucket)
            .map(|bucket| bucket.line)
            .unwrap_or(1);
        events.push(changed_fact_event(
            receipt_event_kind(current_bucket.is_some(), base_bucket.is_some()),
            rel,
            line,
            "git_diff_receipt_section",
            format!("receipt/witness `{bucket}` changed"),
            Some(format!("codemap proof {}", shell_quote(rel))),
            "receipt_section",
        ));
    }
    if events.is_empty() && current_value != base_value {
        return receipt_payload_fallback_event(rel);
    }
    events
}

fn changed_receipt_witness_surface(project: &Project, rel: &str) -> bool {
    project
        .files
        .get(rel)
        .map(|file| file.has_role("receipt") || file.has_role("witness"))
        .unwrap_or_else(|| receipt_witness_path_hint(rel))
}

fn receipt_witness_path_hint(rel: &str) -> bool {
    let lower = rel.to_ascii_lowercase();
    lower.contains("/receipts/")
        || lower.starts_with("receipts/")
        || lower.contains("/witnesses/")
        || lower.starts_with("witnesses/")
        || changed_map_path_file_name(&lower).contains("receipt")
        || changed_map_path_file_name(&lower).contains("witness")
}

fn receipt_json_path(rel: &str) -> bool {
    matches!(
        std::path::Path::new(rel)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("json")
    )
}

#[derive(Clone)]
struct ReceiptBucket {
    value: serde_json::Value,
    line: usize,
}

fn receipt_bucket_values(
    value: &serde_json::Value,
    text: &str,
) -> BTreeMap<String, ReceiptBucket> {
    let Some(object) = value.as_object() else {
        return BTreeMap::new();
    };
    let mut buckets: BTreeMap<String, Vec<(String, serde_json::Value, usize)>> = BTreeMap::new();
    for (key, value) in object {
        let Some(bucket) = receipt_bucket_for_key(key) else {
            continue;
        };
        buckets.entry(bucket.to_string()).or_default().push((
            key.clone(),
            value.clone(),
            json_key_line(text, key).unwrap_or(1),
        ));
    }
    buckets
        .into_iter()
        .map(|(bucket, mut values)| {
            values.sort_by(|a, b| a.0.cmp(&b.0));
            let line = values.iter().map(|(_, _, line)| *line).min().unwrap_or(1);
            let value = serde_json::Value::Array(
                values
                    .into_iter()
                    .map(|(key, value, _)| serde_json::json!({ "key": key, "value": value }))
                    .collect(),
            );
            (bucket, ReceiptBucket { value, line })
        })
        .collect()
}

fn receipt_bucket_for_key(key: &str) -> Option<&'static str> {
    let normalized = key.to_ascii_lowercase().replace(['-', ' '], "_");
    match normalized.as_str() {
        "claim_status" | "status" | "outcome" | "result" | "verdict" => Some("claim_status"),
        "claim_boundary" | "boundary" | "claim" | "claims" | "scope" | "objective"
        | "hypothesis" => Some("claim_boundary"),
        "metrics" | "metric" | "measurements" | "scores" | "results" | "summary" => {
            Some("metrics")
        }
        "controls" | "control" | "falsifiers" | "guardrails" | "checks" | "invariants" => {
            Some("controls")
        }
        "proof_command" | "proof_commands" | "command" | "commands" | "proof" | "validation"
        | "validation_command" => Some("proof_command"),
        "schema" | "schema_version" | "version" | "kind" | "receipt_version" => Some("schema"),
        _ => None,
    }
}

fn receipt_event_kind(current: bool, base: bool) -> &'static str {
    match (current, base) {
        (true, false) => "added_receipt_section",
        (false, true) => "removed_receipt_section",
        _ => "changed_receipt_section",
    }
}

fn receipt_payload_fallback_event(rel: &str) -> Vec<crate::model::ChangedStructuralEvent> {
    vec![changed_fact_event(
        "changed_receipt_payload",
        rel,
        1,
        "git_diff_receipt_payload",
        "receipt/witness payload changed outside known buckets".to_string(),
        Some(format!("codemap diff-map --files {}", shell_quote(rel))),
        "receipt_payload",
    )]
}
