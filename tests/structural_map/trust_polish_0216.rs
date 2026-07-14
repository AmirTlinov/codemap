#[test]
fn proof_changed_all_does_not_poison_default_lens_cache() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"proof-cache-fixture","private":true,"scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join("src/core.ts"),
        "export function coreValue() { return 1; }\n",
    );
    for index in 0..25 {
        write(
            &repo.path().join(format!("tests/core-{index}.test.ts")),
            "import { coreValue } from '../src/core';\ntest('core', () => expect(coreValue()).toBe(1));\n",
        );
    }
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "proof cache fixture"]);
    write(
        &repo.path().join("src/core.ts"),
        "export function coreValue() { return 2; }\n",
    );

    let default_before = run_json(repo.path(), cache.path(), &["proof", "changed", "--format", "json"]);
    assert_schema("schemas/proof.schema.json", &default_before);
    let default_visible = default_before["proofs"].as_array().expect("proofs").len();
    assert!(
        default_visible < 25
            && !default_before["hidden"]
                .as_array()
                .expect("hidden")
                .is_empty(),
        "default proof changed should stay bounded before --all: {default_before:#}"
    );

    let all = run_json(repo.path(), cache.path(), &["proof", "changed", "--all", "--format", "json"]);
    assert_schema("schemas/proof.schema.json", &all);
    assert!(
        all["proofs"].as_array().expect("all proofs").len() > default_visible,
        "--all should expand the current report: {all:#}"
    );

    let default_after = run_json(repo.path(), cache.path(), &["proof", "changed", "--format", "json"]);
    assert_schema("schemas/proof.schema.json", &default_after);
    assert_eq!(
        default_after["proofs"].as_array().expect("after proofs").len(),
        default_visible,
        "--all output must not overwrite the bounded default proof-changed lens cache: {default_after:#}"
    );
    assert!(
        !default_after["hidden"]
            .as_array()
            .expect("after hidden")
            .is_empty(),
        "bounded proof changed should still expose hidden material after --all: {default_after:#}"
    );
}

#[test]
fn schema_proof_unknowns_are_schema_specific_not_source_direct_test() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"schema-proof-fixture","private":true,"scripts":{"db:migrate:status":"prisma migrate status"}}"#,
    );
    write(
        &repo.path().join("prisma/schema.prisma"),
        r#"generator client {
  provider = "prisma-client-js"
}

datasource db {
  provider = "postgresql"
  url      = env("DATABASE_URL")
}

model User {
  id String @id
}
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "schema proof fixture"]);

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", "prisma/schema.prisma", "--section", "unknown"])
        .output()
        .expect("schema proof unknown should run");
    assert!(
        output.status.success(),
        "schema proof unknown failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("schema_migration_not_found")
            && markdown.contains("schema_client_consumer_not_found")
            && !markdown.contains("direct_test_import_not_found"),
        "schema proof unknowns should describe schema map gaps, not source test-import gaps: {markdown}"
    );
}

#[test]
fn proof_runner_cone_shows_soft_neighbor_rails_without_calling_them_proof() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Makefile"),
        "validate-growth:\n\tpython tools/run_growth_episode.py --receipt experiments/receipts/growth.json && python tools/doctor.py\n",
    );
    write(
        &repo.path().join("tools/run_growth_episode.py"),
        "def run_growth_episode():\n    return {'growth': 'carrier'}\n",
    );
    write(
        &repo.path().join("tools/doctor.py"),
        "def check_growth_receipt():\n    return True\n",
    );
    write(
        &repo.path().join("experiments/receipts/growth.json"),
        r#"{"kind":"growth_receipt","carrier":"ok"}"#,
    );
    write(
        &repo.path().join("docs/experiments/growth.md"),
        "# Growth\n\nRunner: tools/run_growth_episode.py\nReceipt: experiments/receipts/growth.json\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "proof runner fixture"]);

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cone", "tools/run_growth_episode.py", "--depth", "1"])
        .output()
        .expect("proof runner cone should run");
    assert!(
        output.status.success(),
        "proof runner cone failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("## Soft Surface Matches")
            && markdown.contains("experiments/receipts/growth.json")
            && markdown.contains("command:make validate-growth")
            && !markdown.contains("\n## Verification Surfaces\n"),
        "proof runner neighbor rails should render as soft surface matches, not direct runnable surfaces: {markdown}"
    );
}

#[test]
fn cone_depth_edges_are_marked_mediated_after_the_anchor_layer() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(&repo.path().join("src/anchor.ts"), "import './middle';\n");
    write(&repo.path().join("src/middle.ts"), "import './leaf';\n");
    write(&repo.path().join("src/leaf.ts"), "export const leaf = 1;\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "cone depth fixture"]);

    let cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/anchor.ts", "--depth", "2", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    let outgoing = cone["outgoing"].as_array().expect("outgoing");
    assert!(
        outgoing.iter().any(|edge| edge["from"] == "src/anchor.ts"
            && edge["to"] == "src/middle.ts"
            && edge["evidence"] == "resolved_import"),
        "anchor layer import should stay direct: {cone:#}"
    );
    assert!(
        outgoing.iter().any(|edge| edge["from"] == "src/middle.ts"
            && edge["to"] == "src/leaf.ts"
            && edge["evidence"] == "resolved_import_via_cone_depth"
            && edge["strength"] == "medium"),
        "deeper cone imports should be labeled as mediated by cone depth: {cone:#}"
    );
}

#[test]
fn compact_unknown_samples_use_single_code_spans() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    let mut files = Vec::new();
    for index in 0..7 {
        let rel = format!("src/file-{index}.ts");
        files.push(rel.clone());
        write(
            &repo.path().join(&rel),
            &format!("export const value{index} = {index};\n"),
        );
    }
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "unknown compact fixture"]);

    let joined = files.join(",");
    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--files", &joined, "--section", "unknown"])
        .output()
        .expect("changed unknown should run");
    assert!(
        output.status.success(),
        "changed unknown failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("sample: `src/file-0.ts`") && !markdown.contains("``src/file-0.ts``"),
        "compact unknown samples should not double-wrap code spans: {markdown}"
    );

    let expanded = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--files", &joined, "--section", "unknown", "--all"])
        .output()
        .expect("changed unknown --all should run");
    assert!(
        expanded.status.success(),
        "changed unknown --all failed: {}",
        String::from_utf8_lossy(&expanded.stderr)
    );
    let expanded = String::from_utf8(expanded.stdout).expect("expanded markdown utf8");
    assert!(
        !expanded.contains("hidden:"),
        "changed --section unknown --all must not collapse fail-open Unknown rows: {expanded}"
    );
}

#[test]
fn support_artifact_json_surface_hints_do_not_fall_back_to_config() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    let rel = "artifacts/147/foo-proof/proof.json";
    write(
        &repo.path().join(rel),
        r#"{"status":"pass","receipt":"artifacts/147/foo-proof/receipt.json"}"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "support artifact fixture"]);
    write(
        &repo.path().join(rel),
        r#"{"status":"pass","receipt":"artifacts/147/foo-proof/receipt.json","review":"ok"}"#,
    );

    let cone = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cone", rel, "--section", "roles"])
        .output()
        .expect("artifact cone roles should run");
    assert!(
        cone.status.success(),
        "artifact cone roles failed: {}",
        String::from_utf8_lossy(&cone.stderr)
    );
    let cone = String::from_utf8(cone.stdout).expect("cone markdown utf8");
    assert!(
        cone.contains("- `witness`") && !cone.contains("- `config`"),
        "support artifact JSON should render as witness, not config surface hint: {cone}"
    );

    let changed = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--section", "roles"])
        .output()
        .expect("artifact changed roles should run");
    assert!(
        changed.status.success(),
        "artifact changed roles failed: {}",
        String::from_utf8_lossy(&changed.stderr)
    );
    let changed = String::from_utf8(changed.stdout).expect("changed markdown utf8");
    assert!(
        changed.contains("- `witness`: `1`") && !changed.contains("- `config`:"),
        "changed support artifact JSON should not be grouped as config: {changed}"
    );
}
