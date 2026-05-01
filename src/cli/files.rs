fn output<T: serde::Serialize>(
    format: OutputFormat,
    value: &T,
    markdown: impl FnOnce(),
) -> Result<()> {
    match format {
        OutputFormat::Json => render::print_json(value),
        OutputFormat::Markdown => {
            markdown();
            Ok(())
        }
    }
}

#[derive(serde::Serialize)]
struct FilesReport {
    kind: &'static str,
    schema_version: &'static str,
    path: String,
    files: Vec<String>,
    count: usize,
}

fn files_report(
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
            schema_version: "2",
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
        .keys()
        .filter(|rel| prefix.as_ref().map(|p| rel.starts_with(p)).unwrap_or(true))
        .cloned()
        .collect();
    files.sort();
    let count = files.len();
    files.truncate(limit);
    Ok(FilesReport {
        kind: "files",
        schema_version: "2",
        path: normalized_path.unwrap_or_else(|| ".".to_string()),
        files,
        count,
    })
}

fn files_markdown(report: &FilesReport) {
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

