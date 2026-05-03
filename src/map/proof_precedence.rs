fn unique_proof_surfaces(values: Vec<ProofSurface>) -> Vec<ProofSurface> {
    let mut seen = BTreeMap::new();
    let mut out = Vec::new();
    for value in values {
        let key = (
            value.command.clone().unwrap_or_default(),
            value.path.clone().unwrap_or_default(),
        );
        if let Some(index) = seen.get(&key).copied() {
            if proof_surface_precedence(&value) > proof_surface_precedence(&out[index]) {
                out[index] = value;
            }
        } else {
            seen.insert(key, out.len());
            out.push(value);
        }
    }
    out
}

fn unique_proof_commands(values: Vec<ProofSurface>) -> Vec<ProofSurface> {
    let mut seen = BTreeMap::new();
    let mut out = Vec::new();
    for value in values {
        let Some(command) = value.command.clone() else {
            continue;
        };
        if let Some(index) = seen.get(&command).copied() {
            if proof_surface_precedence(&value) > proof_surface_precedence(&out[index]) {
                out[index] = value;
            }
        } else {
            seen.insert(command, out.len());
            out.push(value);
        }
    }
    out
}

struct ProofSurfaceBucket {
    first_index: usize,
    precedence: (EvidenceStrength, usize),
    proofs: VecDeque<ProofSurface>,
}

fn balanced_proof_surface_prefix(values: &[ProofSurface], limit: usize) -> Vec<ProofSurface> {
    if values.len() <= limit {
        return values.to_vec();
    }

    let mut buckets: BTreeMap<String, ProofSurfaceBucket> = BTreeMap::new();
    for (index, value) in values.iter().cloned().enumerate() {
        let precedence = proof_surface_precedence(&value);
        let entry = buckets
            .entry(value.evidence.clone())
            .or_insert_with(|| ProofSurfaceBucket {
                first_index: index,
                precedence,
                proofs: VecDeque::new(),
            });
        entry.precedence = entry.precedence.max(precedence);
        entry.proofs.push_back(value);
    }

    let mut buckets = buckets.into_iter().collect::<Vec<_>>();
    buckets.sort_by(|left, right| {
        right
            .1
            .precedence
            .cmp(&left.1.precedence)
            .then_with(|| left.1.first_index.cmp(&right.1.first_index))
            .then_with(|| left.0.cmp(&right.0))
    });

    let mut out = Vec::with_capacity(limit);
    while out.len() < limit && !buckets.is_empty() {
        let mut progressed = false;
        for bucket in &mut buckets {
            if out.len() == limit {
                break;
            }
            if let Some(value) = bucket.1.proofs.pop_front() {
                out.push(value);
                progressed = true;
            }
        }
        buckets.retain(|bucket| !bucket.1.proofs.is_empty());
        if !progressed {
            break;
        }
    }
    out
}

fn proof_surface_precedence(value: &ProofSurface) -> (EvidenceStrength, usize) {
    (value.strength, proof_evidence_precedence(&value.evidence))
}

fn proof_surface_satisfies_specific_proof(proof: &ProofSurface) -> bool {
    crate::proof_classification::proof_surface_is_runnable_validation(proof)
        || proof_surface_command_closes_fallback(proof)
}

fn proof_surface_is_soft_structural_match(proof: &ProofSurface) -> bool {
    matches!(
        proof_base_evidence(&proof.evidence),
        "script_path_token" | "role_script_target"
    )
}

fn proof_surface_command_closes_fallback(proof: &ProofSurface) -> bool {
    crate::proof_classification::proof_surface_is_runnable_validation(proof)
}

fn proof_evidence_precedence(evidence: &str) -> usize {
    let mediated = evidence.ends_with("_via_direct_consumer")
        || evidence.ends_with("_via_direct_dependency")
        || evidence.ends_with("_via_local_symbol_consumer");
    let evidence = evidence
        .split_once(':')
        .map(|(prefix, _)| prefix)
        .unwrap_or(evidence);
    let evidence = evidence
        .strip_suffix("_owning_file")
        .or_else(|| evidence.strip_suffix("_via_direct_consumer"))
        .or_else(|| evidence.strip_suffix("_via_direct_dependency"))
        .or_else(|| evidence.strip_suffix("_via_local_symbol_consumer"))
        .unwrap_or(evidence);
    let base: usize = match evidence {
        "test_import" => 6,
        "test_imported_symbol_reference" => 6,
        "test_reexported_symbol_reference" => 6,
        "e2e_route" => 5,
        "test_support_import" => 5,
        "test_symbol_reference" => 4,
        "test_name" => 3,
        "e2e_surface_phrase" => 3,
        "e2e_path_surface" => 2,
        "test_surface_phrase" => 2,
        "test_surface_tokens" => 1,
        _ => 0,
    };
    if mediated {
        base.saturating_sub(2)
    } else {
        base
    }
}
