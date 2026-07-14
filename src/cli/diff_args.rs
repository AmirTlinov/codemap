// Responsibility: cli-diff-args
mod lens_inputs;
mod path_args;
mod proof_selectors;
mod section_names;

pub(crate) use lens_inputs::*;
pub(crate) use path_args::*;
pub(crate) use proof_selectors::*;
pub(crate) use section_names::*;

use crate::cli::semantic_anchor_problems;
use anyhow::Result;
use anyhow::bail;

pub(crate) fn ensure_valid_config(project: &crate::model::Project) -> Result<()> {
    let semantic_problems = semantic_anchor_problems(project);
    if project.config_errors.is_empty() && semantic_problems.is_empty() {
        return Ok(());
    }
    for error in &project.config_errors {
        eprintln!(
            "codemap: invalid semantic anchor `{}`: {}",
            error.path, error.error
        );
    }
    for problem in semantic_problems {
        eprintln!("codemap: invalid semantic anchor: {problem}");
    }
    bail!("invalid .codemap semantic anchors; run `codemap anchors validate`")
}
