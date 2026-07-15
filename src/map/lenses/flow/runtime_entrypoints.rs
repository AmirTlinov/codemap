// Responsibility: flow-lens-runtime-entrypoints
use crate::map::{
    NonJsCodeState, c_like_code_line_without_strings_and_comments, identifier_ranges,
    next_nonspace_byte, previous_nonspace_byte, runtime_manifest_entrypoints, symbol_is_top_level,
};
use crate::model::{EvidenceLocation, FileInfo, FlowStep, Project, Surface};
use crate::repo;
use std::path::Path;

pub(crate) fn runtime_entrypoint_surface_for_file(project: &Project, rel: &str) -> Option<Surface> {
    let mut surfaces = Vec::new();
    for package in &project.packages {
        let Some(manifest) = project.files.get(&package.manifest) else {
            continue;
        };
        surfaces.extend(runtime_manifest_entrypoints(project, manifest));
    }
    surfaces
        .into_iter()
        .find(|surface| surface.path.as_deref() == Some(rel))
}

pub(crate) fn runtime_entrypoint_symbol_step(
    project: &Project,
    file: &FileInfo,
) -> Option<(FlowStep, crate::model::SymbolInfo)> {
    let symbol = file.symbols.iter().find(|symbol| {
        file.language == "rust"
            && symbol.name == "main"
            && symbol.kind == "function"
            && symbol_is_top_level(project, file, symbol)
    })?;
    let step = FlowStep {
        index: 0,
        anchor: format!("{}#main", file.rel),
        kind: "entry_symbol".to_string(),
        evidence: "rust_main_symbol".to_string(),
        locations: vec![EvidenceLocation {
            path: file.rel.clone(),
            line_start: Some(symbol.line_start),
            line_end: Some(symbol.line_end),
            kind: "entry_symbol".to_string(),
        }],
    };
    Some((step, symbol.clone()))
}

pub(crate) fn runtime_entrypoint_direct_call_steps(
    project: &Project,
    file: &FileInfo,
    entry_symbol: &crate::model::SymbolInfo,
) -> Vec<FlowStep> {
    if file.language != "rust" {
        return Vec::new();
    }
    let Some(text) = project.read_indexed_text(&file.rel) else {
        return Vec::new();
    };
    let body_lines = text
        .lines()
        .enumerate()
        .skip(entry_symbol.line_start.saturating_sub(1))
        .take(
            entry_symbol
                .line_end
                .saturating_sub(entry_symbol.line_start)
                .saturating_add(1),
        )
        .collect::<Vec<_>>();
    let mut steps = Vec::new();
    for target in file.symbols.iter().filter(|symbol| {
        symbol.kind == "function"
            && symbol.name != entry_symbol.name
            && symbol_is_top_level(project, file, symbol)
    }) {
        if let Some(line) = rust_direct_call_line(&body_lines, &target.name) {
            steps.push(FlowStep {
                index: 0,
                anchor: format!("{}#{}", file.rel, target.name),
                kind: "entry_call".to_string(),
                evidence: "rust_entry_direct_call".to_string(),
                locations: vec![EvidenceLocation::line(&file.rel, line, "entry_call")],
            });
        }
    }
    steps.sort_by(|a, b| {
        a.locations
            .first()
            .and_then(|location| location.line_start)
            .cmp(&b.locations.first().and_then(|location| location.line_start))
            .then_with(|| a.anchor.cmp(&b.anchor))
    });
    steps
}

fn rust_direct_call_line(body_lines: &[(usize, &str)], name: &str) -> Option<usize> {
    let mut state = NonJsCodeState::default();
    for (index, line) in body_lines {
        let code = c_like_code_line_without_strings_and_comments(line, "rs", &mut state);
        if rust_line_has_direct_call(&code, name) {
            return Some(index + 1);
        }
    }
    None
}

fn rust_line_has_direct_call(line: &str, name: &str) -> bool {
    identifier_ranges(line, name).any(|(start, end)| {
        let before = &line[..start];
        let after = &line[end..];
        !matches!(previous_nonspace_byte(before), Some(b'.' | b':'))
            && matches!(next_nonspace_byte(after), Some(b'('))
    })
}

pub(crate) fn runtime_entrypoint_locations(
    project: &Project,
    rel: &str,
    surface: &Surface,
) -> Vec<EvidenceLocation> {
    let Some((manifest, command)) = cli_entrypoint_manifest_and_command(surface) else {
        return vec![EvidenceLocation::path(rel, "runtime_entrypoint")];
    };
    if surface.evidence == "cargo_bin_target"
        && let Some(line) = cargo_bin_target_line(project, manifest, rel, command)
    {
        return vec![EvidenceLocation::line(manifest, line, "cargo_bin_target")];
    }
    vec![EvidenceLocation::path(rel, "runtime_entrypoint")]
}

fn cli_entrypoint_manifest_and_command(surface: &Surface) -> Option<(&str, &str)> {
    let rest = surface.id.strip_prefix("surface:cli_entrypoint:")?;
    rest.rsplit_once(':')
}

fn cargo_bin_target_line(
    project: &Project,
    manifest: &str,
    rel: &str,
    command: &str,
) -> Option<usize> {
    let package_path = Path::new(manifest)
        .parent()
        .map(|parent| repo::normalize_rel_path(&parent.to_string_lossy()))
        .filter(|parent| !parent.is_empty())
        .unwrap_or_else(|| ".".to_string());
    let target = rel
        .strip_prefix(&format!("{}/", package_path.trim_end_matches('/')))
        .unwrap_or(rel);
    cargo_bin_table_path_line(project, manifest, command, target)
}

fn cargo_bin_table_path_line(
    project: &Project,
    manifest: &str,
    command: &str,
    target: &str,
) -> Option<usize> {
    let text = project.read_indexed_text(manifest)?;
    let mut current = CargoBinTable::default();
    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if let Some(path_line) = current.matching_path_line(command, target) {
                return Some(path_line);
            }
            current = CargoBinTable {
                in_bin: trimmed == "[[bin]]",
                ..CargoBinTable::default()
            };
            continue;
        }
        if !current.in_bin || trimmed.starts_with('#') {
            continue;
        }
        if let Some(value) = toml_string_assignment(trimmed, "name") {
            current.name = Some(value.to_string());
        } else if let Some(value) = toml_string_assignment(trimmed, "path") {
            current.path = Some(repo::normalize_rel_path(value.trim_start_matches("./")));
            current.path_line = Some(line_number);
        }
    }
    current.matching_path_line(command, target)
}

#[derive(Default)]
struct CargoBinTable {
    in_bin: bool,
    name: Option<String>,
    path: Option<String>,
    path_line: Option<usize>,
}

impl CargoBinTable {
    fn matching_path_line(&self, command: &str, target: &str) -> Option<usize> {
        (self.name.as_deref() == Some(command)
            && self.path.as_deref() == Some(repo::normalize_rel_path(target).as_str()))
        .then_some(self.path_line?)
    }
}

fn toml_string_assignment<'a>(trimmed: &'a str, key: &str) -> Option<&'a str> {
    let (left, right) = trimmed.split_once('=')?;
    if left.trim() != key {
        return None;
    }
    let right = right.trim_start();
    let quote = right
        .chars()
        .next()
        .filter(|ch| *ch == '"' || *ch == '\'')?;
    let value = &right[quote.len_utf8()..];
    let end = value.find(quote)?;
    Some(&value[..end])
}
