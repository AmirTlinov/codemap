// Responsibility: cli-path-args
use crate::cli::shell_quote_arg;
use crate::repo;
use anyhow::Result;
use std::path::Path;
use std::path::PathBuf;

pub(crate) fn parse_files(
    project: &crate::model::Project,
    files: Option<&str>,
    positional: &[String],
) -> Result<Vec<String>> {
    let mut out = Vec::new();
    if let Some(files) = files {
        for file in files.split(',') {
            out.push(project_relative_arg(project, file)?);
        }
    }
    for file in positional {
        out.push(project_relative_arg(project, file)?);
    }
    Ok(out.into_iter().filter(|s| s != ".").collect())
}

pub(crate) fn files_selector(files: &[String]) -> String {
    if files.is_empty() {
        return String::new();
    }
    let files_arg = files
        .iter()
        .map(|file| shell_quote_arg(file))
        .collect::<Vec<_>>()
        .join(",");
    format!("--files {files_arg}")
}

pub(crate) fn project_relative_arg(project: &crate::model::Project, value: &str) -> Result<String> {
    let portable_value = if cfg!(windows) || !value.contains('\\') {
        value.to_string()
    } else {
        value.replace('\\', "/")
    };
    let path = Path::new(&portable_value);
    let root = normalize_absolute_arg(&project.root);
    let absolute = if path.is_absolute() {
        normalize_absolute_arg(path)
    } else {
        normalize_absolute_arg(&root.join(path))
    };
    absolute
        .strip_prefix(root)
        .map(|rel| repo::normalize_rel_path(&rel.to_string_lossy()))
        .map_err(|_| crate::cli::invalid_input(format!("path is outside project root: {value}")))
}

pub(crate) fn flow_anchor_arg(project: &crate::model::Project, value: &str) -> Result<String> {
    let trimmed = value.trim();
    if cli_route_anchor(project, trimmed) {
        return Ok(trimmed.to_string());
    }
    project_relative_arg(project, trimmed)
}

fn cli_route_anchor(project: &crate::model::Project, value: &str) -> bool {
    if value.starts_with('/') {
        let root = normalize_absolute_arg(&project.root);
        let root = root.to_string_lossy();
        return !value.starts_with(root.as_ref());
    }
    let Some((method, path)) = value.split_once(' ') else {
        return false;
    };
    path.trim().starts_with('/')
        && matches!(
            method.trim().to_ascii_uppercase().as_str(),
            "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "ALL" | "HEAD" | "OPTIONS"
        )
}

pub(crate) fn normalize_absolute_arg(path: &Path) -> PathBuf {
    if let Ok(canonical) = path.canonicalize() {
        return canonical;
    }
    let mut tail = Vec::new();
    let mut cursor = path;
    loop {
        if cursor.exists() {
            let mut out = cursor
                .canonicalize()
                .unwrap_or_else(|_| lexical_normalize_absolute(cursor));
            for part in tail.iter().rev() {
                out.push(part);
            }
            return lexical_normalize_absolute(&out);
        }
        let Some(parent) = cursor.parent() else {
            return lexical_normalize_absolute(path);
        };
        if parent == cursor {
            return lexical_normalize_absolute(path);
        }
        let Some(name) = cursor.file_name() else {
            return lexical_normalize_absolute(path);
        };
        tail.push(PathBuf::from(name));
        cursor = parent;
    }
}

fn lexical_normalize_absolute(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            std::path::Component::RootDir => out.push(std::path::MAIN_SEPARATOR.to_string()),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::Normal(part) => out.push(part),
        }
    }
    out
}

pub(crate) fn scoped_project_path(project: &crate::model::Project, value: &str) -> Result<PathBuf> {
    project_relative_arg(project, value)
        .map(|rel| project.root.join(rel))
        .map_err(|_| anyhow::anyhow!("refusing to write outside project root: {value}"))
}
