#[test]
fn snapshot_identity_preserves_rename_delete_typechange_and_conflict_provenance() {
    let (repo, cache) = fixture();
    for (path, body) in [
        ("rename-old.ts", "export const renamed = 1;\n"),
        ("delete-me.ts", "export const removed = 1;\n"),
        ("type-me.ts", "export const typed = 1;\n"),
        ("conflict.ts", "export const side = 'base';\n"),
    ] {
        write(&repo.path().join(path), body);
    }
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "snapshot identity base"]);
    let baseline = snapshot_json(repo.path(), cache.path(), &["changed", "--format", "json"]);
    let token = baseline["session_snapshot"]["token"]
        .as_str()
        .expect("snapshot token")
        .to_string();

    git(repo.path(), &["checkout", "-qb", "snapshot-conflict-side"]);
    write(
        &repo.path().join("conflict.ts"),
        "export const side = 'branch';\n",
    );
    git(repo.path(), &["commit", "-qam", "branch conflict side"]);
    git(repo.path(), &["checkout", "-q", "main"]);
    write(
        &repo.path().join("conflict.ts"),
        "export const side = 'main';\n",
    );
    git(repo.path(), &["commit", "-qam", "main conflict side"]);
    let merge = std::process::Command::new("git")
        .args(["merge", "snapshot-conflict-side"])
        .current_dir(repo.path())
        .output()
        .expect("merge conflict");
    assert!(!merge.status.success(), "fixture should produce a conflict");

    git(repo.path(), &["mv", "rename-old.ts", "rename-new.ts"]);
    std::fs::remove_file(repo.path().join("delete-me.ts")).expect("delete fixture");
    std::fs::remove_file(repo.path().join("type-me.ts")).expect("replace type fixture");
    std::os::unix::fs::symlink("rename-new.ts", repo.path().join("type-me.ts"))
        .expect("create symlink typechange");
    let delta = snapshot_json(
        repo.path(),
        cache.path(),
        &["changed", "--since", &token, "--format", "json"],
    );
    let states = delta["git_state"].as_array().expect("git state");
    let state = |path: &str| {
        states
            .iter()
            .find(|change| change["path"] == path)
            .unwrap_or_else(|| panic!("missing {path}: {states:#?}"))
    };
    assert_eq!(
        state("rename-new.ts")["status"],
        "renamed",
        "{states:#?}"
    );
    assert_eq!(state("rename-new.ts")["old_path"], "rename-old.ts");
    assert_eq!(state("rename-new.ts")["provenance"], "snapshot_content_identity");
    assert_eq!(state("delete-me.ts")["status"], "deleted");
    assert_eq!(state("type-me.ts")["status"], "typechanged");
    assert_eq!(state("conflict.ts")["status"], "conflicted");
    assert_eq!(state("conflict.ts")["provenance"], "git_status_conflict");
}
