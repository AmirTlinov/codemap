// Responsibility: cli-module-root

mod anchors;
mod args;
mod cache_admin;
mod diff_args;
mod fast_paths;
mod files;
mod identity;
mod init;
mod proof_run;
mod run;
mod runtime;
mod schema_and_roots;
mod section_args;
mod since_args;

pub(crate) use anchors::*;
pub(crate) use args::*;
pub(crate) use cache_admin::*;
pub(crate) use diff_args::*;
pub(crate) use fast_paths::*;
pub(crate) use files::*;
pub(crate) use identity::*;
pub(crate) use init::*;
pub(crate) use proof_run::*;
pub use run::run;
pub(crate) use runtime::*;
pub(crate) use schema_and_roots::*;
pub(crate) use section_args::*;
pub(crate) use since_args::*;
