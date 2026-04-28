use serde::Serialize;

use crate::model::{
    BoundaryFinding, ExplainReport, GraphLens, ImpactReport, LocateReport, TaskCapsule,
    VerificationPlan, WidenReport,
};
use crate::route::StatusReport;

pub fn print_json<T: Serialize>(value: &T) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

pub fn status(report: &StatusReport, doctor: bool) {
    println!(
        "{}",
        if doctor {
            "# ctx doctor"
        } else {
            "# ctx status"
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
                vec!["Fingerprint".to_string(), code(&report.fingerprint)],
                vec![
                    "Boundary findings".to_string(),
                    report.boundary_findings.to_string()
                ],
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
    if !report.config_errors.is_empty() {
        println!("\n## Anchor Config Errors\n");
        println!("{}", bullet(&report.config_errors, false, Some(10)));
    }
    if !report.scripts.is_empty() {
        println!("\n## Verification Hints\n");
        println!("{}", bullet(&report.scripts, true, Some(10)));
    }
    if report.unclassified_count > 0 {
        println!(
            "\n## Unclassified Source Files ({})\n",
            report.unclassified_count
        );
        println!(
            "{}",
            bullet(&report.unclassified_source_files, true, Some(30))
        );
    }
}

pub fn locate(report: &LocateReport) {
    println!("# Location Candidates\n");
    println!("Task: `{}`\n", report.task);
    let rows = report
        .candidates
        .iter()
        .map(|c| {
            vec![
                c.domain.id.clone(),
                code(&c.domain.path),
                c.task_kind.clone(),
                c.confidence.clone(),
                c.reasons.join("; "),
            ]
        })
        .collect();
    println!(
        "{}",
        table(&["Domain", "Path", "Task kind", "Confidence", "Why"], rows)
    );
    if let Some(best) = report.candidates.first() {
        println!("\n## Start\n");
        println!(
            "{}",
            code_block("bash", std::slice::from_ref(&best.start_command))
        );
    }
}

pub fn start(capsule: &TaskCapsule) {
    println!("# Task Context Capsule\n");
    println!("## Task\n\n`{}`\n", capsule.task);
    println!("## Classification\n");
    println!(
        "{}",
        table(
            &["Field", "Value"],
            vec![
                vec![
                    "Domain".to_string(),
                    format!("{} ({})", capsule.domain.id, code(&capsule.domain.path)),
                ],
                vec!["Task kind".to_string(), capsule.task_kind.clone()],
                vec!["Confidence".to_string(), capsule.confidence.clone()],
                vec!["Risk".to_string(), capsule.risk.clone()],
                vec!["Fingerprint".to_string(), code(&capsule.cache.fingerprint)],
            ],
        )
    );
    println!("\n## Read first\n");
    if capsule.read_first.is_empty() {
        println!("- no confident starting files found");
    } else {
        for (idx, item) in capsule.read_first.iter().enumerate() {
            let why = if item.reasons.is_empty() {
                String::new()
            } else {
                format!(" - {}", item.reasons.join("; "))
            };
            println!("{}. `{}`{}", idx + 1, item.path, why);
        }
    }
    if !capsule.related_tests.is_empty() {
        println!("\n## Related tests\n");
        println!("{}", bullet(&capsule.related_tests, true, Some(10)));
    }
    if !capsule.do_not_read_yet.is_empty() {
        println!("\n## Do not read yet\n");
        let rows = capsule
            .do_not_read_yet
            .iter()
            .map(|x| vec![code(&x.path), x.reason.clone()])
            .collect();
        println!("{}", table(&["Path", "Reason"], rows));
    }
    if !capsule.source_of_truth.is_empty() {
        println!("\n## Source of truth\n");
        println!("{}", bullet(&capsule.source_of_truth, true, Some(8)));
    }
    if !capsule.invariants.is_empty() {
        println!("\n## Likely invariants\n");
        println!("{}", bullet(&capsule.invariants, false, Some(7)));
    }
    if !capsule.forbidden_moves.is_empty() {
        println!("\n## Forbidden moves\n");
        println!("{}", bullet(&capsule.forbidden_moves, false, Some(7)));
    }
    println!("\n## Minimal verification\n");
    println!("{}", code_block("bash", &capsule.verification.minimal));
    if !capsule.verification.recommended.is_empty() {
        println!("\n## Recommended if risk grows\n");
        println!("{}", code_block("bash", &capsule.verification.recommended));
    }
    println!("\n## Expand only if\n");
    println!("{}", bullet(&capsule.expansion_triggers, false, Some(8)));
    println!("\n## Stop condition\n");
    println!("{}", bullet(&capsule.stop_conditions, false, Some(8)));
}

pub fn impact(report: &ImpactReport) {
    println!("# Impact Report\n");
    if report.changed.is_empty() {
        println!(
            "No changed files detected. Use `--files a,b` or run inside a git repo with changes."
        );
        return;
    }
    println!("## Changed files\n");
    let rows = report
        .files
        .iter()
        .map(|f| vec![code(&f.path), f.risk.clone(), f.reasons.join("; ")])
        .collect();
    println!("{}", table(&["File", "Risk", "Why"], rows));
    println!("\nOverall risk: **{}**", report.risk);
    if !report.impacted.is_empty() {
        println!("\n## Also inspect if needed\n");
        println!("{}", bullet(&report.impacted, true, Some(20)));
    }
    if !report.related_tests.is_empty() {
        println!("\n## Related tests\n");
        println!("{}", bullet(&report.related_tests, true, Some(10)));
    }
    if !report.external_domains.is_empty() {
        println!("\n## Context expansion required\n");
        for domain in &report.external_domains {
            println!("- `{}` ({})", domain.path, domain.id);
        }
    }
    if !report.expansion_triggers.is_empty() {
        println!("\n## Expansion triggers fired\n");
        println!("{}", bullet(&report.expansion_triggers, false, Some(8)));
    }
    println!("\n## Minimal verification\n");
    println!("{}", code_block("bash", &report.minimal_verification));
    if !report.recommended_verification.is_empty() {
        println!("\n## Recommended\n");
        println!("{}", code_block("bash", &report.recommended_verification));
    }
    if !report.full_verification.is_empty() {
        println!("\n## Full only if trigger remains unresolved\n");
        println!("{}", code_block("bash", &report.full_verification));
    }
}

pub fn verify(changed: &[String], plan: &VerificationPlan) {
    println!("# Verification Plan\n");
    if !changed.is_empty() {
        println!("Changed files:");
        println!("{}", bullet(changed, true, Some(20)));
        println!();
    }
    println!("## Minimal\n");
    println!("{}", code_block("bash", &plan.minimal));
    if !plan.recommended.is_empty() {
        println!("\n## Recommended\n");
        println!("{}", code_block("bash", &plan.recommended));
    }
    if !plan.full_only_if_triggered.is_empty() {
        println!("\n## Full only if needed\n");
        println!("{}", code_block("bash", &plan.full_only_if_triggered));
    }
    println!("\n`ctx verify` does not run commands unless `--run` is explicit.");
}

pub fn explain(report: &ExplainReport) {
    if report.kind == "missing" {
        println!(
            "No file or concept found for `{}`.",
            report.target.as_deref().unwrap_or("")
        );
        return;
    }
    if report.kind == "concept" {
        println!("# Concept: {}\n", report.id.as_deref().unwrap_or(""));
        println!(
            "{}",
            table(
                &["Field", "Value"],
                vec![
                    vec![
                        "Domain".to_string(),
                        report
                            .domain
                            .as_ref()
                            .map(|d| format!("{} ({})", d.id, code(&d.path)))
                            .unwrap_or_else(|| "unknown".to_string()),
                    ],
                    vec!["Roles".to_string(), report.roles.join(", ")],
                    vec![
                        "Files".to_string(),
                        report
                            .files
                            .iter()
                            .map(|f| code(f))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ],
                    vec!["Confidence".to_string(), report.confidence.clone()],
                    vec!["Provenance".to_string(), report.provenance.clone()],
                ],
            )
        );
        if !report.invariants.is_empty() {
            println!("\n## Invariants\n");
            println!("{}", bullet(&report.invariants, false, Some(10)));
        }
        return;
    }
    println!("# File: {}\n", report.path.as_deref().unwrap_or(""));
    println!(
        "{}",
        table(
            &["Field", "Value"],
            vec![
                vec![
                    "Domain".to_string(),
                    report
                        .domain
                        .as_ref()
                        .map(|d| format!("{} ({})", d.id, code(&d.path)))
                        .unwrap_or_else(|| "unknown".to_string()),
                ],
                vec![
                    "Roles".to_string(),
                    if report.roles.is_empty() {
                        "unclassified".to_string()
                    } else {
                        report.roles.join(", ")
                    }
                ],
                vec![
                    "Risk".to_string(),
                    report.risk.clone().unwrap_or_else(|| "unknown".to_string())
                ],
                vec!["Confidence".to_string(), report.confidence.clone()],
                vec!["Provenance".to_string(), report.provenance.clone()],
                vec!["Risk reasons".to_string(), report.risk_reasons.join("; ")],
            ],
        )
    );
    section("Exports", &report.exports);
    section("Imports", &report.imports);
    section("Imported by", &report.imported_by);
    section("Related tests", &report.related_tests);
}

pub fn widen(report: &WidenReport) {
    println!("# Widened Context\n");
    println!("Reason: {}\n", report.reason);
    println!(
        "{}",
        table(
            &["Field", "Value"],
            vec![
                vec![
                    "Domain".to_string(),
                    format!("{} ({})", report.domain.id, code(&report.domain.path)),
                ],
                vec!["Confidence".to_string(), report.confidence.clone()],
            ],
        )
    );
    println!("\n## Add\n");
    println!("{}", bullet(&report.add, true, Some(20)));
    if !report.still_do_not_read_yet.is_empty() {
        println!("\n## Still do not read yet\n");
        let rows = report
            .still_do_not_read_yet
            .iter()
            .map(|x| vec![code(&x.path), x.reason.clone()])
            .collect();
        println!("{}", table(&["Path", "Reason"], rows));
    }
    println!("\n## Stop rule\n");
    println!("{}", report.stop_rule);
}

pub fn boundaries(findings: &[BoundaryFinding]) {
    println!("# Boundary Check\n");
    if findings.is_empty() {
        println!("No boundary findings.");
        return;
    }
    let rows = findings
        .iter()
        .map(|f| {
            vec![
                f.status.clone(),
                code(&f.from),
                code(&f.to),
                f.confidence.clone(),
                f.reason.clone(),
            ]
        })
        .collect();
    println!(
        "{}",
        table(&["Status", "From", "To", "Confidence", "Reason"], rows)
    );
    let recoveries: Vec<_> = findings.iter().filter(|f| !f.recovery.is_empty()).collect();
    if !recoveries.is_empty() {
        println!("\n## Recovery paths");
        for finding in recoveries {
            println!("\n### `{}` -> `{}`\n", finding.from, finding.to);
            println!("{}", bullet(&finding.recovery, false, None));
        }
    }
}

pub fn graph_markdown(graph: &GraphLens) {
    println!("# Graph Lens: {}\n", graph.lens);
    println!("Domain: `{}` (`{}`)\n", graph.domain.id, graph.domain.path);
    println!("## Nodes\n");
    println!("{}", bullet(&graph.nodes, true, Some(30)));
    println!("\n## Edges\n");
    if graph.edges.is_empty() {
        println!("- none");
    } else {
        let rows = graph
            .edges
            .iter()
            .map(|e| vec![code(&e.from), e.edge_type.clone(), code(&e.to)])
            .collect();
        println!("{}", table(&["From", "Type", "To"], rows));
    }
}

pub fn graph_mermaid(graph: &GraphLens) {
    println!("graph TD");
    if graph.nodes.is_empty() && graph.edges.is_empty() {
        println!("  Empty[\"No graph data for lens\"]");
        return;
    }
    for node in &graph.nodes {
        println!("  {}[\"{}\"]", mermaid_id(node), escape_mermaid(node));
    }
    for edge in &graph.edges {
        println!(
            "  {} -->|{}| {}",
            mermaid_id(&edge.from),
            escape_mermaid(&edge.edge_type),
            mermaid_id(&edge.to)
        );
    }
}

pub fn init_suggestion(path: Option<&str>) {
    println!("{}", suggested_ctx_yml_for(path));
}

pub fn agents_bootloader() -> &'static str {
    "# Agent Bootstrap\n\nFor coding tasks in this repository, start with:\n\n```bash\nctx start --task \"<user task>\" --path \"$PWD\"\n```\n\nAfter edits:\n\n```bash\nctx impact --changed\nctx verify --changed\n```\n\nDo not manually scan the repository before using `ctx`.\nFollow ctx read order, forbidden moves, expansion triggers, verification plan, and stop conditions.\n"
}

pub fn global_instruction() -> &'static str {
    "For coding tasks, if `ctx` is available in PATH, begin with:\n\n```bash\nctx start --task \"<user task>\" --path \"$PWD\"\n```\n\nAfter edits:\n\n```bash\nctx impact --changed\nctx verify --changed\n```\n\nDo not manually scan the repository before using `ctx` unless ctx confidence is low or an expansion trigger fires.\nFollow the returned read order, negative context, forbidden moves, expansion triggers, verification plan, and stop conditions.\nWiden context only with `ctx widen` or when an expansion trigger fires.\n"
}

pub fn suggested_ctx_yml_for(path: Option<&str>) -> String {
    let domain_id = path
        .and_then(|p| p.trim_end_matches('/').rsplit('/').next())
        .filter(|p| !p.is_empty() && *p != ".")
        .unwrap_or("repo");
    format!(
        "version: 1\n\ndomain:\n  id: {domain_id}\n\nboundaries:\n  forbidden: []\n\ntask_routes: {{}}\n\nverification:\n  default: []\n"
    )
}

fn section(title: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    println!("\n## {title}\n");
    println!("{}", bullet(values, true, Some(20)));
}

fn table(headers: &[&str], rows: Vec<Vec<String>>) -> String {
    let mut out = Vec::new();
    out.push(format!("| {} |", headers.join(" | ")));
    out.push(format!("|{}|", vec!["---"; headers.len()].join("|")));
    for row in rows {
        out.push(format!(
            "| {} |",
            row.into_iter()
                .map(|cell| cell.replace('\n', "<br>"))
                .collect::<Vec<_>>()
                .join(" | ")
        ));
    }
    out.join("\n")
}

fn bullet(values: &[String], code_style: bool, limit: Option<usize>) -> String {
    let mut items: Vec<String> = values.to_vec();
    if let Some(limit) = limit
        && items.len() > limit
    {
        let extra = items.len() - limit;
        items.truncate(limit);
        items.push(format!("... +{extra} more"));
    }
    if items.is_empty() {
        return "- none".to_string();
    }
    items
        .into_iter()
        .map(|item| {
            if code_style {
                format!("- `{item}`")
            } else {
                format!("- {item}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn code(value: &str) -> String {
    format!("`{value}`")
}

fn code_block(lang: &str, commands: &[String]) -> String {
    if commands.is_empty() {
        return format!("```{lang}\n# no command inferred\n```");
    }
    format!("```{lang}\n{}\n```", commands.join("\n"))
}

fn mermaid_id(value: &str) -> String {
    let body: String = value
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("n_{body}")
}

fn escape_mermaid(value: &str) -> String {
    value.replace('"', "'").replace('\n', " ")
}
