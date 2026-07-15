// Responsibility: repo-static-language-import-bindings
use crate::model::ImportBindingsBySpec;
use std::path::Path;

pub(crate) fn extract_python_import_bindings(text: &str) -> ImportBindingsBySpec {
    let mut out = ImportBindingsBySpec::new();
    let mut statement = String::new();
    for line in text.lines() {
        let code = line.split('#').next().unwrap_or_default().trim();
        if statement.is_empty() {
            if !(code.starts_with("from ") || code.starts_with("import ")) {
                continue;
            }
            statement.push_str(code);
        } else {
            statement.push(' ');
            statement.push_str(code);
        }
        if python_import_continues(&statement) {
            continue;
        }
        parse_python_import_statement(&statement, &mut out);
        statement.clear();
    }
    if !statement.is_empty() {
        parse_python_import_statement(&statement, &mut out);
    }
    out
}

fn python_import_continues(statement: &str) -> bool {
    statement.ends_with('\\')
        || statement.chars().filter(|ch| *ch == '(').count()
            > statement.chars().filter(|ch| *ch == ')').count()
}

fn parse_python_import_statement(statement: &str, out: &mut ImportBindingsBySpec) {
    if let Some(rest) = statement.strip_prefix("from ") {
        let Some((spec, names)) = rest.split_once(" import ") else {
            return;
        };
        let spec = spec.trim();
        for item in names.trim().trim_matches(['(', ')']).split(',') {
            let (imported, local) = import_alias(item);
            if imported.is_empty() {
                continue;
            }
            out.entry(spec.to_string())
                .or_default()
                .insert(local.to_string(), imported.to_string());
        }
        return;
    }
    let Some(rest) = statement.strip_prefix("import ") else {
        return;
    };
    for item in rest.split(',') {
        let (spec, alias) = import_alias(item);
        if spec.is_empty() {
            continue;
        }
        let local = if alias == spec {
            spec.to_string()
        } else {
            alias.to_string()
        };
        out.entry(spec.to_string())
            .or_default()
            .insert(local, "*".to_string());
    }
}

fn import_alias(item: &str) -> (&str, &str) {
    let item = item.trim().trim_end_matches('\\').trim();
    item.split_once(" as ")
        .map(|(source, local)| (source.trim(), local.trim()))
        .unwrap_or((item, item))
}

pub(crate) fn extract_go_import_bindings(text: &str) -> ImportBindingsBySpec {
    let mut out = ImportBindingsBySpec::new();
    let mut in_block = false;
    for line in text.lines() {
        let code = line.split("//").next().unwrap_or_default().trim();
        if in_block {
            if code.starts_with(')') {
                in_block = false;
                continue;
            }
            record_go_import(code, &mut out);
            continue;
        }
        let Some(rest) = code.strip_prefix("import") else {
            continue;
        };
        let rest = rest.trim_start();
        if rest.starts_with('(') {
            in_block = true;
            record_go_import(rest.trim_start_matches('(').trim(), &mut out);
        } else {
            record_go_import(rest, &mut out);
        }
    }
    out
}

fn record_go_import(value: &str, out: &mut ImportBindingsBySpec) {
    let Some(quote_start) = value.find('"') else {
        return;
    };
    let tail = &value[quote_start + 1..];
    let Some(quote_end) = tail.find('"') else {
        return;
    };
    let spec = &tail[..quote_end];
    if spec.is_empty() {
        return;
    }
    let alias = value[..quote_start].trim();
    let local = if alias.is_empty() {
        Path::new(spec)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(spec)
    } else if alias == "." {
        "*"
    } else {
        alias
    };
    if local == "_" {
        return;
    }
    out.entry(spec.to_string())
        .or_default()
        .insert(local.to_string(), "*".to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_python_aliases_and_go_package_names() {
        let python = extract_python_import_bindings(
            "from app.owner import Thing as Alias, helper\nimport app.api as api\n",
        );
        assert_eq!(python["app.owner"]["Alias"], "Thing");
        assert_eq!(python["app.api"]["api"], "*");

        let go = extract_go_import_bindings(
            "import (\n  alias \"example.com/project/owner\"\n  \"example.com/project/plain\"\n  . \"example.com/project/dotted\"\n)\n",
        );
        assert_eq!(go["example.com/project/owner"]["alias"], "*");
        assert_eq!(go["example.com/project/plain"]["plain"], "*");
        assert_eq!(go["example.com/project/dotted"]["*"], "*");
    }
}
