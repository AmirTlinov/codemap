// Responsibility: map-proof-owner-ci-validation
use crate::map::{command_tokens, strip_inline_shell_comment};

pub(crate) fn ci_owner_validation_step_reason(command: &str) -> Option<String> {
    let command = strip_inline_shell_comment(command);
    let command = command.trim();
    if command.is_empty()
        || ci_owner_command_has_unsupported_shell_control(command)
        || ci_owner_command_has_unsupported_shell_composition(command)
        || ci_owner_command_is_non_validation(command)
    {
        return None;
    }
    let tokens = command_tokens(command);
    if ci_owner_cargo_validation(&tokens) {
        return Some("CI workflow run step invokes Cargo validation".to_string());
    }
    if ci_owner_package_script_validation(&tokens) {
        return Some("CI workflow run step invokes package validation script".to_string());
    }
    if ci_owner_direct_tool_validation(command) {
        return Some("CI workflow run step invokes test or validation tool".to_string());
    }
    if ci_owner_make_or_just_validation(&tokens) {
        return Some("CI workflow run step invokes make/just validation target".to_string());
    }
    if command_has_codemap_validation(&tokens) {
        return Some("CI workflow run step invokes codemap validation".to_string());
    }
    None
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum CiOwnerStepKind {
    Validation,
    Release,
    Setup,
    Control,
}

impl CiOwnerStepKind {
    pub(crate) fn edge_type(self) -> &'static str {
        match self {
            Self::Validation => "ci_validation_step",
            Self::Release => "ci_release_step",
            Self::Setup => "ci_setup_step",
            Self::Control => "ci_control_step",
        }
    }

    pub(crate) fn evidence(self) -> &'static str {
        match self {
            Self::Validation => "ci_run_validation",
            Self::Release => "ci_run_release",
            Self::Setup => "ci_run_setup",
            Self::Control => "ci_run_control",
        }
    }
}

pub(crate) fn ci_owner_step_kind(command: &str) -> Option<CiOwnerStepKind> {
    let command = strip_inline_shell_comment(command);
    let command = command.trim();
    if command.is_empty() {
        return None;
    }
    if ci_owner_command_is_shell_syntax_only(command) {
        return None;
    }
    if ci_owner_validation_step_reason(command).is_some() {
        return Some(CiOwnerStepKind::Validation);
    }
    if ci_owner_command_is_control(command) {
        return Some(CiOwnerStepKind::Control);
    }
    if ci_owner_command_is_release_or_mutation(command) {
        return Some(CiOwnerStepKind::Release);
    }
    if ci_owner_command_is_setup(command) {
        return Some(CiOwnerStepKind::Setup);
    }
    Some(CiOwnerStepKind::Control)
}

mod command_classes;
mod shell_composition;
mod validation_matchers;

pub(crate) use command_classes::*;
pub(crate) use shell_composition::*;
pub(crate) use validation_matchers::*;
