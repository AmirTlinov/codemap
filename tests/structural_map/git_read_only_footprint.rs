#[cfg(unix)]
#[derive(Debug, PartialEq, Eq)]
struct GitIndexSnapshot {
    sha256: String,
    len: u64,
    mtime: (i64, i64),
    ctime: (i64, i64),
}

#[cfg(unix)]
fn git_index_snapshot(path: &Path) -> GitIndexSnapshot {
    use sha2::{Digest, Sha256};
    use std::os::unix::fs::MetadataExt;

    let bytes = fs::read(path).expect("read git index");
    let metadata = fs::metadata(path).expect("stat git index");
    GitIndexSnapshot {
        sha256: format!("{:x}", Sha256::digest(bytes)),
        len: metadata.len(),
        mtime: (metadata.mtime(), metadata.mtime_nsec()),
        ctime: (metadata.ctime(), metadata.ctime_nsec()),
    }
}

#[cfg(unix)]
fn assert_git_index_unchanged(
    repo: &Path,
    cache: &Path,
    args: &[&str],
    expected: &GitIndexSnapshot,
) {
    let output = codemap()
        .current_dir(repo)
        .env("CODEMAP_CACHE_DIR", cache)
        .args(args)
        .output()
        .expect("codemap read-only probe should run");
    assert!(
        output.status.success(),
        "codemap {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        git_index_snapshot(&repo.join(".git/index")),
        *expected,
        "codemap {args:?} must not refresh or rewrite .git/index"
    );
}

#[cfg(unix)]
#[test]
fn where_and_cone_keep_git_index_byte_and_metadata_stable_cold_and_warm() {
    let repo = TempDir::new().expect("read-only git repo");
    let where_cache = TempDir::new().expect("where cache");
    let cone_cache = TempDir::new().expect("cone cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"git-read-only-fixture","private":true}"#,
    );
    let source = "export function target() { return 1; }\n";
    write(&repo.path().join("src/a.ts"), source);
    write(
        &repo.path().join("src/use.ts"),
        "import { target } from './a';\nexport const value = target();\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    // Make Git's cached stat data stale without changing the tracked content.
    // A normal `git status` is then allowed to refresh the index; codemap is not.
    std::thread::sleep(std::time::Duration::from_millis(10));
    fs::write(repo.path().join("src/a.ts"), source).expect("refresh source metadata");
    let expected = git_index_snapshot(&repo.path().join(".git/index"));

    for args in [&["where", "target"][..], &["where", "target"][..]] {
        assert_git_index_unchanged(repo.path(), where_cache.path(), args, &expected);
    }
    for args in [
        &["cone", "src/a.ts#target"][..],
        &["cone", "src/a.ts#target"][..],
    ] {
        assert_git_index_unchanged(repo.path(), cone_cache.path(), args, &expected);
    }
}
