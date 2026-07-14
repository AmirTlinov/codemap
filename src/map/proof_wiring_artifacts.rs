fn artifact_chain_facts_for_command(
    project: &Project,
    command: &str,
    proof: &ProofSurface,
    selector: &str,
) -> Vec<ProofWiringFact> {
    let mut facts = Vec::new();
    let artifacts = artifact_paths_for_command_or_file(project, command, proof.path.as_deref());
    if artifacts.is_empty() {
        facts.push(proof_wiring_fact(
            ("artifact", "unknown"),
            command.to_string(),
            proof.path.clone(),
            "artifact write path was not statically found",
            "verification command may run, but codemap cannot connect it to a produced receipt/report artifact",
            proof.locations.clone(),
            proof_wiring_expand_for_proof(selector, proof),
        ));
        return facts;
    }
    for artifact in artifacts {
        let exists = project.files.contains_key(&artifact);
        facts.push(proof_wiring_fact(
            ("artifact", if exists { "wired" } else { "missing" }),
            command.to_string(),
            Some(artifact.clone()),
            if exists {
                "declared artifact path exists in the indexed repo".to_string()
            } else {
                "declared artifact path is not indexed".to_string()
            },
            if exists {
                "runner-to-artifact edge is structural; codemap did not execute the writer"
                    .to_string()
            } else {
                "runner declares or mentions an artifact that is absent".to_string()
            },
            vec![EvidenceLocation::path(&artifact, "artifact_path")],
            Some(format!("codemap cone {} --depth 2", shell_quote(&artifact))),
        ));
        facts.extend(artifact_consumption_facts(project, &artifact, selector));
    }
    facts
}

fn artifact_contract_wiring_facts_limited(
    project: &Project,
    anchor: &str,
    selector: &str,
    limit: usize,
) -> Vec<ProofWiringFact> {
    let Some(file) = project.files.get(anchor) else {
        return Vec::new();
    };
    if limit == 0 {
        return Vec::new();
    }
    if !(file.has_role("receipt") || file.has_role("witness") || file.has_role("owner_doc")) {
        return Vec::new();
    }
    let mut facts = artifact_consumption_facts(project, anchor, selector);
    facts.truncate(limit);
    if facts.len() >= limit {
        return facts;
    }
    facts.extend(receipt_declared_command_wiring_facts(project, anchor, selector));
    facts.truncate(limit);
    if facts.len() >= limit {
        return facts;
    }
    let remaining = limit.saturating_sub(facts.len());
    facts.extend(receipt_field_wiring_facts_limited(
        project, anchor, selector, remaining,
    ));
    facts.truncate(limit);
    if facts.len() >= limit {
        return facts;
    }
    let remaining = limit.saturating_sub(facts.len());
    facts.extend(markdown_declared_field_wiring_facts_limited(
        project, anchor, selector, remaining,
    ));
    facts.truncate(limit);
    facts
}

fn artifact_consumption_facts(project: &Project, artifact: &str, selector: &str) -> Vec<ProofWiringFact> {
    let consumers = artifact_consumers(project, artifact);
    if consumers.is_empty() {
        return vec![proof_wiring_fact(
            ("evidence_consumption", "unknown"),
            artifact.to_string(),
            Some(artifact.to_string()),
            "artifact consumer was not found",
            "evidence object is produced/present, but codemap cannot find a receipt/report/review/predicate consumer",
            vec![EvidenceLocation::path(artifact, "artifact")],
            Some(format!("codemap proof {} --section unknown", selector)),
        )];
    }
    let load_bearing = consumers
        .iter()
        .any(|(consumer, _)| file_has_predicate_language(project, consumer));
    vec![proof_wiring_fact(
        ("evidence_consumption", if load_bearing { "load_bearing" } else { "wired" }),
        artifact.to_string(),
        Some(artifact.to_string()),
        if load_bearing {
            "artifact is referenced by a consumer with predicate/control language"
        } else {
            "artifact is referenced by another indexed file"
        },
        if load_bearing {
            "evidence participates in a statically visible control/predicate surface"
        } else {
            "consumer exists, but pass predicate participation was not proven"
        },
        consumers
            .into_iter()
            .map(|(path, line)| EvidenceLocation::line(path, line, "artifact_consumer"))
            .collect(),
        Some(format!("codemap cone {} --depth 2", shell_quote(artifact))),
    )]
}

fn receipt_declared_command_wiring_facts(
    project: &Project,
    rel: &str,
    selector: &str,
) -> Vec<ProofWiringFact> {
    let Ok(text) = std::fs::read_to_string(project.root.join(rel)) else {
        return Vec::new();
    };
    let Some(command) = json_string_field(&text, "proof_command")
        .or_else(|| json_string_field(&text, "command"))
        .or_else(|| json_string_field(&text, "validation_command"))
    else {
        return Vec::new();
    };
    let proof = ProofSurface {
        command: Some(command),
        path: Some(rel.to_string()),
        target_anchor: Some(rel.to_string()),
        evidence: "artifact_declared_command".to_string(),
        strength: EvidenceStrength::High,
        reason: "artifact/receipt declares a verification command".to_string(),
        locations: field_line_locations(&text, rel, &["proof_command", "command", "validation_command"]),
    };
    proof_surface_wiring_facts(project, &proof, selector)
}

fn receipt_field_wiring_facts_limited(
    project: &Project,
    rel: &str,
    selector: &str,
    limit: usize,
) -> Vec<ProofWiringFact> {
    let Ok(text) = std::fs::read_to_string(project.root.join(rel)) else {
        return Vec::new();
    };
    if limit == 0 {
        return Vec::new();
    }
    let fields = declared_receipt_fields(rel, &text);
    let consumer_texts = proof_wiring_consumer_texts(project, rel);
    let mut facts = Vec::new();
    for (field, line) in fields.into_iter().take(12) {
        if facts.len() >= limit {
            break;
        }
        let consumers = field_consumers_from_texts(&consumer_texts, &field);
        if consumers.is_empty() {
            let status = if receipt_field_is_execution(&field) {
                "executed"
            } else {
                "unknown"
            };
            facts.push(proof_wiring_fact(
                ("contract_field", status),
                field.clone(),
                Some(rel.to_string()),
                format!("receipt/control field `{field}` is present but no consumer was found"),
                if receipt_field_is_execution(&field) {
                    "exit-code field is present; no downstream consumer was found"
                } else {
                    "field presence alone is not proof unless a predicate/report/review consumes it"
                },
                vec![EvidenceLocation::line(rel, line, "receipt_field")],
                Some(format!("codemap proof {} --section unknown", selector)),
            ));
            continue;
        }
        let load_bearing = consumers.iter().any(|(consumer, _)| {
            consumer_texts
                .iter()
                .find(|(rel, _)| rel == consumer)
                .is_some_and(|(_, text)| text_has_predicate_language(text))
        });
        let status = if receipt_field_is_execution(&field) {
            "executed"
        } else if receipt_field_is_schema(&field) && load_bearing {
            "validated"
        } else if load_bearing {
            "load_bearing"
        } else {
            "wired"
        };
        facts.push(proof_wiring_fact(
            ("contract_field", status),
            field.clone(),
            Some(rel.to_string()),
            format!("receipt/control field `{field}` is consumed"),
            if receipt_field_is_execution(&field) {
                "field carries mechanical execution evidence and is consumed"
            } else if receipt_field_is_schema(&field) && load_bearing {
                "schema/version field is consumed by a visible validator/predicate surface"
            } else if load_bearing {
                "field is referenced by a visible predicate/control consumer"
            } else {
                "field is referenced, but load-bearing predicate participation was not proven"
            },
            std::iter::once(EvidenceLocation::line(rel, line, "receipt_field"))
                .chain(
                    consumers
                        .into_iter()
                        .map(|(path, line)| EvidenceLocation::line(path, line, "field_consumer")),
                )
                .collect(),
            Some(format!("codemap cone {} --depth 2", shell_quote(rel))),
        ));
    }
    facts
}

fn markdown_declared_field_wiring_facts_limited(
    project: &Project,
    rel: &str,
    selector: &str,
    limit: usize,
) -> Vec<ProofWiringFact> {
    let Ok(artifact_text) = std::fs::read_to_string(project.root.join(rel)) else {
        return Vec::new();
    };
    if limit == 0 {
        return Vec::new();
    }
    let present_fields = declared_receipt_fields(rel, &artifact_text)
        .into_iter()
        .map(|(field, _)| field)
        .collect::<BTreeSet<_>>();
    let basename = rel.rsplit('/').next().unwrap_or(rel);
    let stem = basename.split('.').next().unwrap_or(basename);
    let mut facts = Vec::new();
    let mut seen = BTreeSet::new();
    for file in project.files.values() {
        if facts.len() >= limit {
            break;
        }
        if file.rel == rel || file.language != "markdown" {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(project.root.join(&file.rel)) else {
            continue;
        };
        if !text.contains(rel) && !text.contains(basename) && !text.contains(stem) {
            continue;
        }
        for (index, line) in text.lines().enumerate() {
            if facts.len() >= limit {
                break;
            }
            for field in markdown_declared_fields(line) {
                if facts.len() >= limit {
                    break;
                }
                if !seen.insert((file.rel.clone(), field.clone())) {
                    continue;
                }
                let present = present_fields.contains(&field);
                facts.push(proof_wiring_fact(
                    ("contract_field", if present { "wired" } else { "missing" }),
                    field.clone(),
                    Some(rel.to_string()),
                    format!("owner/doc declares field `{field}` near `{rel}`"),
                    if present {
                        "declared field is present in the artifact; consumer/predicate wiring is reported separately"
                    } else {
                        "declared field is absent from the artifact"
                    },
                    vec![EvidenceLocation::line(&file.rel, index + 1, "field_declaration")],
                    Some(format!("codemap proof {} --section links", selector)),
                ));
            }
        }
    }
    facts
}
