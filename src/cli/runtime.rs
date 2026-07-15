// Responsibility: runtime-command-execution
use crate::cli::{OutputFormat, RuntimeArgs, ensure_valid_config, output, project_relative_arg};
use crate::{map, render, repo};
use anyhow::{Result, bail};

pub(crate) fn run_runtime(project: &crate::model::Project, args: RuntimeArgs) -> Result<()> {
    ensure_valid_config(project)?;
    let scope = project_relative_arg(project, &args.scope)?;
    let include_hidden = args.include_hidden || args.format == OutputFormat::Json;
    let limit = if include_hidden {
        usize::MAX / 2
    } else {
        args.limit.max(1)
    };
    let report = map::runtime_report(project, &scope, include_hidden, limit);
    let validation = if include_hidden {
        report.validate_full_projection()
    } else {
        report.validate_bounded_projection(limit)
    };
    if let Err(error) = validation {
        bail!("invalid runtime observation ledger: {error:?}");
    }
    if scope == "." && !include_hidden && limit == 20 {
        crate::cache::runtime_root::write_runtime_root(project, repo::VERSION)?;
    }
    output(args.format, &report, || render::runtime(&report))
}
