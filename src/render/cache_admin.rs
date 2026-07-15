// Responsibility: cache-diagnostic-readable-output
use crate::cache::CacheAdminReport;
use crate::render::{code, table};

pub fn cache_admin(report: &CacheAdminReport) {
    println!("# codemap cache {}\n", report.action);
    println!(
        "{}",
        table(
            &["Field", "Value"],
            vec![
                vec!["Root".to_string(), code(&report.root)],
                vec!["Cache".to_string(), code(&report.cache_dir)],
                vec![
                    "Outside repository".to_string(),
                    report.outside_repository.to_string(),
                ],
                vec!["Exists".to_string(), report.exists.to_string()],
                vec!["Files".to_string(), report.files.to_string()],
                vec!["Bytes".to_string(), report.bytes.to_string()],
                vec!["Snapshots".to_string(), report.snapshots.to_string()],
                vec![
                    "Quarantine receipts".to_string(),
                    report.quarantine_receipts.to_string(),
                ],
                vec![
                    "Private file permissions".to_string(),
                    report.private_file_permissions.to_string(),
                ],
                vec![
                    "Removed files".to_string(),
                    report.removed_files.to_string()
                ],
                vec![
                    "Removed bytes".to_string(),
                    report.removed_bytes.to_string()
                ],
            ],
        )
    );
    list("Contents", &report.contents);
    list("Retention", &report.retention);
    list("Privacy", &report.privacy);
    if !report.diagnostic_events.is_empty() {
        println!("\n## Diagnostics\n");
        for event in &report.diagnostic_events {
            println!(
                "- {} `{}` {} — {}",
                event.operation, event.artifact, event.outcome, event.detail
            );
        }
    }
}

fn list(title: &str, values: &[&str]) {
    println!("\n## {title}\n");
    for value in values {
        println!("- {value}");
    }
}
