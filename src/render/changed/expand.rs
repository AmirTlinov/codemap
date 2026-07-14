// Responsibility: render-changed-expand
use crate::model::ChangedReport;
use crate::render::root_aware_expand;

pub(crate) fn changed_compact_expand_section(report: &ChangedReport) {
    if report.expand.is_empty() {
        return;
    }
    println!("\n## Expand\n");
    let section_commands = report
        .expand
        .iter()
        .filter(|command| changed_expand_is_section(command))
        .cloned()
        .collect::<Vec<_>>();
    let lens_commands = report
        .expand
        .iter()
        .filter(|command| !changed_expand_is_section(command))
        .cloned()
        .collect::<Vec<_>>();
    if !section_commands.is_empty() {
        println!(
            "- sections: {}",
            section_commands
                .iter()
                .map(|command| format!("`{}`", root_aware_expand(command)))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if !lens_commands.is_empty() {
        println!(
            "- lenses: {}",
            lens_commands
                .iter()
                .map(|command| format!("`{}`", root_aware_expand(command)))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
}

fn changed_expand_is_section(command: &str) -> bool {
    command
        .strip_prefix("codemap changed")
        .is_some_and(|tail| tail.split_whitespace().any(|part| part == "--section"))
}
