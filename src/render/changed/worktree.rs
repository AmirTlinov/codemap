// Responsibility: render-changed-worktree
use crate::model::ChangedReport;
use crate::render::current_map_prelude;

pub(crate) fn changed_worktree_section(report: &ChangedReport, compact: bool) {
    println!("\n## Worktree\n");
    if let Some(prelude) = current_map_prelude()
        && prelude.vcs.as_deref() == Some("git")
    {
        if compact {
            println!(
                "- selector: `{}`; selected files: `{}`; staged=`{}`; unstaged=`{}`; untracked=`{}`; conflicts=`{}`",
                report.selector,
                report.total_changed_count,
                prelude.worktree.staged,
                prelude.worktree.unstaged,
                prelude.worktree.untracked,
                prelude.worktree.conflicted
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
                "- selector: `{}`; selected files: `{}`; staged=`{staged}`; unstaged=`{unstaged}`; untracked=`{untracked}`; conflicts=`{conflicted}`",
                report.selector, report.total_changed_count
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
