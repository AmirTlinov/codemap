#[test]
fn graph_lens_reports_hidden_nodes_with_concrete_expand() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/multi-proof.ts"),
        "export function multiProof() {\n  return true;\n}\n",
    );
    for index in 1..=3 {
        write(
            &repo
                .path()
                .join(format!("packages/replay/tests/multi-proof-{index}.test.ts")),
            "import { multiProof } from '../src/multi-proof';\n\ntest('multi proof', () => {\n  expect(multiProof()).toBe(true);\n});\n",
        );
    }

    let graph = run_json(
        repo.path(),
        cache.path(),
        &[
            "graph",
            "--path",
            "packages/replay/src/multi-proof.ts",
            "--lens",
            "proof",
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/graph.schema.json", &graph);
    assert_eq!(graph["schema_version"], "4");
    assert_eq!(graph["nodes"].as_array().expect("nodes").len(), 1);
    assert!(
        graph["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|group| group["reason"] == "graph nodes hidden by limit"
                && group["count"] == 3
                && group["expand"]
                    == "codemap graph --lens proof --path packages/replay/src/multi-proof.ts --limit 4"),
        "graph should not silently drop nodes hidden by --limit: {graph:#}"
    );
}

#[test]
fn graph_hidden_expand_preserves_path_and_changed_selector() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/session.ts"),
        "import { Timeline } from './timeline';\nimport type { FrameDto } from './types';\n\nexport function seek(cursor: number): FrameDto {\n  return { frame: new Timeline().frameAt(cursor + 1) };\n}\n",
    );
    write(
        &repo.path().join("packages/replay/src/types.ts"),
        "export interface FrameDto {\n  frame: number;\n  source?: string;\n}\n",
    );

    let graph = run_json(
        repo.path(),
        cache.path(),
        &[
            "graph",
            "--path",
            "packages/replay/src/session.ts",
            "--lens",
            "impact",
            "--changed",
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/graph.schema.json", &graph);
    assert!(
        graph["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|group| group["reason"] == "graph nodes hidden by limit"
                && group["expand"].as_str().is_some_and(|expand| {
                    expand.contains("--path packages/replay/src/session.ts")
                        && expand.contains("--changed")
                        && !expand.contains("<larger-number>")
                })),
        "graph hidden expand should preserve both path scope and changed selector: {graph:#}"
    );
}
