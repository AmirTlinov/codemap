// Responsibility: contract-and-data-lineage-regressions
#[test]
fn contract_lineage_connects_sql_owner_consumers_topics_health_and_direct_proof() {
    let repo = TempDir::new().expect("repo");
    let cache = TempDir::new().expect("cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"lineage","private":true,"workspaces":["packages/*","apps/*"],"scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join("packages/adapters/package.json"),
        r#"{"name":"@fixture/adapters","exports":{".":"./src/index.ts"}}"#,
    );
    write(
        &repo.path().join("apps/web/package.json"),
        r#"{"name":"@fixture/web","dependencies":{"@fixture/adapters":"workspace:*"}}"#,
    );
    write(
        &repo.path().join("apps/web/.env.example"),
        "CONTROL_CENTER_OUTBOX_URL=postgresql://localhost/outbox\n",
    );
    write(
        &repo.path().join("apps/web/db/migrations/001_outbox.sql"),
        "CREATE TABLE IF NOT EXISTS control_center_outbox (\n  id TEXT PRIMARY KEY,\n  topic TEXT NOT NULL,\n  payload JSONB NOT NULL\n);\n",
    );
    write(
        &repo.path().join("packages/adapters/src/outbox/repository.ts"),
        r#"const DEFAULT_TABLE_NAME = "control_center_outbox";
export function createOutboxRepository(tableName = DEFAULT_TABLE_NAME) {
  return {
    enqueue: (payload: unknown) => { client.query('BEGIN'); return client.query(`INSERT INTO ${tableName} (payload) VALUES ($1)`, [payload]); },
    list: () => client.query(`SELECT * FROM ${tableName}`),
  };
}
"#,
    );
    write(
        &repo.path().join("packages/adapters/src/index.ts"),
        "export { createOutboxRepository } from './outbox/repository';\n",
    );
    write(
        &repo.path().join("apps/web/src/dispatcher.ts"),
        r#"import { createOutboxRepository } from "@fixture/adapters";
const repository = createOutboxRepository();
export async function enqueue() {
  const event = { topic: "catalog.item.created", payload: {} };
  const computedEvent = { topic: buildTopic(), payload: {} };
  void computedEvent;
  return repository.enqueue(event);
}
export function getOutboxHealth() { return { configured: Boolean(process.env.CONTROL_CENTER_OUTBOX_URL) }; }
"#,
    );
    write(
        &repo.path().join("apps/web/src/health.ts"),
        "import { getOutboxHealth } from './dispatcher';\nexport const health = () => getOutboxHealth();\n",
    );
    write(
        &repo.path().join("apps/web/tests/health.test.ts"),
        "import { health } from '../src/health';\ntest('health', () => expect(health()).toBeTruthy());\n",
    );
    write(
        &repo.path().join("apps/web/tests/unrelated-outbox.test.ts"),
        "test('outbox words are not lineage', () => expect('outbox').toBeTruthy());\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "sql lineage"]);

    let report = run_json(
        repo.path(),
        cache.path(),
        &["contract", "apps/web/db/migrations/001_outbox.sql", "--all", "--format", "json"],
    );
    assert_schema("schemas/contract.schema.json", &report);
    let declarations = report["declarations"].as_array().expect("declarations");
    for expected in ["table:control_center_outbox", "field:control_center_outbox.topic"] {
        assert!(
            declarations.iter().any(|surface| surface["id"] == expected),
            "missing declaration {expected}: {report:#}"
        );
    }
    assert!(
        declarations
            .iter()
            .any(|surface| surface["id"] == "config:CONTROL_CENTER_OUTBOX_URL"),
        "static config declaration is missing: {report:#}"
    );
    let lineage = report["lineage"].as_array().expect("lineage");
    let has = |kind: &str, from: &str, to: &str| {
        lineage.iter().any(|edge| {
            edge["type"] == kind
                && edge["from"].as_str().is_some_and(|value| value.contains(from))
                && edge["to"].as_str().is_some_and(|value| value.contains(to))
        })
    };
    for (kind, from, to) in [
        ("declares", "001_outbox.sql", "table:control_center_outbox"),
        ("writes", "repository.ts", "table:control_center_outbox"),
        ("reads", "repository.ts", "table:control_center_outbox"),
        ("consumes", "dispatcher.ts", "index.ts"),
        ("consumes", "health.ts", "dispatcher.ts"),
        ("emits", "dispatcher.ts", "topic:catalog.item.created"),
        (
            "reads_config",
            "dispatcher.ts",
            "config:CONTROL_CENTER_OUTBOX_URL",
        ),
        (
            "declares",
            ".env.example",
            "config:CONTROL_CENTER_OUTBOX_URL",
        ),
        ("crosses_boundary", "repository.ts", "transaction_group:"),
    ] {
        assert!(has(kind, from, to), "missing {kind} {from} -> {to}: {report:#}");
    }
    assert!(
        lineage.iter().any(|edge| {
            edge["from"].as_str().is_some_and(|path| path.ends_with("src/index.ts"))
                && edge["evidence"] == "direct_static_consumer"
        }),
        "the first exact repository consumer must remain direct: {report:#}"
    );
    assert!(
        lineage.iter().any(|edge| {
            edge["from"].as_str().is_some_and(|path| path.ends_with("dispatcher.ts"))
                && edge["evidence"] == "mediated_static_consumer"
        }),
        "downstream consumers must be marked as mediated instead of soft overlap: {report:#}"
    );
    let proof = report["proof"].as_array().expect("proof");
    assert!(
        proof.iter().any(|edge| edge["from"].as_str().is_some_and(|path| path.ends_with("health.test.ts"))),
        "direct health proof is missing: {report:#}"
    );
    assert!(
        proof.iter().all(|edge| !edge["from"].as_str().is_some_and(|path| path.ends_with("unrelated-outbox.test.ts"))),
        "lexically similar tests must not enter verification: {report:#}"
    );
    assert!(
        report["unknowns"].as_array().expect("unknowns").iter().any(|item| item["kind"] == "dynamic_sql_table"),
        "runtime table override must remain a typed stop: {report:#}"
    );
    assert!(
        report["unknowns"].as_array().expect("unknowns").iter().any(|item| item["kind"] == "computed_topic"),
        "computed topics must remain typed stops: {report:#}"
    );
}

#[test]
fn contract_lineage_stops_at_computed_codegen_outputs() {
    let repo = TempDir::new().expect("repo");
    let cache = TempDir::new().expect("cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(&repo.path().join("contracts/admin.yaml"), "openapi: 3.0.0\npaths: {}\n");
    write(
        &repo.path().join("scripts/generate.mjs"),
        "generate('contracts/admin.yaml', process.env.GENERATED_OUTPUT);\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "dynamic codegen"]);
    let report = run_json(
        repo.path(),
        cache.path(),
        &["contract", "contracts/admin.yaml", "--format", "json"],
    );
    assert!(
        report["unknowns"].as_array().expect("unknowns").iter().any(|item| item["kind"] == "runtime_generated_schema"),
        "computed generated artifacts must not become guessed paths: {report:#}"
    );
}

#[test]
fn contract_lineage_declares_graphql_and_protobuf_types_fields_and_operations() {
    let repo = TempDir::new().expect("repo");
    let cache = TempDir::new().expect("cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("contracts/admin.graphql"),
        "type User {\n  id: ID!\n}\ntype Query {\n  user(id: ID!): User\n}\n",
    );
    write(
        &repo.path().join("contracts/admin.proto"),
        "syntax = \"proto3\";\nmessage User {\n  string id = 1;\n}\nservice Admin {\n  rpc GetUser (User) returns (User);\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "schema declarations"]);

    let graphql = run_json(
        repo.path(),
        cache.path(),
        &["contract", "contracts/admin.graphql", "--all", "--format", "json"],
    );
    let proto = run_json(
        repo.path(),
        cache.path(),
        &["contract", "contracts/admin.proto", "--all", "--format", "json"],
    );
    for (report, expected) in [
        (&graphql, ["schema_type:User", "field:User.id", "field:Query.user"]),
        (&proto, ["schema_type:User", "field:User.id", "field:Admin.GetUser"]),
    ] {
        let declarations = report["declarations"].as_array().expect("declarations");
        for id in expected {
            assert!(
                declarations.iter().any(|surface| surface["id"] == id),
                "missing declaration {id}: {report:#}"
            );
        }
    }
}

#[test]
fn contract_lineage_connects_openapi_codegen_export_consumer_and_drift_verification() {
    let repo = TempDir::new().expect("repo");
    let cache = TempDir::new().expect("cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"codegen-lineage","private":true,"workspaces":["packages/*","apps/*"],"scripts":{"generate:check":"node packages/api/scripts/generate.mjs && git diff --exit-code packages/api/src/types.ts"}}"#,
    );
    write(
        &repo.path().join("contracts/admin.yaml"),
        "openapi: 3.0.0\npaths:\n  /users:\n    get:\n      responses: {}\n",
    );
    write(
        &repo.path().join("packages/api/package.json"),
        r#"{"name":"@fixture/api","exports":{".":"./src/types.ts"}}"#,
    );
    write(
        &repo.path().join("packages/api/scripts/generate.mjs"),
        "generate('contracts/admin.yaml', 'packages/api/src/types.ts');\n",
    );
    write(
        &repo.path().join("packages/api/src/types.ts"),
        "// generated\nexport interface paths { '/users': unknown }\n",
    );
    write(
        &repo.path().join("apps/web/package.json"),
        r#"{"name":"@fixture/web","dependencies":{"@fixture/api":"workspace:*"}}"#,
    );
    write(
        &repo.path().join("apps/web/src/client.ts"),
        "import type { paths } from '@fixture/api';\nexport type Api = paths;\n",
    );
    write(
        &repo.path().join("apps/web/tests/generated.test.ts"),
        "import type { paths } from '@fixture/api';\ntest('generated', () => expect(true).toBe(true));\n",
    );
    write(
        &repo.path().join("docs/codegen.md"),
        "Regenerate contracts/admin.yaml into packages/api/src/types.ts.\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "codegen lineage"]);

    let report = run_json(
        repo.path(),
        cache.path(),
        &["contract", "contracts/admin.yaml", "--all", "--format", "json"],
    );
    assert_schema("schemas/contract.schema.json", &report);
    let lineage = report["lineage"].as_array().expect("lineage");
    let kinds = lineage
        .iter()
        .filter_map(|edge| edge["type"].as_str())
        .collect::<BTreeSet<_>>();
    for kind in ["consumes", "generates", "exports", "verifies_directly"] {
        assert!(kinds.contains(kind), "missing {kind}: {report:#}");
    }
    assert!(
        lineage.iter().any(|edge| edge["type"] == "generates" && edge["to"].as_str().is_some_and(|to| to.contains("packages/api/src/types.ts"))),
        "generated artifact edge is missing: {report:#}"
    );
    assert!(
        lineage.iter().any(|edge| edge["type"] == "consumes" && edge["from"].as_str().is_some_and(|from| from.ends_with("client.ts"))),
        "application package consumer is missing: {report:#}"
    );
    assert!(
        lineage.iter().all(|edge| {
            edge["from"].as_str().is_none_or(|from| {
                !from.ends_with("generated.test.ts") && !from.ends_with("docs/codegen.md")
            })
        }),
        "tests and documentation must not masquerade as application or generator lineage: {report:#}"
    );
    assert!(
        report["unknowns"].as_array().expect("unknowns").iter().all(|item| item["kind"] != "generation_verification_missing"),
        "exact drift command should close generation verification: {report:#}"
    );

    let manifest_report = run_json(
        repo.path(),
        cache.path(),
        &["contract", "package.json", "--format", "json"],
    );
    assert!(
        manifest_report["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .all(|item| item["kind"] != "contract_codegen_consumer_missing"),
        "ordinary JSON manifests must not be promoted to contract sources: {manifest_report:#}"
    );
}
