#[test]
fn cone_xray_card_maps_anchor_without_recommendations() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"xray-fixture","private":true,"scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join("src/feature/run.ts"),
        r#"import fs from "node:fs";
import { validateFeature } from "./validate";

function _digest(value: string) {
  return value.trim();
}

export function runFeature(path: string, value: string) {
  validateFeature(value);
  fs.writeFileSync(path, _digest(value));
}
"#,
    );
    write(
        &repo.path().join("src/feature/validate.ts"),
        "export function validateFeature(value: string) {\n  if (!value) throw new Error('missing');\n}\n",
    );
    write(
        &repo.path().join("tests/run.test.ts"),
        "import { runFeature } from '../src/feature/run';\n\ntest('run feature', () => {\n  expect(runFeature).toBeDefined();\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "xray fixture"]);

    let cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/feature/run.ts", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    let xray = &cone["xray"];
    assert!(
        xray["roles"].as_array().expect("roles").iter().any(|role| {
            role["kind"] == "source" || role["kind"] == "domain" || role["kind"] == "adapter"
        }),
        "x-ray should expose a compact structural role surface: {cone:#}"
    );
    assert!(
        xray["outputs"]
            .as_array()
            .expect("outputs")
            .iter()
            .any(|surface| surface["kind"] == "public_export"
                && surface["path"] == "src/feature/run.ts#runFeature"),
        "x-ray should distinguish public exports as outputs: {cone:#}"
    );
    assert!(
        xray["outputs"]
            .as_array()
            .expect("outputs")
            .iter()
            .any(|surface| surface["kind"] == "defined_private_symbols"
                && surface["examples"]
                    .as_array()
                    .expect("examples")
                    .iter()
                    .any(|example| example == "_digest")),
        "x-ray should keep private helpers visible without calling them public exports: {cone:#}"
    );
    assert!(
        xray["side_effects"]
            .as_array()
            .expect("side_effects")
            .iter()
            .any(|surface| surface["kind"] == "storage_write"),
        "x-ray should expose observed side-effect surfaces: {cone:#}"
    );
    assert!(
        xray["nearby"]
            .as_array()
            .expect("nearby")
            .iter()
            .any(|surface| surface["kind"] == "validator"
                && surface["path"] == "src/feature/validate.ts"),
        "x-ray should show implemented nearby surfaces without ranking them: {cone:#}"
    );
    assert!(
        xray["proof_direct"]
            .as_array()
            .expect("proof_direct")
            .iter()
            .any(|edge| edge["evidence"] == "test_import"),
        "direct test imports should land in the direct proof bucket: {cone:#}"
    );

    let markdown = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cone", "src/feature/run.ts"])
        .output()
        .expect("cone markdown should run");
    assert!(
        markdown.status.success(),
        "cone markdown failed: {}",
        String::from_utf8_lossy(&markdown.stderr)
    );
    let text = String::from_utf8(markdown.stdout).expect("markdown utf8");
    assert!(
        text.contains("## X-Ray Card")
            && text.contains("branch=`")
            && text.contains("dirty=`")
            && text.contains("cache=`")
            && text.contains("repo_footprint=`zero`")
            && text.contains("Existing Nearby Surfaces")
            && text.contains("Proof Sensors:")
            && text.contains("[Direct]")
            && text.contains("[Soft]")
            && !text.contains("Recommended")
            && !text.contains("best file")
            && !text.contains("should edit"),
        "x-ray markdown should stay a source-backed map, not advice: {text}"
    );
}
