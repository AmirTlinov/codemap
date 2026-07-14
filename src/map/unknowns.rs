fn unknown(
    kind: impl Into<String>,
    path: Option<impl Into<String>>,
    line_start: Option<usize>,
    reason: impl Into<String>,
    effect: impl Into<String>,
    expand: Option<impl Into<String>>,
) -> Unknown {
    Unknown {
        kind: kind.into(),
        path: path.map(Into::into),
        line_start,
        reason: reason.into(),
        effect: effect.into(),
        expand: expand.map(Into::into),
    }
}

fn unknown_unindexed_anchor(path: &str) -> Unknown {
    unknown(
        "unindexed_anchor",
        Some(path),
        None,
        "anchor is not indexed as a file or directory",
        "structural edges cannot be resolved for this anchor",
        Some(format!(
            "codemap ls {}",
            shell_quote(&parent_anchor_for_missing(path))
        )),
    )
}

fn unknown_symbol_outgoing(path: &str) -> Unknown {
    unknown(
        "unsupported_symbol_flow",
        Some(path),
        None,
        "symbol-level outgoing call/dataflow is not inferred without structural evidence",
        "use file or import edges when symbol body references are not structurally visible",
        Some(format!("codemap ls {}", shell_quote(path))),
    )
}

fn unknown_missing_symbol_anchor(file: &str, symbol: &str) -> Unknown {
    unknown(
        "missing_symbol_anchor",
        Some(file),
        None,
        format!("symbol `{symbol}` was not found in the indexed file"),
        "symbol-level structural map cannot be resolved; the file is not treated as the requested symbol",
        Some(format!("codemap ls {}", shell_quote(file))),
    )
}

fn unknown_directory_aggregate(path: &str, depth: usize) -> Unknown {
    unknown(
        "directory_aggregate",
        Some(path),
        None,
        "directory cone is aggregated at this level",
        "use a deeper anchor for file-level edges and exact evidence locations",
        Some(format!(
            "codemap cone {} --depth {}",
            shell_quote(path),
            depth + 1
        )),
    )
}

fn unknown_unresolved_import(path: &str, spec: &str, line_start: Option<usize>) -> Unknown {
    unknown(
        "unresolved_import",
        Some(path),
        line_start,
        format!("local import `{spec}` did not resolve to an indexed target"),
        "structural import edge is omitted until the target can be resolved",
        Some(format!("codemap ls {}", shell_quote(path))),
    )
}

fn unknown_missing_deterministic_proof(path: &str, expand: String) -> Unknown {
    unknown(
        "missing_deterministic_proof",
        Some(path),
        None,
        "no runnable command surface or direct linked verification surface was found for this proof scope",
        "linked, setup/support, and soft-match surfaces remain visible, but they do not create a direct linked surface",
        Some(expand),
    )
}

fn unknown_ci_validation_step_not_found(path: &str) -> Unknown {
    unknown(
        "ci_validation_step_not_found",
        Some(path),
        None,
        "no statically linked validation run step was found in this CI surface",
        "CI surfaces can show fallback commands, but no workflow validation step was linked from a CI run line",
        Some(format!(
            "codemap cone {} --section links",
            shell_quote(path)
        )),
    )
}

fn unresolved_import_unknowns(project: &Project, file: &FileInfo) -> Vec<Unknown> {
    let mut unknowns = file
        .unresolved_imports
        .iter()
        .map(|spec| {
            unknown_unresolved_import(
                &file.rel,
                spec,
                unresolved_import_line(project, file, spec),
            )
        })
        .collect::<Vec<_>>();
    unknowns.extend(rust_include_blind_spot_unknowns(project, file));
    unknowns
}

fn rust_include_blind_spot_unknowns(project: &Project, file: &FileInfo) -> Vec<Unknown> {
    if file.ext != "rs" {
        return Vec::new();
    }
    let Ok(text) = std::fs::read_to_string(project.root.join(&file.rel)) else {
        return Vec::new();
    };
    repo::rust_include_blind_spot_lines(&text)
        .into_iter()
        .map(|line| {
            unknown(
                "rust_include_unresolved",
                Some(&file.rel),
                Some(line),
                "Rust include! target is not a static .rs string literal",
                "included Rust fragments share the parent lexical scope; dependencies inside unresolved include! fragments are not represented in this cone",
                Some(format!("codemap ls {}", shell_quote(&file.rel))),
            )
        })
        .collect()
}

fn unresolved_import_line(project: &Project, file: &FileInfo, spec: &str) -> Option<usize> {
    let text = std::fs::read_to_string(project.root.join(&file.rel)).ok()?;
    text.lines()
        .enumerate()
        .find(|(_, line)| line.contains(spec) && line_looks_like_import_or_reexport(line.trim()))
        .map(|(index, _)| index + 1)
}
