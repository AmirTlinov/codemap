fn group_duplicate_proof_surfaces(
    values: &mut Vec<ProofSurface>,
    hidden: &mut Vec<HiddenGroup>,
    reason: &str,
    expand: &str,
) {
    let mut seen = BTreeMap::new();
    let mut out = Vec::new();
    let mut duplicate_count = 0usize;
    for value in values.drain(..) {
        let key = proof_surface_group_key(&value);
        if let Some(index) = seen.get(&key).copied() {
            duplicate_count += 1;
            if proof_surface_precedence(&value) > proof_surface_precedence(&out[index]) {
                out[index] = value;
            }
        } else {
            seen.insert(key, out.len());
            out.push(value);
        }
    }
    if duplicate_count > 0 {
        hidden.push(HiddenGroup {
            reason: reason.to_string(),
            count: duplicate_count,
            expand: expand_with_concrete_limit(expand, out.len() + duplicate_count),
        });
    }
    *values = out;
}

fn proof_surface_group_key(value: &ProofSurface) -> (String, String, String, String) {
    let detail = if value.evidence == "e2e_visited_route" {
        value.reason.clone()
    } else {
        String::new()
    };
    (
        value.command.clone().unwrap_or_default(),
        value.path.clone().unwrap_or_default(),
        value.target_anchor.clone().unwrap_or_default(),
        detail,
    )
}

fn group_duplicate_missing_surfaces(
    values: &mut Vec<Surface>,
    hidden: &mut Vec<HiddenGroup>,
    reason: &str,
    expand: &str,
) {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    let mut duplicate_count = 0usize;
    for value in values.drain(..) {
        let key = (
            value.kind.clone(),
            value.path.clone().unwrap_or_default(),
            value.evidence.clone(),
        );
        if seen.insert(key) {
            out.push(value);
        } else {
            duplicate_count += 1;
        }
    }
    if duplicate_count > 0 {
        hidden.push(HiddenGroup {
            reason: reason.to_string(),
            count: duplicate_count,
            expand: expand_with_concrete_limit(expand, out.len() + duplicate_count),
        });
    }
    *values = out;
}

fn group_duplicate_unknowns(
    values: &mut Vec<Unknown>,
    hidden: &mut Vec<HiddenGroup>,
    reason: &str,
    expand: &str,
) {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    let mut duplicate_count = 0usize;
    for value in values.drain(..) {
        let key = (
            value.kind.clone(),
            value.path.clone().unwrap_or_default(),
            value.line_start.unwrap_or_default(),
            value.reason.clone(),
            value.effect.clone(),
        );
        if seen.insert(key) {
            out.push(value);
        } else {
            duplicate_count += 1;
        }
    }
    if duplicate_count > 0 {
        hidden.push(HiddenGroup {
            reason: reason.to_string(),
            count: duplicate_count,
            expand: expand_with_concrete_limit(expand, out.len() + duplicate_count),
        });
    }
    *values = out;
}

fn proof_map_expand(selector: &str, raw_sensors: bool) -> String {
    let raw = if raw_sensors { " --raw-sensors" } else { "" };
    format!("codemap proof-map {selector}{raw} --limit <larger-number>")
}

fn proof_map_proof_expand(selector: &str) -> String {
    format!("codemap proof {selector}")
}
