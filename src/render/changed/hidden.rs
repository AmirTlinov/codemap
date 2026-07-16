// Responsibility: render-changed-hidden
use crate::model::ChangedReport;
use crate::render::{changed_selector_suffix, hidden_section, root_aware_expand};

pub(crate) fn changed_hidden_section(
    report: &ChangedReport,
    hidden: &[crate::model::HiddenGroup],
    force: bool,
    compact: bool,
) {
    if hidden.is_empty() {
        if force {
            println!("\n## Hidden\n");
            println!("No hidden material.");
        }
        return;
    }
    if compact && !force {
        println!("\n## Hidden\n");
        let reasons = hidden
            .iter()
            .take(3)
            .map(|group| group.reason.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        let suffix = if hidden.len() > 3 {
            format!("; +{} more", hidden.len() - 3)
        } else {
            String::new()
        };
        println!(
            "- hidden groups: `{}`; reasons: {reasons}{suffix}",
            hidden.len()
        );
        println!(
            "  expand: `{}`",
            root_aware_expand(&format!(
                "codemap changed{} --section hidden",
                changed_selector_suffix(&report.selector)
            ))
        );
        return;
    }
    hidden_section(hidden);
}
