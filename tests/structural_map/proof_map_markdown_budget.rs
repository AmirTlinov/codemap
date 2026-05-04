#[test]
fn proof_map_runnable_command_summary_stays_bounded() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "bounded-proof-map",
  "private": true,
  "workspaces": ["packages/*"],
  "scripts": {
    "build": "tsc -b",
    "build:all": "pnpm run build",
    "check": "tsc --noEmit",
    "e2e": "playwright test",
    "lint": "eslint .",
    "test": "vitest run",
    "test:fast": "vitest run src",
    "typecheck": "tsc -b",
    "type-check:api": "tsc --noEmit",
    "verify": "pnpm test",
    "verify:local": "pnpm run lint && pnpm test"
  }
}
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "bounded proof-map command summary fixture"]);

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof-map", "."])
        .output()
        .expect("proof-map markdown should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    let runnable = markdown
        .split("## Runnable Commands")
        .nth(1)
        .and_then(|section| section.split("\n## ").next())
        .expect("runnable commands section");
    let visible_commands = runnable
        .lines()
        .filter(|line| line.starts_with("- `"))
        .count();
    assert!(
        visible_commands <= 8,
        "proof-map runnable command summary should stay bounded; visible={visible_commands}: {markdown}"
    );
    assert!(
        runnable.contains("- hidden runnable commands: `"),
        "bounded proof-map command summary should preserve hidden count: {markdown}"
    );
}
