#[test]
fn proof_changed_renders_coverage_summary_and_gaps() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/direct-proof.ts"),
        "export function directProof() {\n  return true;\n}\n",
    );
    write(
        &repo.path().join("packages/replay/tests/direct-proof.test.ts"),
        "import { directProof } from '../src/direct-proof';\n\ntest('direct proof', () => {\n  expect(directProof()).toBe(true);\n});\n",
    );
    write(
        &repo.path().join("packages/replay/src/no-proof.ts"),
        "export const noProof = true;\n",
    );

    let proof = run_json(repo.path(), cache.path(), &["proof", "changed", "--format", "json"]);
    assert_schema("schemas/proof.schema.json", &proof);
    assert_eq!(proof["schema_version"], "8");
    let coverage = &proof["coverage"];
    assert_eq!(coverage["changed_count"].as_u64(), Some(3));
    assert!(
        coverage["runnable_deterministic"]
            .as_array()
            .expect("runnable deterministic")
            .iter()
            .any(|entry| entry["path"] == "packages/replay/src/direct-proof.ts"),
        "proof changed should mark direct test imports as runnable deterministic coverage: {proof:#}"
    );
    assert!(
        coverage["missing"]
            .as_array()
            .expect("missing")
            .iter()
            .any(|entry| entry["path"] == "packages/replay/src/no-proof.ts"
                && entry["kind"] == "direct_deterministic_proof_not_found"),
        "proof changed should keep missing direct proof gaps by changed path: {proof:#}"
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", "changed"])
        .output()
        .expect("proof changed markdown should run");
    assert!(
        output.status.success(),
        "proof changed failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("\n## Coverage\n")
            && markdown.contains("- runnable deterministic: `1`")
            && markdown.contains("- missing direct proof: `1`")
            && markdown.contains("packages/replay/src/direct-proof.ts")
            && markdown.contains("packages/replay/src/no-proof.ts")
            && markdown.contains(
                "codemap proof-map --files packages/replay/src/no-proof.ts --raw-sensors",
            )
            && !markdown.contains("recommended")
            && !markdown.contains("best file"),
        "proof changed should render coverage as facts, not advice: {markdown}"
    );
}

#[test]
fn compact_changed_staged_keeps_section_expands_grouped() {
    let (repo, cache) = fixture();
    write_many_staged_files(repo.path(), 25);

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--staged"])
        .output()
        .expect("changed --staged markdown should run");
    assert!(
        output.status.success(),
        "changed --staged failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("sections: `codemap changed --staged --section observed`")
            && markdown.contains("`codemap changed --staged --section proof`")
            && !markdown.contains("lenses: `codemap changed --staged --section observed`"),
        "compact changed --staged should keep section expands out of lenses: {markdown}"
    );
}

#[test]
fn proof_staged_large_compact_expands_staged_scope() {
    let (repo, cache) = fixture();
    write_many_staged_files(repo.path(), 25);

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", "--staged"])
        .output()
        .expect("proof --staged markdown should run");
    assert!(
        output.status.success(),
        "proof --staged failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("codemap changed --staged --section observed --limit 25")
            && markdown.contains("codemap proof-map --staged --raw-sensors --limit")
            && !markdown.contains("codemap changed --section observed --limit 25")
            && !markdown.contains("codemap proof-map --changed --raw-sensors"),
        "proof --staged should preserve staged scope in exact expand commands: {markdown}"
    );
}

fn write_many_staged_files(root: &std::path::Path, count: usize) {
    for index in 0..count {
        write(
            &root.join(format!("packages/replay/src/staged-{index}.ts")),
            &format!("export const staged{index} = {index};\n"),
        );
    }
    git(root, &["add", "packages/replay/src"]);
}
