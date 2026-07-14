// Responsibility: map-command-inference-roles-context
use crate::map::anchor_file_rel;
use crate::model::Project;
use std::collections::BTreeSet;

pub(crate) struct ProofRoleContext {
    pub(crate) roles: BTreeSet<String>,
    pub(crate) tokens: BTreeSet<String>,
    pub(crate) has_role_surface: bool,
}

pub(crate) fn proof_role_context(
    project: &Project,
    anchors: &[String],
) -> Option<ProofRoleContext> {
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

pub(crate) fn proof_context_token(token: &str) -> bool {
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
