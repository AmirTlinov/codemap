#[test]
fn standalone_playwright_consumer_and_repository_runner_prove_next_route() {
    let (repo, cache) = fixture();
    let route = "packages/app/src/app/(game)/town/chronicle/[match_id]/page.tsx";
    let browser = "packages/app/scripts/replay-live-route-capture.mjs";
    let dynamic_browser = "packages/app/scripts/dynamic-route-capture.mjs";
    let runner = "tools/review/replay-live-route-capture.sh";
    write(
        &repo.path().join(route),
        "export default function ChronicleReplayPage() { return <main data-replay-scene />; }\n",
    );
    write(
        &repo.path().join(browser),
        r#"import { chromium } from "playwright";
const port = Number(process.env.PORT);
const url = new URL(`http://127.0.0.1:${port}/town/chronicle/worldgen_match_4242`);
const instance = await chromium.launch({ headless: true });
const page = await instance.newPage();
await page.goto(url.toString(), { waitUntil: "networkidle" });
await page.locator("[data-replay-scene]").waitFor();
await instance.close();
"#,
    );
    write(
        &repo.path().join(runner),
        r#"#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
(cd "$repo_root/packages/app" && PORT=7001 node "$repo_root/packages/app/scripts/replay-live-route-capture.mjs")
"#,
    );
    write(
        &repo.path().join(dynamic_browser),
        r#"import { chromium } from "playwright";
const matchId = process.env.MATCH_ID;
const url = new URL(`http://127.0.0.1:7001/town/chronicle/${matchId}`);
const instance = await chromium.launch();
const page = await instance.newPage();
await page.goto(url.toString());
"#,
    );
    write(
        &repo.path().join("tools/review/commented-capture.sh"),
        "#!/usr/bin/env bash\n# node packages/app/scripts/replay-live-route-capture.mjs\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "browser route proof fixture"]);

    let browser_ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", browser, "--format", "json"],
    );
    assert!(
        browser_ls["anchor"]["roles"]
            .as_array()
            .expect("browser roles")
            .iter()
            .any(|role| role == "proof_runner"),
        "a standalone Playwright route capture is an explicit proof runner: {browser_ls:#}"
    );
    let dynamic_ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", dynamic_browser, "--format", "json"],
    );
    assert!(
        dynamic_ls["anchor"]["roles"]
            .as_array()
            .expect("dynamic browser roles")
            .iter()
            .all(|role| role != "proof_runner"),
        "a dynamic route value must not become static browser proof: {dynamic_ls:#}"
    );

    let cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", route, "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    let proof = cone["proof"].as_array().expect("cone proof");
    for expected in [browser, runner] {
        assert!(
            proof.iter().any(|edge| edge["from"] == expected
                && edge["to"] == route
                && edge["evidence"] == "e2e_route"),
            "exact cone should show the browser route consumer and its repository runner ({expected}): {cone:#}"
        );
    }
    assert!(
        proof
            .iter()
            .all(|edge| edge["from"] != "tools/review/commented-capture.sh"
                && edge["from"] != dynamic_browser),
        "commented process text and dynamic routes must not become route proof: {cone:#}"
    );

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &["proof-map", route, "--format", "json"],
    );
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    let direct_surfaces = proof_map["hard"]
        .as_array()
        .expect("hard proof")
        .iter()
        .chain(
            proof_map["direct_evidence"]
                .as_array()
                .expect("direct evidence"),
        )
        .collect::<Vec<_>>();
    for expected in [browser, runner] {
        assert!(
            direct_surfaces.iter().any(|surface| surface["path"] == expected
                    && surface["evidence"] == "e2e_visited_route"),
            "proof-map should preserve the complete browser proof chain ({expected}): {proof_map:#}"
        );
    }
    assert!(
        proof_map["commands"]
            .as_array()
            .expect("proof commands")
            .iter()
            .all(|surface| surface["path"] != browser),
        "standalone browser capture must not be rendered as an invalid Vitest file command: {proof_map:#}"
    );
}
