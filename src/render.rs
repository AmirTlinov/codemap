use std::path::PathBuf;

use serde::Serialize;

use crate::repo::RepoStatus;

pub fn doctor(status: &RepoStatus) {
    println!("# ctx doctor");
    println!();
    println!("Status: ok");
    println!("Zero-footprint default: {}", status.zero_footprint_default);
    println!("Cache root: `{}`", status.cache_dir.display());
    match &status.repo_root {
        Some(root) => println!("Repo root: `{}`", root.display()),
        None => println!("Repo root: not detected"),
    }
}

pub fn status(status: &RepoStatus) {
    println!("# ctx status");
    println!();
    println!("| Field | Value |");
    println!("|---|---|");
    println!("| CWD | `{}` |", status.cwd.display());
    println!(
        "| Repo root | {} |",
        status
            .repo_root
            .as_ref()
            .map(|root| format!("`{}`", root.display()))
            .unwrap_or_else(|| "not detected".to_string())
    );
    println!("| Cache | `{}` |", status.cache_dir.display());
    println!("| Writes project by default | no |");
}

pub fn init_suggestion(agents: bool) {
    if agents {
        println!("# Suggested AGENTS.md bootloader");
        println!();
        println!("```md");
        println!("# Agent Bootstrap");
        println!();
        println!("For coding tasks, if `ctx` is available, start with:");
        println!();
        println!("`ctx start --task \"<user task>\" --path \"$PWD\"`");
        println!();
        println!("After edits:");
        println!();
        println!("`ctx impact --changed`");
        println!("`ctx verify --changed`");
        println!();
        println!(
            "Follow ctx read order, negative context, expansion triggers, verification plan, and stop rules."
        );
        println!("```");
    } else {
        println!("# Suggested .ctx.yml");
        println!();
        println!("```yaml");
        println!("version: 1");
        println!("concepts: {{}}");
        println!("boundaries:");
        println!("  forbidden: []");
        println!("```");
    }
}

pub fn planned(command: &str, detail: &str) {
    println!("# ctx {}", command);
    println!();
    println!("Status: planned");
    println!();
    println!("{}", detail);
}

#[derive(Debug, Serialize)]
pub struct StubCapsule {
    kind: &'static str,
    task: String,
    path: Option<PathBuf>,
    confidence: &'static str,
    status: &'static str,
    read_first: Vec<String>,
    do_not_read_yet: Vec<String>,
    verification: Vec<String>,
}

pub fn stub_capsule_json(task: &str, path: Option<&PathBuf>) -> StubCapsule {
    StubCapsule {
        kind: "task_context_capsule",
        task: task.to_string(),
        path: path.cloned(),
        confidence: "low",
        status: "routing_engine_not_implemented_yet",
        read_first: path
            .map(|p| vec![p.display().to_string()])
            .unwrap_or_default(),
        do_not_read_yet: vec!["unbounded repository scan".to_string()],
        verification: vec!["ctx verify --changed".to_string()],
    }
}

pub fn stub_capsule_markdown(task: &str, path: Option<&PathBuf>) {
    println!("# Task Context Capsule");
    println!();
    println!("## Task");
    println!();
    println!("`{}`", task);
    println!();
    println!("## Confidence");
    println!();
    println!("Low");
    println!();
    println!(
        "Reason: routing engine is not implemented yet. This repository is prepared for implementation."
    );
    println!();
    println!("## Read First");
    println!();
    match path {
        Some(path) => println!("1. `{}`", path.display()),
        None => println!("No route yet. Implement repo discovery and task scoring first."),
    }
    println!();
    println!("## Do Not Inspect Yet");
    println!();
    println!("- unbounded repository scan");
    println!();
    println!("## Verification");
    println!();
    println!("```bash");
    println!("ctx verify --changed");
    println!("```");
    println!();
    println!("## Stop Condition");
    println!();
    println!(
        "Stop after implementing the routing slice and replacing this stub output with real facts."
    );
}
