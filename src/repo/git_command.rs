// Responsibility: repository-read-only-git-command
use std::process::Command;

/// Builds a Git command that is guaranteed not to refresh the target repository index.
///
/// Git treats index refreshes performed by otherwise read-only commands such as
/// `status` and `diff` as optional writes. Disabling optional locks therefore keeps
/// codemap's probes observational while preserving ordinary Git error behavior.
pub(crate) fn read_only_git_command() -> Command {
    let mut command = Command::new("git");
    command.env("GIT_OPTIONAL_LOCKS", "0");
    command
}
