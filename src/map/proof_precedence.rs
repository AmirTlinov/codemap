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

fn proof_surface_precedence(value: &ProofSurface) -> (EvidenceStrength, usize) {
    (value.strength, proof_evidence_precedence(&value.evidence))
}

fn proof_surface_is_deterministic(proof: &ProofSurface) -> bool {
    proof.strength >= EvidenceStrength::High
}

fn proof_surface_satisfies_specific_proof(proof: &ProofSurface) -> bool {
    proof_surface_is_deterministic(proof) || proof_surface_command_closes_fallback(proof)
}

fn proof_surface_is_soft_structural_match(proof: &ProofSurface) -> bool {
    matches!(
        proof_base_evidence(&proof.evidence),
        "script_path_token" | "role_script_target"
    )
}

fn proof_surface_command_closes_fallback(proof: &ProofSurface) -> bool {
    if proof.command.is_none() {
        return false;
    }
    if proof_surface_is_deterministic(proof) {
        return true;
    }
    matches!(
        proof_base_evidence(&proof.evidence),
        "test_import"
            | "test_imported_symbol_reference"
            | "test_reexported_symbol_reference"
            | "test_support_import"
            | "test_symbol_reference"
            | "e2e_route"
    )
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
