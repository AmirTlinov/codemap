use serde::Serialize;
use std::path::Path;
use std::sync::OnceLock;

use crate::map::StatusReport;
use crate::model::{
    BoundaryFinding, BoundaryMapReport, ChangedReport, ConeReport, ContractReport, DeleteReport,
    DiffMapReport, EnvSurface, EvidenceLocation, FlowReport, GraphEdge, GraphLens, ImpactCluster,
    ImpactReport, LsReport, PlaceReport, ProofMapReport, ProofReport, ProofSurface, RuntimeReport,
    RuntimeRoute, SiblingsReport, StructuralEdge, Surface, TeachReport, Unknown,
};

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

fn rewrite_expand_fields(value: &mut serde_json::Value) {
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

pub fn status(report: &StatusReport, doctor: bool) {
    println!(
        "{}",
        if doctor {
            "# codemap doctor"
        } else {
            "# codemap status"
        }
    );
    println!();
    println!(
        "{}",
        table(
            &["Field", "Value"],
            vec![
                vec!["Root".to_string(), code(&report.root)],
                vec!["CWD".to_string(), code(&report.cwd)],
                vec![
                    "VCS".to_string(),
                    report.vcs.clone().unwrap_or_else(|| "none".to_string())
                ],
                vec![
                    "Config".to_string(),
                    report
                        .config
                        .as_ref()
                        .map(|x| code(x))
                        .unwrap_or_else(|| "zero-config".to_string()),
                ],
                vec![
                    "Nearest AGENTS.md".to_string(),
                    report
                        .nearest_agents
                        .as_ref()
                        .map(|x| code(x))
                        .unwrap_or_else(|| "none".to_string()),
                ],
                vec!["Cache".to_string(), code(&report.cache_dir)],
                vec!["Cache state".to_string(), report.cache_state.clone()],
                vec![
                    "Cache strategy".to_string(),
                    report.cache_strategy.clone()
                ],
                vec![
                    "Zero-footprint default".to_string(),
                    report.zero_footprint_default.to_string()
                ],
                vec![
                    "Package manager".to_string(),
                    report.package_manager.clone()
                ],
                vec![
                    "Languages".to_string(),
                    if report.languages.is_empty() {
                        "unknown".to_string()
                    } else {
                        report.languages.join(", ")
                    },
                ],
                vec![
                    "Files scanned".to_string(),
                    report.files_scanned.to_string()
                ],
                vec![
                    "Files reused".to_string(),
                    report.files_reused.to_string()
                ],
                vec![
                    "Files visited".to_string(),
                    report.scanner.files_visited.to_string()
                ],
                vec![
                    "Files skipped".to_string(),
                    report.scanner.files_skipped.to_string()
                ],
                vec![
                    "Bytes scanned".to_string(),
                    report.scanner.bytes_scanned.to_string()
                ],
                vec!["Fingerprint".to_string(), code(&report.fingerprint)],
                vec![
                    "Boundary findings".to_string(),
                    report.boundary_findings.to_string()
                ],
            ],
        )
    );
    println!("\n## Project Timings\n");
    println!(
        "{}",
        table(
            &["Phase", "ms"],
            vec![
                vec!["root".to_string(), report.timings.root_ms.to_string()],
                vec!["scan".to_string(), report.timings.scan_ms.to_string()],
                vec!["facts".to_string(), report.timings.facts_ms.to_string()],
                vec![
                    "cache_artifacts".to_string(),
                    report.timings.cache_artifact_ms.to_string()
                ],
                vec![
                    "cache_write".to_string(),
                    report.timings.cache_write_ms.to_string()
                ],
                vec!["total".to_string(), report.timings.total_ms.to_string()],
            ],
        )
    );
    if !report.domains.is_empty() {
        println!("\n## Domains\n");
        let rows = report
            .domains
            .iter()
            .map(|d| {
                vec![
                    d.id.clone(),
                    code(&d.path),
                    d.config
                        .as_ref()
                        .map(|x| code(x))
                        .unwrap_or_else(|| "no".to_string()),
                ]
            })
            .collect();
        println!("{}", table(&["ID", "Path", "Semantic config"], rows));
    }
    if !report.cache_artifacts.is_empty() {
        println!("\n## Cache Artifacts\n");
        let rows = report
            .cache_artifacts
            .iter()
            .map(|artifact| {
                vec![
                    code(&artifact.name),
                    artifact.exists.to_string(),
                    artifact
                        .bytes
                        .map(|bytes| bytes.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    artifact
                        .fingerprint_match
                        .map(|matches| matches.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                ]
            })
            .collect();
        println!(
            "{}",
            table(&["Artifact", "Exists", "Bytes", "Fingerprint match"], rows)
        );
    }
    if !report.scanner.ignored.is_empty() || !report.scanner.generated.is_empty() {
        println!("\n## Scanner Groups\n");
        let mut rows = Vec::new();
        for group in &report.scanner.ignored {
            rows.push(vec![
                "ignored".to_string(),
                code(&group.reason),
                group.count.to_string(),
                group.examples.join(", "),
            ]);
        }
        for group in &report.scanner.generated {
            rows.push(vec![
                "generated".to_string(),
                code(&group.reason),
                group.count.to_string(),
                group.examples.join(", "),
            ]);
        }
        println!("{}", table(&["Kind", "Reason", "Count", "Examples"], rows));
    }
    if !report.config_errors.is_empty() {
        println!("\n## Anchor Config Errors\n");
        println!("{}", bullet(&report.config_errors, false, Some(10)));
    }
    if !report.map_quality.is_empty() {
        println!("\n## Map Quality Warnings\n");
        let rows = report
            .map_quality
            .iter()
            .map(|warning| {
                vec![
                    code(&warning.kind),
                    warning.count.to_string(),
                    warning.examples.join(", "),
                    warning.effect.clone(),
                    warning
                        .expand
                        .as_ref()
                        .map(|command| code(command))
                        .unwrap_or_else(|| "-".to_string()),
                ]
            })
            .collect();
        println!(
            "{}",
            table(&["Kind", "Count", "Examples", "Effect", "Expand"], rows)
        );
    }
    if !report.scripts.is_empty() {
        println!("\n## Verification Hints\n");
        println!("{}", bullet(&report.scripts, true, Some(10)));
    }
    if report.unclassified_count > 0 {
        println!(
            "\n## Source Files With Only Generic Hints ({})\n",
            report.unclassified_count
        );
        println!(
            "These files are indexed as source, but codemap found no stronger deterministic path/name/manifest pattern. This is not an intent, ownership, or correctness verdict.\n"
        );
        println!(
            "{}",
            bullet(&report.unclassified_source_files, true, Some(30))
        );
    }
}

pub fn teach(report: &TeachReport) {
    println!("# Repo Dialect Draft\n");
    println!("No repository files were written.\n");
    println!(
        "{}",
        table(
            &["Field", "Value"],
            vec![
                vec![
                    "Config".to_string(),
                    report
                        .config
                        .as_ref()
                        .map(|value| code(value))
                        .unwrap_or_else(|| "zero-config".to_string()),
                ],
                vec![
                    "Surface hint patterns".to_string(),
                    report.role_patterns.len().to_string(),
                ],
                vec![
                    "Proof changed commands".to_string(),
                    report.proof_changed.len().to_string(),
                ],
            ],
        )
    );
    if !report.role_patterns.is_empty() {
        println!("\n## Surface Hints\n");
        println!("Derived from deterministic configured patterns. Not intent, correctness, or ownership truth.\n");
        for role in &report.role_patterns {
            println!(
                "- `{}` -> `{}` [{}; matched: `{}`]",
                role.pattern, role.role, role.evidence, role.matched
            );
            if !role.examples.is_empty() {
                println!("  examples: {}", role.examples.join(", "));
            }
        }
    }
    if !report.proof_changed.is_empty() {
        println!("\n## Proof Changed\n");
        for command in &report.proof_changed {
            let source = command
                .source
                .as_ref()
                .map(|path| match command.line_start {
                    Some(line) => format!("{path}:{line}"),
                    None => path.clone(),
                })
                .unwrap_or_else(|| "script catalog".to_string());
            println!(
                "- `{}` [{}; source: `{}`]",
                command.command, command.evidence, source
            );
        }
    }
    if !report.ctx_yml.is_empty() {
        println!("\n## ctx.yml\n");
        println!("{}", code_block("yaml", &report.ctx_yml));
    }
    section("Expand", &report.expand);
}
