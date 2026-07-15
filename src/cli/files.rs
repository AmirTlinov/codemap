// Responsibility: cli-files
use crate::cli::{OutputFormat, project_relative_arg};
use crate::render;
use anyhow::Result;

pub(crate) fn output<T: serde::Serialize>(
    format: OutputFormat,
    value: &T,
    markdown: impl FnOnce(),
) -> Result<()> {
    match format {
        OutputFormat::Json => render::print_json(value, &crate::cli::build_identity(false)),
        OutputFormat::Markdown => {
            markdown();
            Ok(())
        }
    }
}

pub(crate) fn output_with_prelude<T: serde::Serialize>(
    format: OutputFormat,
    value: &T,
    prelude: &crate::model::MapPrelude,
    markdown: impl FnOnce(),
) -> Result<()> {
    let build_identity = crate::cli::build_identity(false);
    match format {
        OutputFormat::Json => render::print_json_with_prelude(value, prelude, &build_identity),
        OutputFormat::Markdown => {
            render::set_map_prelude(prelude.clone(), build_identity);
            markdown();
            Ok(())
        }
    }
}

#[derive(serde::Serialize)]
pub(crate) struct FilesReport {
    kind: &'static str,
    schema_version: &'static str,
    path: String,
    files: Vec<String>,
    count: usize,
}

pub(crate) fn files_report(
    project: &crate::model::Project,
    path: Option<&str>,
    limit: usize,
) -> Result<FilesReport> {
    let normalized_path = path
        .map(|path| project_relative_arg(project, path))
        .transpose()?;
    if let Some(rel) = normalized_path.as_deref()
        && project.files.contains_key(rel)
    {
        let mut files = vec![rel.to_string()];
        let count = files.len();
        files.truncate(limit);
        return Ok(FilesReport {
            kind: "files",
            schema_version: "3",
            path: rel.to_string(),
            files,
            count,
        });
    }
    let prefix = normalized_path
        .as_deref()
        .filter(|p| *p != ".")
        .map(|p| format!("{}/", p.trim_end_matches('/')));
    let mut files: Vec<String> = project
        .files
        .values()
        .filter(|file| {
            file.indexed_boundary != Some(crate::model::IndexedBoundary::IgnoredTrackedFile)
        })
        .map(|file| &file.rel)
        .filter(|rel| prefix.as_ref().map(|p| rel.starts_with(p)).unwrap_or(true))
        .cloned()
        .collect();
    files.sort();
    let count = files.len();
    files.truncate(limit);
    Ok(FilesReport {
        kind: "files",
        schema_version: "3",
        path: normalized_path.unwrap_or_else(|| ".".to_string()),
        files,
        count,
    })
}

pub(crate) fn files_markdown(report: &FilesReport) {
    println!("# Files\n");
    println!("Path: `{}`", report.path);
    println!("Shown: `{}` of `{}`\n", report.files.len(), report.count);
    if report.files.is_empty() {
        println!("- none");
    } else {
        for file in &report.files {
            println!("- `{file}`");
        }
    }
}
