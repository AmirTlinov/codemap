// Responsibility: stable-cli-exit-taxonomy
use anyhow::Error;
use std::any::Any;
use std::fmt::{Display, Formatter};
use std::process::ExitCode;
use std::sync::Arc;

pub(crate) const EXIT_SUCCESS: u8 = 0;
pub(crate) const EXIT_VALID_EMPTY: u8 = 10;
pub(crate) const EXIT_INVALID_INPUT: u8 = 20;
pub(crate) const EXIT_UNSUPPORTED_REQUEST: u8 = 21;
pub(crate) const EXIT_DIAGNOSTIC_FAILURE: u8 = 22;
pub(crate) const EXIT_UNSAFE_REFUSED: u8 = 23;
pub(crate) const EXIT_INTERNAL_ERROR: u8 = 70;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExitClass {
    InvalidInput,
    UnsupportedRequest,
    DiagnosticFailure,
    UnsafeRefused,
    InternalError,
}

impl ExitClass {
    fn code(self) -> u8 {
        match self {
            Self::InvalidInput => EXIT_INVALID_INPUT,
            Self::UnsupportedRequest => EXIT_UNSUPPORTED_REQUEST,
            Self::DiagnosticFailure => EXIT_DIAGNOSTIC_FAILURE,
            Self::UnsafeRefused => EXIT_UNSAFE_REFUSED,
            Self::InternalError => EXIT_INTERNAL_ERROR,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::InvalidInput => "invalid_input",
            Self::UnsupportedRequest => "unsupported_request",
            Self::DiagnosticFailure => "diagnostic_failure",
            Self::UnsafeRefused => "unsafe_execution_refused",
            Self::InternalError => "internal_error",
        }
    }
}

#[derive(Debug)]
pub(crate) struct CliFailure {
    class: ExitClass,
    message: String,
}

impl Display for CliFailure {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CliFailure {}

pub(crate) fn invalid_input(message: impl Into<String>) -> Error {
    failure(ExitClass::InvalidInput, message)
}

pub(crate) fn unsupported_request(message: impl Into<String>) -> Error {
    failure(ExitClass::UnsupportedRequest, message)
}

pub(crate) fn diagnostic_failure(message: impl Into<String>) -> Error {
    failure(ExitClass::DiagnosticFailure, message)
}

pub(crate) fn unsafe_refused(message: impl Into<String>) -> Error {
    failure(ExitClass::UnsafeRefused, message)
}

fn failure(class: ExitClass, message: impl Into<String>) -> Error {
    Error::new(CliFailure {
        class,
        message: message.into(),
    })
}

pub fn main_exit() -> ExitCode {
    let default_hook = Arc::new(std::panic::take_hook());
    let visible_hook = Arc::clone(&default_hook);
    std::panic::set_hook(Box::new(move |info| {
        if !is_broken_pipe(info.payload()) {
            visible_hook(info);
        }
    }));
    let run = std::panic::catch_unwind(crate::cli::run);
    let result = match run {
        Ok(result) => result,
        Err(payload) if is_broken_pipe(payload.as_ref()) => return ExitCode::SUCCESS,
        Err(payload) => std::panic::resume_unwind(payload),
    };
    match result {
        Ok(()) => ExitCode::from(crate::render::take_report_exit()),
        Err(error) => {
            let class = error
                .downcast_ref::<CliFailure>()
                .map(|failure| failure.class)
                .unwrap_or(ExitClass::InternalError);
            eprintln!("codemap: {}: {error:#}", class.label());
            ExitCode::from(class.code())
        }
    }
}

fn is_broken_pipe(payload: &(dyn Any + Send)) -> bool {
    payload
        .downcast_ref::<String>()
        .is_some_and(|message| message.contains("Broken pipe"))
        || payload
            .downcast_ref::<&str>()
            .is_some_and(|message| message.contains("Broken pipe"))
}

pub(crate) fn clap_failure(error: clap::Error) -> Error {
    use clap::error::ErrorKind;
    let class = match error.kind() {
        ErrorKind::InvalidSubcommand | ErrorKind::UnknownArgument => ExitClass::UnsupportedRequest,
        _ => ExitClass::InvalidInput,
    };
    failure(class, error.to_string().trim().to_string())
}
