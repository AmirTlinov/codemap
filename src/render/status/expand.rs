// Responsibility: expand-root-rewriting
use serde::Serialize;
use std::path::Path;
use std::sync::OnceLock;

static EXPAND_ROOT: OnceLock<String> = OnceLock::new();

pub fn set_expand_root(root: Option<&Path>) {
    if let Some(root) = root {
        let _ = EXPAND_ROOT.set(root.to_string_lossy().to_string());
    }
}

pub fn root_aware_expand(command: &str) -> String {
    let command = public_expand_command(command);
    let Some(root) = EXPAND_ROOT.get() else {
        return command;
    };
    prefix_expand_command(&command, root)
}

pub fn print_json<T: Serialize>(value: &T) -> anyhow::Result<()> {
    let mut value = serde_json::to_value(value)?;
    rewrite_expand_fields(&mut value);
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

pub(crate) fn rewrite_expand_fields(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                if key == "expand" {
                    rewrite_expand_value(child);
                } else {
                    rewrite_expand_fields(child);
                }
            }
        }
        serde_json::Value::Array(values) => {
            for child in values {
                rewrite_expand_fields(child);
            }
        }
        _ => {}
    }
}

fn rewrite_expand_value(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(command) => {
            *command = root_aware_expand(command);
        }
        serde_json::Value::Array(commands) => {
            for command in commands {
                rewrite_expand_value(command);
            }
        }
        _ => rewrite_expand_fields(value),
    }
}

fn prefix_expand_command(command: &str, root: &str) -> String {
    if !command.starts_with("codemap ") || command.starts_with("codemap --root ") {
        return command.to_string();
    }
    format!(
        "codemap --root {} {}",
        shell_quote_for_expand(root),
        command.trim_start_matches("codemap ")
    )
}

fn public_expand_command(command: &str) -> String {
    command
        .replace("codemap proof --changed", "codemap proof changed")
        .replace(" --include-hidden", " --all")
}

fn shell_quote_for_expand(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-' | ':'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}
