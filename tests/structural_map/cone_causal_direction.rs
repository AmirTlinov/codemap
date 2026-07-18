// Responsibility: exact-file-cone-causal-direction
#[test]
fn file_cone_depth_follows_dependencies_without_recursing_through_consumers() {
    let repo = TempDir::new().expect("repo");
    let cache = TempDir::new().expect("cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("src/entry.ts"),
        "import { run } from './owner';\nexport const response = run();\n",
    );
    write(
        &repo.path().join("src/owner.ts"),
        "import { load } from './carrier';\nexport const run = () => load();\n",
    );
    write(
        &repo.path().join("src/carrier.ts"),
        "import { proof } from './proof';\nexport const load = () => proof;\n",
    );
    write(
        &repo.path().join("src/proof.ts"),
        "export const proof = 'observed';\n",
    );
    write(
        &repo.path().join("src/unrelated.ts"),
        "import { response } from './entry';\nimport { noise } from './noise';\nexport const view = response + noise;\n",
    );
    write(
        &repo.path().join("src/noise.ts"),
        "export const noise = 'noise';\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "directed cone fixture"]);

    let cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/entry.ts", "--depth", "3", "--format", "json"],
    );
    let outgoing = cone["outgoing"].as_array().expect("outgoing");
    assert!(
        outgoing
            .iter()
            .any(|edge| edge["from"] == "src/carrier.ts" && edge["to"] == "src/proof.ts"),
        "the dependency chain must reach its proof carrier: {cone:#}"
    );
    assert!(
        outgoing.iter().all(|edge| edge["from"] != "src/unrelated.ts"
            && edge["to"] != "src/noise.ts"),
        "a direct consumer belongs in incoming, not in recursive dependency traversal: {cone:#}"
    );
    assert!(
        cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .any(|edge| edge["from"] == "src/unrelated.ts" && edge["to"] == "src/entry.ts"),
        "the direct consumer must remain visible at the anchor boundary: {cone:#}"
    );
}
