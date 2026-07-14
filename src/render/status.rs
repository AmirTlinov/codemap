// Responsibility: render-status
use crate::map::StatusReport;
use crate::render::{bullet, code, table};
use serde::Serialize;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

static EXPAND_ROOT: OnceLock<String> = OnceLock::new();
static MAP_SNAPSHOT: OnceLock<String> = OnceLock::new();
static BRIEF: OnceLock<bool> = OnceLock::new();

pub fn set_expand_root(root: Option<&Path>) {
    if let Some(root) = root {
        let _ = EXPAND_ROOT.set(root.to_string_lossy().to_string());
    }
}

pub fn set_brief(value: bool) {
    let _ = BRIEF.set(value);
}

pub(crate) fn brief() -> bool {
    BRIEF.get().copied().unwrap_or(false)
}

pub fn set_map_snapshot(project: &crate::model::Project) {
    let fingerprint = crate::cache::fingerprint(project, None);
    set_map_snapshot_full(
        &project.root,
        Some(&fingerprint),
        Some(&project.cache_state),
        Some(&project.cache_strategy),
        Some(project.cache_dir.to_string_lossy().as_ref()),
    );
}

pub fn set_cached_map_snapshot_parts(root: &Path, fingerprint: Option<&str>, cache_dir: &Path) {
    set_map_snapshot_full(
        root,
        fingerprint,
        Some("hit"),
        Some("cached_lens"),
        Some(cache_dir.to_string_lossy().as_ref()),
    );
}

pub fn set_inventory_map_snapshot_parts(root: &Path, fingerprint: Option<&str>, cache_dir: &Path) {
    let cache_state = if !crate::cache::cache_enabled() {
        "disabled"
    } else if crate::cache::cached_status_fingerprint(cache_dir).is_some() {
        "stale"
    } else {
        "cold"
    };
    set_map_snapshot_full(
        root,
        fingerprint,
        Some(cache_state),
        Some("inventory_fast_path"),
        Some(cache_dir.to_string_lossy().as_ref()),
    );
}

// Cache telemetry (state/strategy/external_cache/location) is debug-only: agents
// never need the cache path or fingerprint provenance. It is opt-in on the snapshot
// line via CODEMAP_CACHE_TELEMETRY=1 and otherwise stays only in the `doctor` and
// `status` tables.
fn cache_telemetry_enabled() -> bool {
    std::env::var("CODEMAP_CACHE_TELEMETRY").is_ok_and(|value| !value.is_empty() && value != "0")
}

// The snapshot token is the full per-file fingerprint; it doubles as the
// `--since` delta token (see cache::snapshots).
fn set_map_snapshot_full(
    root: &Path,
    fingerprint: Option<&str>,
    cache_state: Option<&str>,
    cache_strategy: Option<&str>,
    cache_location: Option<&str>,
) {
    let fingerprint = fingerprint.unwrap_or("unknown");
    let snapshot = fingerprint.get(..16).unwrap_or(fingerprint);
    let line = if brief() {
        format!("Map Snapshot: snapshot=`{snapshot}`; schema=`structural:5`; repo_footprint=`zero`")
    } else {
        let head = map_snapshot_git_head(root).unwrap_or_else(|| "none".to_string());
        let branch = map_snapshot_git_branch(root).unwrap_or_else(|| "unknown".to_string());
        let dirty = map_snapshot_dirty_count(root).unwrap_or(0);
        if cache_telemetry_enabled() {
            let short_fingerprint = fingerprint.get(..12).unwrap_or(fingerprint);
            let external_cache = if crate::cache::cache_enabled() {
                "enabled"
            } else {
                "disabled"
            };
            format!(
                "Map Snapshot: root=`{}`; head=`{}`; branch=`{}`; dirty=`{}`; snapshot=`{}`; fingerprint=`{}`; cache=`{}` strategy=`{}` external_cache=`{}` location=`{}`; schema=`structural:5`; repo_footprint=`zero`",
                root.to_string_lossy(),
                head,
                branch,
                dirty,
                snapshot,
                short_fingerprint,
                cache_state.unwrap_or("unknown"),
                cache_strategy.unwrap_or("unknown"),
                external_cache,
                cache_location.unwrap_or("unknown"),
            )
        } else {
            format!(
                "Map Snapshot: root=`{}`; head=`{}`; branch=`{}`; dirty=`{}`; snapshot=`{}`; schema=`structural:5`; repo_footprint=`zero`",
                root.to_string_lossy(),
                head,
                branch,
                dirty,
                snapshot
            )
        }
    };
    let _ = MAP_SNAPSHOT.set(line);
}

fn map_snapshot_git_head(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--short=12", "--verify", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let head = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!head.is_empty()).then_some(head)
}

fn map_snapshot_git_branch(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!branch.is_empty()).then_some(branch)
}

fn map_snapshot_dirty_count(root: &Path) -> Option<usize> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).lines().count())
}

pub(crate) fn map_snapshot_line() {
    if let Some(snapshot) = MAP_SNAPSHOT.get() {
        println!("{snapshot}");
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
                vec!["Cache strategy".to_string(), report.cache_strategy.clone()],
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
