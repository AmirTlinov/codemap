// Responsibility: render-changed-worktree
use crate::model::ChangedReport;
use crate::render::current_map_prelude;

pub(crate) fn changed_worktree_section(report: &ChangedReport, compact: bool) {
    println!("\n## Worktree\n");
    changed_session_lines(report, compact);
    if let Some(prelude) = current_map_prelude()
        && prelude.vcs.as_deref() == Some("git")
    {
        if compact {
            println!(
                "- selector: `{}`; selected files: `{}`; staged=`{}`; unstaged=`{}`; untracked=`{}`; conflicts=`{}`; session=`{}` [{}]; selection=`{}`",
                report.selector,
                report.total_changed_count,
                prelude.worktree.staged,
                prelude.worktree.unstaged,
                prelude.worktree.untracked,
                prelude.worktree.conflicted,
                report.session_snapshot.token,
                report.session_snapshot.freshness,
                report.selection.kind
            );
            return;
        }
        println!("- selector: `{}`", report.selector);
        println!("- selected files: `{}`", report.total_changed_count);
        println!("- staged: `{}`", prelude.worktree.staged);
        println!("- unstaged: `{}`", prelude.worktree.unstaged);
        println!("- untracked: `{}`", prelude.worktree.untracked);
        println!("- conflicted: `{}`", prelude.worktree.conflicted);
        println!("- renamed: `{}`", prelude.worktree.renamed);
        println!("- deleted: `{}`", prelude.worktree.deleted);
        println!("- typechanged: `{}`", prelude.worktree.typechanged);
    } else {
        let staged = report
            .git_state
            .iter()
            .filter(|change| change.staged)
            .count();
        let unstaged = report
            .git_state
            .iter()
            .filter(|change| change.unstaged)
            .count();
        let untracked = report
            .git_state
            .iter()
            .filter(|change| change.status == "untracked")
            .count();
        let conflicted = report
            .git_state
            .iter()
            .filter(|change| change.status == "conflicted")
            .count();
        if compact {
            println!(
                "- selector: `{}`; selected files: `{}`; staged=`{staged}`; unstaged=`{unstaged}`; untracked=`{untracked}`; conflicts=`{conflicted}`; session=`{}` [{}]; selection=`{}`",
                report.selector,
                report.total_changed_count,
                report.session_snapshot.token,
                report.session_snapshot.freshness,
                report.selection.kind
            );
            return;
        }
        println!("- selector: `{}`", report.selector);
        println!("- selected files: `{}`", report.total_changed_count);
        println!("- staged: `{staged}`");
        println!("- unstaged: `{unstaged}`");
        println!("- untracked: `{untracked}`");
        println!("- conflicted: `{conflicted}`");
    }
    if report.selector != "--changed" {
        println!(
            "\nNote: Worktree counts show current repo state; selected files show `{}`.",
            report.selector
        );
    }
}

fn changed_session_lines(report: &ChangedReport, compact: bool) {
    let snapshot = &report.session_snapshot;
    let created = snapshot
        .created_unix_seconds
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unavailable".to_string());
    let selection = &report.selection;
    if compact {
        return;
    }
    println!(
        "- session snapshot: `{}` [freshness={}; created={created}; files={}; content={}; storage={}]",
        snapshot.token,
        snapshot.freshness,
        snapshot.file_count,
        snapshot.content_files,
        snapshot.storage
    );
    println!("  reuse: `{}`", snapshot.reuse);
    println!(
        "- selection: `{}` [resolved={}; selected={}; fallback={}; content_complete={}]",
        selection.kind,
        selection.resolved,
        selection.selected_files,
        selection.fallback_files,
        selection.content_complete
    );
    if let Some(baseline) = &selection.baseline_snapshot {
        println!(
            "  baseline snapshot: `{}` [created={}; files={}; freshness={}]",
            baseline.token,
            baseline
                .created_unix_seconds
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unavailable".to_string()),
            baseline.file_count,
            baseline.freshness
        );
    }
}
