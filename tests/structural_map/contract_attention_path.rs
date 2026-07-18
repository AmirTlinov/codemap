// Responsibility: contract-attention-path-regression
#[test]
fn cone_contract_where_and_embedded_schema_form_one_exact_attention_path() {
    let repo = TempDir::new().expect("repo");
    let cache = TempDir::new().expect("cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"contract-attention","private":true,"workspaces":["apps/*"]}"#,
    );
    write(
        &repo.path().join("apps/web/package.json"),
        r#"{"name":"@fixture/web","private":true}"#,
    );
    write(
        &repo.path().join("apps/web/src/route.ts"),
        "import { ReplayMount } from './ReplayMount';\nimport { createTerrainPlacement } from './surface-layer';\nexport const route = [ReplayMount, createTerrainPlacement];\n",
    );
    write(
        &repo.path().join("apps/web/src/ReplayMount.tsx"),
        "import type { ReplayManifest } from './replay/types';\nimport { ReplayHud } from './ReplayHud';\nexport const ReplayMount = (manifest: ReplayManifest) => <ReplayHud id={manifest.session_id} />;\n",
    );
    write(
        &repo.path().join("apps/web/src/ReplayHud.tsx"),
        "export const ReplayHud = ({ id }: { id: string }) => <p>{id}</p>;\n",
    );
    write(
        &repo.path().join("apps/web/src/replay/types.ts"),
        "export interface ReplayManifest { session_id: string }\n",
    );
    write(
        &repo.path().join("apps/web/src/surface-layer.ts"),
        "import type { TerrainTextures } from './terrain/load-texture';\nexport const createTerrainPlacement = () => 1;\nexport const buildTerrainLayer = (textures: TerrainTextures) => textures.ready;\n",
    );
    write(
        &repo.path().join("apps/web/src/terrain/load-texture.ts"),
        "export interface TerrainTextures { ready: boolean }\n",
    );
    write(
        &repo.path().join("apps/web/src/schemas.generated.ts"),
        "// generated from replay-manifest.schema.json\nexport interface ReplayManifest { session_id: string }\n",
    );
    write(
        &repo.path().join("crates/replay-format/Cargo.toml"),
        "[package]\nname = \"replay-format\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(
        &repo.path().join("crates/replay-format/src/lib.rs"),
        "pub struct ReplayManifest { pub session_id: String }\nconst SCHEMA: &str = include_str!(\"../../../schemas/replay-manifest.schema.json\");\n",
    );
    write(
        &repo.path().join("schemas/replay-manifest.schema.json"),
        r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","title":"ReplayManifest","type":"object"}"#,
    );
    write(
        &repo.path().join("contracts/07-replay-contract.md"),
        "# Replay contract\n\nThe replay manifest is the package authority.\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let cone = run_markdown(
        repo.path(),
        cache.path(),
        &["cone", "apps/web/src/route.ts"],
    );
    assert!(
        cone.contains("codemap contract apps/web/src/replay/types.ts"),
        "a visible contract owner should become an exact focused expand: {cone}"
    );
    assert!(
        cone.contains("apps/web/src/terrain/load-texture.ts"),
        "the adjacent renderer owner should retain its texture-success boundary: {cone}"
    );
    assert!(
        !cone.contains("contract -> `apps/web/src/ReplayHud.tsx`"),
        "rendered components are runtime uses, not public type contracts: {cone}"
    );

    let contract = run_markdown(
        repo.path(),
        cache.path(),
        &["contract", "apps/web/src/replay/types.ts"],
    );
    assert!(
        contract.contains("codemap where ReplayManifest"),
        "parallel exact contract definitions should be inspectable without name search: {contract}"
    );
    for expected in [
        "crates/replay-format/src/lib.rs#ReplayManifest",
        "schemas/replay-manifest.schema.json",
        "contracts/07-replay-contract.md",
    ] {
        assert!(
            contract.contains(expected),
            "the contract map should expose the supported parallel owner chain `{expected}`: {contract}"
        );
    }

    let where_report = run_markdown(repo.path(), cache.path(), &["where", "ReplayManifest"]);
    assert!(
        where_report.contains("codemap cone 'crates/replay-format/src/lib.rs#ReplayManifest'"),
        "multi-definition where must print the exact cone for the Rust owner: {where_report}"
    );

    let rust_owner = run_json(
        repo.path(),
        cache.path(),
        &["cone", "crates/replay-format/src/lib.rs", "--format", "json"],
    );
    assert!(
        rust_owner["outgoing"]
            .as_array()
            .expect("outgoing")
            .iter()
            .any(|edge| {
                edge["from"] == "crates/replay-format/src/lib.rs"
                    && edge["to"] == "schemas/replay-manifest.schema.json"
                    && edge["type"] == "imports"
            }),
        "include_str! must expose its exact embedded schema dependency: {rust_owner:#}"
    );
}

#[test]
fn exact_entry_cone_keeps_adjacent_domain_contract_visible() {
    let repo = TempDir::new().expect("repo");
    let cache = TempDir::new().expect("cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("apps/web/src/chronicle/page.ts"),
        "import { loadReplay } from '../lib/replay/loader';\nexport const page = loadReplay;\n",
    );
    write(
        &repo.path().join("apps/web/src/lib/replay/loader.ts"),
        "export function loadReplay() { return 'mounted'; }\n",
    );
    write(
        &repo.path().join("contracts/07-replay-contract.md"),
        "# Replay contract\n\nThe terrain carrier is mandatory.\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let cone = run_markdown(
        repo.path(),
        cache.path(),
        &["cone", "apps/web/src/chronicle/page.ts"],
    );
    assert!(
        cone.contains("contracts/07-replay-contract.md"),
        "an adjacent domain owner should retain its repository contract: {cone}"
    );
    assert!(
        cone.contains("codemap contract contracts/07-replay-contract.md"),
        "the contract candidate should be directly expandable: {cone}"
    );
}

#[test]
fn exported_symbol_type_dependency_opens_the_shared_contract_consumers() {
    let repo = TempDir::new().expect("repo");
    let cache = TempDir::new().expect("cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"private":true,"workspaces":["apps/*"]}"#,
    );
    write(
        &repo.path().join("apps/ws/package.json"),
        r#"{"name":"@fixture/ws","exports":{"./protocol":"./src/protocol.ts"}}"#,
    );
    write(
        &repo.path().join("apps/web/package.json"),
        r#"{"name":"@fixture/web","private":true}"#,
    );
    write(
        &repo.path().join("apps/ws/src/protocol.ts"),
        "export interface ClientFrame { type: string }\n",
    );
    write(
        &repo.path().join("apps/ws/src/hub.ts"),
        "import type { ClientFrame } from './protocol';\nexport class TownHub {\n  handle(frame: ClientFrame): string { return frame.type; }\n}\n",
    );
    write(
        &repo.path().join("apps/web/src/ChatStrip.ts"),
        "import type { ClientFrame } from '../../ws/src/protocol';\nexport const render = (frame: ClientFrame) => frame.type;\n",
    );
    write(
        &repo.path().join("apps/web/src/ChatStrip.test.ts"),
        "import { render } from './ChatStrip';\ntest('renders a welcome frame', () => expect(render({ type: 'welcome' })).toBe('welcome'));\n",
    );
    write(
        &repo.path().join("apps/ws/src/protocol.test.ts"),
        "import type { ClientFrame } from './protocol';\ntest('keeps the welcome frame', () => expect(({ type: 'welcome' } as ClientFrame).type).toBe('welcome'));\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "shared protocol fixture"]);

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "apps/ws/src/hub.ts#TownHub",
            "--format",
            "json",
        ],
    );
    assert!(
        cone["contracts"].as_array().expect("contracts").iter().any(|edge| {
            edge["from"] == "apps/ws/src/hub.ts#TownHub"
                && edge["to"] == "apps/ws/src/protocol.ts"
                && edge["evidence"] == "public_symbol_type_dependency"
        }),
        "the public class signature should expose its shared protocol owner: {cone:#}"
    );
    assert!(
        cone["expand"]
            .as_array()
            .expect("expands")
            .iter()
            .any(|expand| expand == "codemap contract apps/ws/src/protocol.ts"),
        "the exact symbol cone should open the contract consumer map directly: {cone:#}"
    );
    assert!(
        cone["contracts"]
            .as_array()
            .expect("contracts")
            .iter()
            .any(|edge| {
                edge["from"] == "apps/web/src/ChatStrip.ts"
                    && edge["to"] == "apps/ws/src/protocol.ts"
                    && edge["evidence"] == "public_symbol_type_consumer"
            }),
        "the exact symbol cone should inline the cross-package contract consumer: {cone:#}"
    );
    assert!(
        cone["proof"].as_array().expect("proof").iter().any(|edge| {
            edge["from"] == "apps/web/src/ChatStrip.test.ts"
                && edge["to"] == "apps/ws/src/protocol.ts"
        }),
        "the first symbol cone should retain verification of its shared contract consumer: {cone:#}"
    );

    let contract = run_json(
        repo.path(),
        cache.path(),
        &["contract", "apps/ws/src/protocol.ts", "--format", "json"],
    );
    assert!(
        contract["cross_package_consumers"]
            .as_array()
            .expect("cross package consumers")
            .iter()
            .any(|edge| edge["from"] == "apps/web/src/ChatStrip.ts"),
        "the exact contract expand must carry the downstream UI consumer: {contract:#}"
    );
    assert!(
        contract["proof"].as_array().expect("proof").iter().any(|edge| {
            edge["from"] == "apps/ws/src/protocol.test.ts"
                && edge["to"] == "apps/ws/src/protocol.ts"
        }),
        "parallel contract lineage must not replace direct contract verification: {contract:#}"
    );
}
