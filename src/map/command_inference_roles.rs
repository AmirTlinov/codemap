fn role_aware_minimal_commands(
    project: &Project,
    files: &[String],
    changed: &[String],
) -> Vec<String> {
    let anchors = if changed.is_empty() { files } else { changed };
    let Some(context) = proof_role_context(project, anchors) else {
        return Vec::new();
    };
    let mut candidates = project
        .scripts
        .iter()
        .filter_map(|script| role_aware_script_rank(script, &context).map(|rank| (rank, script)))
        .collect::<Vec<_>>();
    candidates.sort_by(|(left_rank, left), (right_rank, right)| {
        left_rank
            .cmp(right_rank)
            .then_with(|| left.command.cmp(&right.command))
    });
    unique(
        candidates
            .into_iter()
            .map(|(_, script)| script.command.clone())
            .collect(),
    )
    .into_iter()
    .take(3)
    .collect()
}

struct ProofRoleContext {
    roles: BTreeSet<String>,
    tokens: BTreeSet<String>,
    has_role_surface: bool,
}

fn proof_role_context(project: &Project, anchors: &[String]) -> Option<ProofRoleContext> {
    let mut roles = BTreeSet::new();
    let mut tokens = BTreeSet::new();
    for anchor in anchors {
        let rel = anchor_file_rel(anchor);
        let Some(file) = project.files.get(&rel) else {
            continue;
        };
        roles.extend(file.roles.iter().cloned());
        tokens.extend(
            file.tokens
                .iter()
                .filter(|token| proof_context_token(token))
                .cloned(),
        );
    }
    let has_role_surface = roles.iter().any(|role| proof_planner_role(role));
    if !has_role_surface && tokens.is_empty() {
        return None;
    }
    Some(ProofRoleContext {
        roles,
        tokens,
        has_role_surface,
    })
}

fn proof_context_token(token: &str) -> bool {
    token.len() >= 3
        && !matches!(
            token,
            "src"
                | "lib"
                | "app"
                | "apps"
                | "test"
                | "tests"
                | "tools"
                | "scripts"
                | "experiments"
                | "docs"
                | "json"
                | "jsonl"
                | "md"
                | "py"
                | "rs"
                | "ts"
                | "tsx"
                | "js"
                | "jsx"
        )
}

fn proof_planner_role(role: &str) -> bool {
    matches!(
        role,
        "receipt"
            | "witness"
            | "proof_runner"
            | "owner_doc"
            | "migration"
            | "schema"
            | "schema_contract"
            | "deploy"
            | "entrypoint"
            | "runtime_surface"
            | "doctor"
    )
}

fn role_aware_script_rank(
    script: &crate::model::ScriptInfo,
    context: &ProofRoleContext,
) -> Option<usize> {
    let text = format!(
        "{} {} {}",
        script.name.to_ascii_lowercase(),
        script.command.to_ascii_lowercase(),
        script.reason.to_ascii_lowercase()
    );
    if script_is_mutating_without_validation(&text) {
        return None;
    }
    let token_hits = context
        .tokens
        .iter()
        .filter(|token| script_text_has_token(&text, token))
        .count();
    if token_hits >= 3 {
        return Some(0);
    }
    if token_hits >= 2
        && script_text_has_any(
            &text,
            &[
                "audit",
                "check",
                "doctor",
                "falsifier",
                "hardening",
                "proof",
                "qwen",
                "test",
                "validate",
                "verify",
            ],
        )
    {
        return Some(1);
    }
    if !script_is_validation_surface_text(&text) {
        return None;
    }
    if !context.has_role_surface {
        return None;
    }
    role_specific_script_rank(&context.roles, &text)
}

fn role_specific_script_rank(roles: &BTreeSet<String>, text: &str) -> Option<usize> {
    let mut ranks = Vec::new();
    if roles.contains("receipt") || roles.contains("witness") {
        ranks.extend(role_keyword_ranks(
            text,
            &[
                "validate-receipts",
                "receipt",
                "witness",
                "validate",
                "proof",
                "doctor",
                "next",
                "test",
            ],
            10,
        ));
    }
    if roles.contains("proof_runner") {
        ranks.extend(role_keyword_ranks(
            text,
            &["qwen", "proof", "validate", "doctor", "next", "test"],
            20,
        ));
    }
    if roles.contains("owner_doc") {
        ranks.extend(role_keyword_ranks(
            text,
            &["doctor", "next", "validate", "proof", "test"],
            30,
        ));
    }
    if roles.contains("migration")
        || roles.contains("schema")
        || roles.contains("schema_contract")
    {
        ranks.extend(role_keyword_ranks(
            text,
            &[
                "validate",
                "schema",
                "migration",
                "migrate",
                "db",
                "doctor",
                "test",
            ],
            40,
        ));
    }
    if roles.contains("deploy") {
        ranks.extend(role_keyword_ranks(
            text,
            &["check", "lint", "validate", "doctor", "test"],
            50,
        ));
    }
    if roles.contains("entrypoint") || roles.contains("runtime_surface") {
        ranks.extend(role_keyword_ranks(
            text,
            &["test", "check", "doctor", "validate", "build"],
            60,
        ));
    }
    if roles.contains("doctor") {
        ranks.extend(role_keyword_ranks(text, &["doctor", "check", "test"], 70));
    }
    ranks.into_iter().min()
}

fn role_keyword_ranks(text: &str, keywords: &[&str], offset: usize) -> Vec<usize> {
    keywords
        .iter()
        .enumerate()
        .filter_map(|(index, keyword)| {
            script_text_has_keyword(text, keyword).then_some(offset + index)
        })
        .collect()
}

fn script_is_validation_surface_text(text: &str) -> bool {
    if script_is_mutating_without_validation(text) {
        return false;
    }
    script_text_has_any(
        text,
        &[
            "test", "check", "lint", "type", "doctor", "verify", "validate", "proof", "receipt",
            "witness", "qwen", "next", "schema", "migration", "migrate", "db", "build",
        ],
    )
}

fn script_is_mutating_without_validation(text: &str) -> bool {
    script_text_has_any(
        text,
        &[
            "deploy",
            "release",
            "publish",
            "apply",
            "destroy",
            "delete",
            "drop",
            "migrate deploy",
        ],
    ) && !script_text_has_any(text, &["validate", "verify", "check", "lint", "test", "doctor"])
}

fn script_text_has_any(text: &str, needles: &[&str]) -> bool {
    needles
        .iter()
        .any(|needle| script_text_has_keyword(text, needle))
}

fn script_text_has_keyword(text: &str, keyword: &str) -> bool {
    if keyword
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return script_text_has_token(text, keyword);
    }
    text.contains(keyword)
}

fn script_text_has_token(text: &str, token: &str) -> bool {
    text.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .any(|part| part == token)
}
