#[test]
fn directory_cone_hides_test_support_side_effects_by_default() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/tests_support.ts"),
        "export function writeFixture() {\n  fs.writeFile('/tmp/codemap-fixture', 'ok');\n}\n",
    );

    let directory = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cone", "packages/app/src"])
        .output()
        .expect("directory cone should run");
    assert!(
        directory.status.success(),
        "directory cone failed: {}",
        String::from_utf8_lossy(&directory.stderr)
    );
    let directory = String::from_utf8(directory.stdout).expect("markdown utf8");
    assert!(
        !directory.contains("storage_write"),
        "directory cone should not surface test/support storage writes as owner side effects: {directory}"
    );

    let exact = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cone", "packages/app/src/tests_support.ts"])
        .output()
        .expect("exact cone should run");
    assert!(
        exact.status.success(),
        "exact cone failed: {}",
        String::from_utf8_lossy(&exact.stderr)
    );
    let exact = String::from_utf8(exact.stdout).expect("markdown utf8");
    assert!(
        exact.contains("storage_write"),
        "exact test/support cone should still show its own side effects: {exact}"
    );
}
