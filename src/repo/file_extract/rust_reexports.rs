// Responsibility: repo-rust-use-bindings
use std::collections::BTreeSet;

use crate::model::ImportBindingsBySpec;
use crate::repo::code_without_comments_or_strings;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct RustUseFacts {
    pub(crate) imports: BTreeSet<String>,
    pub(crate) bindings: ImportBindingsBySpec,
}

/// Expands Rust use trees into the same resolved import/binding facts used by
/// the symbol edge owner. A leaf keeps its complete static path as the import
/// spec; resolution later selects the longest module prefix that exists.
pub(crate) fn extract_rust_use_facts(text: &str) -> RustUseFacts {
    let cleaned = code_without_comments_or_strings(text, "rs");
    let mut facts = RustUseFacts::default();
    for statement in rust_use_statements(&cleaned) {
        let Some((tree, reexported)) = rust_use_tree(statement) else {
            continue;
        };
        let mut leaves = Vec::new();
        expand_use_tree(tree, "", &mut leaves);
        for leaf in leaves {
            record_leaf(&mut facts, leaf, reexported);
        }
    }
    facts
}

/// Crate-relative paths used outside a `use` statement are imports too. They
/// are recorded as path-shaped locals so the reference edge can retain the
/// exact qualified expression and line rather than degrading to a file edge.
pub(crate) fn extract_rust_qualified_path_facts(text: &str) -> RustUseFacts {
    let cleaned = code_without_comments_or_strings(text, "rs");
    let without_uses = blank_rust_use_statements(&cleaned);
    let bytes = without_uses.as_bytes();
    let mut facts = RustUseFacts::default();
    let mut index = 0;
    while index < bytes.len() {
        if !rust_ident_start(bytes[index]) {
            index += 1;
            continue;
        }
        let start = index;
        index = rust_ident_end(bytes, index);
        let root = &without_uses[start..index];
        if !matches!(root, "crate" | "self" | "super") || bytes.get(index..index + 2) != Some(b"::")
        {
            continue;
        }
        let mut end = index;
        while bytes.get(end..end + 2) == Some(b"::") {
            let segment_start = end + 2;
            if !bytes
                .get(segment_start)
                .is_some_and(|byte| rust_ident_start(*byte))
            {
                break;
            }
            end = rust_ident_end(bytes, segment_start);
        }
        if end <= index + 2 {
            continue;
        }
        let path = without_uses[start..end].to_string();
        let imported = path.rsplit("::").next().unwrap_or_default().to_string();
        facts.imports.insert(path.clone());
        facts
            .bindings
            .entry(path.clone())
            .or_default()
            .insert(path, imported);
        index = end;
    }
    facts
}

#[derive(Debug)]
struct UseLeaf {
    target: String,
    source: String,
    local: String,
    glob: bool,
}

fn rust_use_statements(cleaned: &str) -> Vec<&str> {
    let bytes = cleaned.as_bytes();
    let mut out = Vec::new();
    let mut line_start = 0;
    while line_start < bytes.len() {
        let line_end = bytes[line_start..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|offset| line_start + offset)
            .unwrap_or(bytes.len());
        let line = &cleaned[line_start..line_end];
        let trimmed = line.trim_start();
        if rust_use_tree(trimmed).is_some() {
            let statement_start = line_start + (line.len() - trimmed.len());
            let statement_end = bytes[statement_start..]
                .iter()
                .position(|byte| *byte == b';')
                .map(|offset| statement_start + offset + 1)
                .unwrap_or(line_end);
            out.push(&cleaned[statement_start..statement_end]);
            line_start = statement_end;
            continue;
        }
        line_start = (line_end + 1).min(bytes.len());
    }
    out
}

fn blank_rust_use_statements(cleaned: &str) -> String {
    let mut out = cleaned.as_bytes().to_vec();
    for statement in rust_use_statements(cleaned) {
        let start = statement.as_ptr() as usize - cleaned.as_ptr() as usize;
        for byte in &mut out[start..start + statement.len()] {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
    }
    String::from_utf8(out).expect("Rust source remains UTF-8 after blanking uses")
}

fn rust_use_tree(statement: &str) -> Option<(&str, bool)> {
    let trimmed = statement.trim_start();
    let (after_visibility, reexported) = if let Some(rest) = trimmed.strip_prefix("pub ") {
        (rest, true)
    } else if trimmed.starts_with("pub(") {
        let close = trimmed.find(')')?;
        (trimmed[close + 1..].trim_start(), true)
    } else {
        (trimmed, false)
    };
    let tree = after_visibility.strip_prefix("use ")?.trim();
    Some((tree.trim_end_matches(';').trim(), reexported))
}

fn expand_use_tree(tree: &str, prefix: &str, out: &mut Vec<UseLeaf>) {
    let tree = tree.trim();
    if tree.is_empty() {
        return;
    }
    if let Some(open) = top_level_open_brace(tree) {
        let Some(close) = matching_brace(tree, open) else {
            return;
        };
        if !tree[close + 1..].trim().is_empty() {
            return;
        }
        let branch = tree[..open].trim().trim_end_matches("::");
        let next_prefix = join_rust_path(prefix, branch);
        for item in split_top_level_commas(&tree[open + 1..close]) {
            expand_use_tree(item, &next_prefix, out);
        }
        return;
    }
    let (path, alias) = split_alias(tree);
    if let Some(target) = path.strip_suffix("::*") {
        let target = join_rust_path(prefix, target);
        if !target.is_empty() {
            out.push(UseLeaf {
                target,
                source: "*".to_string(),
                local: "*".to_string(),
                glob: true,
            });
        }
        return;
    }
    if path == "*" {
        if !prefix.is_empty() {
            out.push(UseLeaf {
                target: prefix.to_string(),
                source: "*".to_string(),
                local: "*".to_string(),
                glob: true,
            });
        }
        return;
    }
    if path == "self" {
        let source = prefix.rsplit("::").next().unwrap_or(prefix);
        if !prefix.is_empty() && !source.is_empty() {
            out.push(UseLeaf {
                target: prefix.to_string(),
                source: source.to_string(),
                local: alias.unwrap_or(source).to_string(),
                glob: false,
            });
        }
        return;
    }
    let target = join_rust_path(prefix, path);
    let source = path.rsplit("::").next().unwrap_or(path);
    if !target.is_empty() && !source.is_empty() {
        out.push(UseLeaf {
            target,
            source: source.to_string(),
            local: alias.unwrap_or(source).to_string(),
            glob: false,
        });
    }
}

fn record_leaf(facts: &mut RustUseFacts, leaf: UseLeaf, reexported: bool) {
    facts.imports.insert(leaf.target.clone());
    let bindings = facts.bindings.entry(leaf.target).or_default();
    if reexported {
        if leaf.glob {
            bindings.insert("export:*".to_string(), "*".to_string());
        } else {
            bindings.insert(format!("export:{}", leaf.local), leaf.source);
        }
    } else {
        bindings.insert(leaf.local, leaf.source);
    }
}

fn top_level_open_brace(value: &str) -> Option<usize> {
    value
        .char_indices()
        .find_map(|(index, ch)| (ch == '{').then_some(index))
}

fn matching_brace(value: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, ch) in value[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(open + offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn split_top_level_commas(value: &str) -> Vec<&str> {
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut out = Vec::new();
    for (index, ch) in value.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                out.push(&value[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    out.push(&value[start..]);
    out
}

fn split_alias(value: &str) -> (&str, Option<&str>) {
    value
        .rsplit_once(" as ")
        .map(|(path, alias)| (path.trim(), Some(alias.trim())))
        .unwrap_or((value.trim(), None))
}

fn join_rust_path(prefix: &str, path: &str) -> String {
    match (prefix.trim_end_matches("::"), path.trim_start_matches("::")) {
        ("", path) => path.to_string(),
        (prefix, "") => prefix.to_string(),
        (prefix, path) => format!("{prefix}::{path}"),
    }
}

fn rust_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn rust_ident_end(bytes: &[u8], mut index: usize) -> usize {
    while bytes
        .get(index)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
    {
        index += 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_grouped_alias_glob_and_qualified_rust_paths() {
        let text = r#"
use crate::{map, model::{ConeReport as Report, Project}};
pub(crate) use super::where_locator::*;
fn run(_: crate::model::ConeReport) { map::where_report(); }
"#;
        let uses = extract_rust_use_facts(text);
        assert_eq!(uses.bindings["crate::map"]["map"], "map");
        assert_eq!(
            uses.bindings["crate::model::ConeReport"]["Report"],
            "ConeReport"
        );
        assert_eq!(uses.bindings["super::where_locator"]["export:*"], "*");
        let qualified = extract_rust_qualified_path_facts(text);
        assert_eq!(
            qualified.bindings["crate::model::ConeReport"]["crate::model::ConeReport"],
            "ConeReport"
        );
    }
}
