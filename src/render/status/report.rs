// Responsibility: status-report-tables
use crate::map::StatusReport;
use crate::render::{bullet, code, table};

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
                    "Running executable".to_string(),
                    code(&report.identity.build_identity.executable_path),
                ],
                vec![
                    "Running version".to_string(),
                    report.identity.build_identity.semver.clone(),
                ],
                vec![
                    "Binary SHA-256".to_string(),
                    report
                        .identity
                        .build_identity
                        .binary_sha256
                        .as_ref()
                        .map(|hash| code(hash))
                        .unwrap_or_else(|| {
                            report.identity.build_identity.binary_sha256_state.clone()
                        }),
                ],
                vec![
                    "Source commit".to_string(),
                    report
                        .identity
                        .build_identity
                        .source_commit
                        .as_ref()
                        .map(|commit| code(commit))
                        .unwrap_or_else(|| "unknown".to_string()),
                ],
                vec![
                    "Dirty build".to_string(),
                    report
                        .identity
                        .build_identity
                        .dirty_build
                        .map(|dirty| dirty.to_string())
                        .unwrap_or_else(|| "unknown".to_string()),
                ],
                vec![
                    "Cache format".to_string(),
                    code(&report.identity.build_identity.cache_format),
                ],
                vec![
                    "Schema manifest version".to_string(),
                    report
                        .identity
                        .build_identity
                        .schema_manifest_version
                        .to_string(),
                ],
                vec![
                    "PATH executable".to_string(),
                    report
                        .identity
                        .path_identity
                        .executable_path
                        .as_ref()
                        .map(|path| code(path))
                        .unwrap_or_else(|| "not found".to_string()),
                ],
                vec![
                    "PATH version".to_string(),
                    report
                        .identity
                        .path_identity
                        .semver
                        .clone()
                        .unwrap_or_else(|| report.identity.path_identity.version_probe.clone()),
                ],
                vec![
                    "PATH binary SHA-256".to_string(),
                    report
                        .identity
                        .path_identity
                        .binary_sha256
                        .as_ref()
                        .map(|hash| code(hash))
                        .unwrap_or_else(|| {
                            report.identity.path_identity.binary_sha256_state.clone()
                        }),
                ],
                vec![
                    "Executable mismatch".to_string(),
                    report
                        .identity
                        .executable_mismatch
                        .map(|mismatch| mismatch.to_string())
                        .unwrap_or_else(|| "unknown (PATH executable not found)".to_string()),
                ],
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
                vec!["Cache strategy".to_string(), report.cache_strategy.clone()],
                vec![
                    "Per-file facts reused".to_string(),
                    report.cache_work.per_file_facts_reused.to_string(),
                ],
                vec![
                    "Per-file facts rebuilt".to_string(),
                    report.cache_work.per_file_facts_rebuilt.to_string(),
                ],
                vec![
                    "Reverse import strategy".to_string(),
                    report.cache_work.reverse_import_strategy.clone(),
                ],
                vec![
                    "Reverse targets rebuilt".to_string(),
                    report.cache_work.reverse_import_targets_rebuilt.to_string(),
                ],
                vec![
                    "Zero repo footprint default".to_string(),
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
                vec!["Files reused".to_string(), report.files_reused.to_string()],
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
                vec![
                    "cache_probe".to_string(),
                    report.timings.cache_probe_ms.to_string(),
                ],
                vec!["scan".to_string(), report.timings.scan_ms.to_string()],
                vec!["facts".to_string(), report.timings.facts_ms.to_string()],
                vec![
                    "reverse_index".to_string(),
                    report.timings.reverse_index_ms.to_string(),
                ],
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
    if !report.cache_diagnostics.is_empty() {
        println!("\n## Cache Diagnostics\n");
        let rows = report
            .cache_diagnostics
            .iter()
            .map(|event| {
                vec![
                    event.unix_seconds.to_string(),
                    event.operation.clone(),
                    code(&event.artifact),
                    event.outcome.clone(),
                    event.detail.clone(),
                ]
            })
            .collect();
        println!(
            "{}",
            table(
                &["Unix time", "Operation", "Artifact", "Outcome", "Detail"],
                rows,
            )
        );
    }
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
