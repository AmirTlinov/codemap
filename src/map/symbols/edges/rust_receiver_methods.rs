// Responsibility: map-symbols-rust-typed-receiver-methods
use crate::map::{
    imported_symbol_owner, matching_symbols, non_js_code_line_without_strings_and_comments,
    sort_edges, structural_edge_with_locations, symbol_anchor_path,
};
use crate::model::{EvidenceLocation, EvidenceStrength, FileInfo, Project, StructuralEdge};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn rust_typed_receiver_method_edges(
    project: &Project,
    info: &FileInfo,
    symbol_name: &str,
) -> Vec<StructuralEdge> {
    if info.ext != "rs" {
        return Vec::new();
    }
    let Some(symbol) = matching_symbols(info, symbol_name).into_iter().next() else {
        return Vec::new();
    };
    let Some(text) = project.read_indexed_text(&info.rel) else {
        return Vec::new();
    };
    let mut receiver_types = rust_signature_receiver_types(&text, symbol.line_start, symbol_name);
    if let Some(owner_type) = enclosing_rust_impl_type(info, symbol.line_start, symbol.line_end) {
        receiver_types.insert("self".to_string(), owner_type);
    }
    if receiver_types.is_empty() {
        return Vec::new();
    }
    let mut edges = Vec::new();
    let mut state = crate::map::NonJsCodeState::default();
    for (offset, line) in text
        .lines()
        .skip(symbol.line_start.saturating_sub(1))
        .take(symbol.line_end.saturating_sub(symbol.line_start) + 1)
        .enumerate()
    {
        let code = non_js_code_line_without_strings_and_comments(line, "rs", &mut state);
        for (receiver, method) in rust_receiver_calls(&code) {
            let Some(type_name) = receiver_types.get(&receiver) else {
                continue;
            };
            let Some((owner_rel, owner_method)) =
                unique_rust_method_owner(project, info, type_name, &method)
            else {
                continue;
            };
            edges.push(structural_edge_with_locations(
                symbol_anchor_path(&info.rel, symbol_name),
                symbol_anchor_path(&owner_rel, &owner_method),
                "symbol_uses",
                "typed_receiver_method_in_symbol_body",
                EvidenceStrength::High,
                vec![EvidenceLocation::line(
                    &info.rel,
                    symbol.line_start + offset,
                    "typed_receiver_method_call",
                )],
            ));
        }
    }
    sort_edges(&mut edges);
    edges.dedup_by(|left, right| left.from == right.from && left.to == right.to);
    edges
}

fn rust_signature_receiver_types(
    text: &str,
    line_start: usize,
    symbol_name: &str,
) -> BTreeMap<String, String> {
    let prefix = text
        .lines()
        .skip(line_start.saturating_sub(1))
        .take(12)
        .collect::<Vec<_>>();
    let signature = prefix
        .iter()
        .copied()
        .take_while(|line| !line.contains('{'))
        .chain(prefix.iter().copied().find(|line| line.contains('{')))
        .collect::<Vec<_>>()
        .join(" ");
    let Some(function_start) = signature.find(&format!("fn {symbol_name}")) else {
        return BTreeMap::new();
    };
    let Some((_, parameters)) = signature[function_start..].split_once('(') else {
        return BTreeMap::new();
    };
    let parameters = parameters
        .split_once(')')
        .map_or(parameters, |(head, _)| head);
    parameters
        .split(',')
        .filter_map(rust_typed_binding)
        .collect()
}

fn rust_typed_binding(parameter: &str) -> Option<(String, String)> {
    let (binding, type_name) = parameter.split_once(':')?;
    let binding = binding
        .split_whitespace()
        .rfind(|part| !matches!(*part, "mut" | "ref"))?;
    let type_name = type_name.trim().trim_start_matches('&').trim_start();
    let type_name = type_name.strip_prefix("mut ").unwrap_or(type_name);
    let type_name = type_name
        .split(['<', '[', ' '])
        .next()?
        .rsplit("::")
        .next()?;
    valid_rust_identifier(binding)
        .then(|| (binding.to_string(), type_name.to_string()))
        .filter(|(_, type_name)| valid_rust_identifier(type_name))
}

fn enclosing_rust_impl_type(info: &FileInfo, line_start: usize, line_end: usize) -> Option<String> {
    let mut owners = info
        .symbols
        .iter()
        .filter(|symbol| {
            symbol.kind == "impl" && symbol.line_start < line_start && symbol.line_end >= line_end
        })
        .map(|symbol| rust_impl_target_type(&symbol.name))
        .collect::<BTreeSet<_>>();
    (owners.len() == 1).then(|| owners.pop_first().expect("one impl owner"))
}

fn rust_impl_target_type(name: &str) -> String {
    name.rsplit(" for ")
        .next()
        .unwrap_or(name)
        .split('<')
        .next()
        .unwrap_or(name)
        .rsplit("::")
        .next()
        .unwrap_or(name)
        .trim()
        .to_string()
}

fn rust_receiver_calls(line: &str) -> Vec<(String, String)> {
    let bytes = line.as_bytes();
    let mut calls = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if !rust_identifier_start(bytes[index]) {
            index += 1;
            continue;
        }
        let receiver_start = index;
        index += 1;
        while bytes
            .get(index)
            .is_some_and(|byte| rust_identifier_continue(*byte))
        {
            index += 1;
        }
        let receiver = &line[receiver_start..index];
        let mut dot = index;
        while bytes.get(dot).is_some_and(u8::is_ascii_whitespace) {
            dot += 1;
        }
        if bytes.get(dot) != Some(&b'.') {
            continue;
        }
        let mut method_start = dot + 1;
        while bytes.get(method_start).is_some_and(u8::is_ascii_whitespace) {
            method_start += 1;
        }
        if !bytes
            .get(method_start)
            .is_some_and(|byte| rust_identifier_start(*byte))
        {
            continue;
        }
        let mut method_end = method_start + 1;
        while bytes
            .get(method_end)
            .is_some_and(|byte| rust_identifier_continue(*byte))
        {
            method_end += 1;
        }
        let mut call = method_end;
        while bytes.get(call).is_some_and(u8::is_ascii_whitespace) {
            call += 1;
        }
        if bytes.get(call) == Some(&b'(') {
            calls.push((
                receiver.to_string(),
                line[method_start..method_end].to_string(),
            ));
        }
        index = method_end;
    }
    calls
}

fn unique_rust_method_owner(
    project: &Project,
    source: &FileInfo,
    type_name: &str,
    method: &str,
) -> Option<(String, String)> {
    let mut owners = rust_type_owner_files(project, source, type_name)
        .into_iter()
        .filter_map(|rel| project.files.get(&rel))
        .filter(|file| rust_file_impl_defines_method(file, type_name, method))
        .map(|file| (file.rel.clone(), method.to_string()))
        .collect::<Vec<_>>();
    owners.sort();
    owners.dedup();
    (owners.len() == 1).then(|| owners.remove(0))
}

fn rust_type_owner_files(
    project: &Project,
    source: &FileInfo,
    type_name: &str,
) -> BTreeSet<String> {
    let mut files = BTreeSet::new();
    if source
        .symbols
        .iter()
        .any(|symbol| symbol.name == type_name && matches!(symbol.kind.as_str(), "struct" | "enum"))
    {
        files.insert(source.rel.clone());
    }
    for (target_rel, bindings) in &source.resolved_import_bindings {
        for (local, imported) in bindings {
            if local == type_name
                && let Some(owner) = imported_symbol_owner(project, target_rel, imported)
            {
                files.insert(owner.rel);
            }
        }
    }
    files
}

fn rust_file_impl_defines_method(file: &FileInfo, type_name: &str, method: &str) -> bool {
    file.symbols.iter().any(|implementation| {
        implementation.kind == "impl"
            && rust_impl_target_type(&implementation.name) == type_name
            && file.symbols.iter().any(|symbol| {
                symbol.name == method
                    && symbol.line_start > implementation.line_start
                    && symbol.line_end <= implementation.line_end
            })
    })
}

fn valid_rust_identifier(value: &str) -> bool {
    value.bytes().next().is_some_and(rust_identifier_start)
        && value.bytes().all(rust_identifier_continue)
}

fn rust_identifier_start(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphabetic()
}

fn rust_identifier_continue(byte: u8) -> bool {
    rust_identifier_start(byte) || byte.is_ascii_digit()
}
