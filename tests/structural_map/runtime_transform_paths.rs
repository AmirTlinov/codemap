// Responsibility: runtime-boundary-transformation-regressions
#[test]
fn runtime_path_carries_guard_service_projection_response_and_deployment_config() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("app/api/panels/route.ts"),
        r#"import { NextResponse } from "next/server";
import { guardTenantMutation } from "../../../lib/guard";
import { createPanel } from "../../../lib/panels";

const createSchema = z.object({ title: z.string() });

export async function POST(req: Request) {
  const parsed = createSchema.safeParse(await req.json());
  return guardTenantMutation(req, async () => {
    const internal = await createPanel(normalizeInput(parsed.data));
    return respond(stripInternal(internal));
  });
}

function normalizeInput(result: { title: string; ignored?: string }) {
  return { title: result.title };
}

function respond(body: unknown) {
  return NextResponse.json(body);
}

function stripInternal(result: { id: string; title: string; internalToken?: string }) {
  if (!result.internalToken) return result;
  return {
    id: result.id,
    title: result.title,
  };
}
"#,
    );
    write(
        &repo.path().join("lib/guard.ts"),
        r#"import { getSession } from "./session";
export async function guardTenantMutation(req: Request, handler: () => Promise<unknown>) {
  const key = process.env.SESSION_KEY;
  recordCsrfFailure();
  await getSession(key);
  return handler();
}
function recordCsrfFailure() { return true; }
"#,
    );
    write(
        &repo.path().join("lib/session.ts"),
        "export async function getSession(key: string | undefined) { return key; }\n",
    );
    write(
        &repo.path().join("lib/panels.ts"),
        r#"import { savePanel } from "./store";
export async function createPanel(input: { title: string }) {
  return savePanel({ ...input, internalToken: "secret" });
}
"#,
    );
    write(
        &repo.path().join("lib/store.ts"),
        "export async function savePanel(value: unknown) { return value; }\n",
    );
    write(
        &repo.path().join("deploy/k8s/deployment.yaml"),
        "env:\n  - name: SESSION_KEY\n    valueFrom:\n      secretKeyRef:\n        name: app-session\n        key: session\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "runtime boundary path"]);

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &[
            "runtime",
            "app/api/panels/route.ts",
            "--all",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/runtime.schema.json", &runtime);
    let paths = runtime["paths"].as_array().expect("runtime paths");
    let has = |kind: &str, from: &str, to: &str| {
        paths.iter().any(|edge| {
            edge["type"] == kind
                && edge["from"].as_str().is_some_and(|value| value.contains(from))
                && edge["to"].as_str().is_some_and(|value| value.contains(to))
        })
    };
    for (kind, from, to) in [
        ("routes_to", "POST /api/panels", "route.ts#POST"),
        ("guarded_by", "route.ts#POST", "guard.ts#guardTenantMutation"),
        ("guarded_by", "route.ts#POST", "route.ts#createSchema"),
        ("routes_to", "route.ts#POST", "panels.ts#createPanel"),
        ("routes_to", "panels.ts#createPanel", "store.ts#savePanel"),
        ("transforms", "route.ts#POST", "route.ts#stripInternal"),
        ("transforms", "route.ts#stripInternal", "route.ts#respond"),
        ("transforms", "route.ts#respond", "external_response:"),
        ("transforms", "route.ts#stripInternal", "without(internalToken)"),
        ("reads", "guard.ts#guardTenantMutation", "environment:SESSION_KEY"),
        ("configured_by", "environment:SESSION_KEY", "deployment.yaml"),
    ] {
        assert!(has(kind, from, to), "missing {kind} {from} -> {to}: {runtime:#}");
    }
    assert!(
        paths.iter().all(|edge| edge["from"] != edge["to"]),
        "runtime paths must not emit self-edges: {runtime:#}"
    );
    assert!(
        paths.iter().filter_map(|edge| edge["to"].as_str()).all(|to| {
            !to.starts_with("external_response:") || !to.contains("internalToken")
        }),
        "internal fields must not be copied onto the external response surface: {runtime:#}"
    );
    assert_eq!(
        paths
            .iter()
            .filter(|edge| edge["to"]
                .as_str()
                .is_some_and(|to| to.starts_with("external_response:")))
            .count(),
        1,
        "NextResponse.json must not also match its Response.json suffix: {runtime:#}"
    );
    assert!(
        paths.iter().filter_map(|edge| edge["to"].as_str()).all(|to| {
            !to.starts_with("response_projection:") || !to.contains("normalizeInput")
        }),
        "input normalization must not be claimed as an external response projection: {runtime:#}"
    );
    let post = runtime["routes"]
        .as_array()
        .expect("routes")
        .iter()
        .find(|route| route["method"] == "POST")
        .expect("POST route");
    let guards = post["middleware_or_guards"].as_array().expect("guards");
    for expected in ["createSchema", "guardTenantMutation", "getSession"] {
        assert!(
            guards.iter().any(|guard| guard["name"] == expected),
            "route must retain `{expected}` as a MiddlewareOrGuard entity: {runtime:#}"
        );
    }
    assert!(
        guards.iter().all(|guard| guard["name"] != "recordCsrfFailure"),
        "a CSRF-related observer name is not by itself a guard: {runtime:#}"
    );

    let flow = run_json(
        repo.path(),
        cache.path(),
        &["flow", "POST /api/panels", "--all", "--format", "json"],
    );
    assert_schema("schemas/flow.schema.json", &flow);
    let kinds = flow["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .filter_map(|step| step["kind"].as_str())
        .collect::<BTreeSet<_>>();
    for kind in ["routes_to", "guarded_by", "transforms", "reads", "configured_by"] {
        assert!(kinds.contains(kind), "flow lost runtime path relation {kind}: {flow:#}");
    }
    assert!(
        flow["steps"].as_array().expect("steps").iter().any(|step| {
            step["kind"] == "transforms"
                && step["anchor"].as_str().is_some_and(|anchor| {
                    anchor.contains("stripInternal") && anchor.contains("respond")
                })
        }),
        "flow path steps must retain both source and target owners: {flow:#}"
    );

    let cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "app/api/panels/route.ts", "--all", "--format", "json"],
    );
    let cone_kinds = cone["outgoing"]
        .as_array()
        .expect("outgoing")
        .iter()
        .filter_map(|edge| edge["type"].as_str())
        .collect::<BTreeSet<_>>();
    for kind in ["routes_to", "guarded_by", "transforms"] {
        assert!(cone_kinds.contains(kind), "cone lost runtime relation {kind}: {cone:#}");
    }
}

#[test]
fn express_middleware_arguments_keep_last_handler_and_explicit_guards() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/server.ts"),
        "router.post('/users', authGuard, auditMiddleware, createUser);\nfunction authGuard() {}\nfunction auditMiddleware() {}\nexport function createUser() { return true; }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "middleware route"]);

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "packages/app/src/server.ts", "--format", "json"],
    );
    let route = &runtime["routes"][0];
    assert_eq!(route["handler_symbol"], "createUser", "{runtime:#}");
    let guards = route["middleware_or_guards"].as_array().expect("guards");
    assert_eq!(
        guards
            .iter()
            .filter_map(|guard| guard["name"].as_str())
            .collect::<Vec<_>>(),
        vec!["auditMiddleware", "authGuard"],
        "{runtime:#}"
    );
}

#[test]
fn exact_route_beats_cross_package_catchall_and_dynamic_dispatch_stays_a_stop() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/app/api/panels/route.ts"),
        "export async function POST(req: Request) {\n  const handler = container.resolve(req);\n  return handlers[req.method](req);\n}\n",
    );
    write(
        &repo.path().join("packages/proxy/app/api/[[...path]]/route.ts"),
        "export async function POST() { return Response.json({ proxy: true }); }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "exact and dynamic route"]);

    let flow = run_json(
        repo.path(),
        cache.path(),
        &["flow", "POST /api/panels", "--format", "json"],
    );
    assert!(
        flow["steps"].as_array().expect("steps").iter().any(|step| {
            step["kind"] == "route_anchor" && step["anchor"] == "POST /api/panels"
        }),
        "literal route must win over another package's catch-all: {flow:#}"
    );
    let unknowns = flow["unknown_breaks"].as_array().expect("unknowns");
    for expected in ["runtime_di_boundary", "runtime_dynamic_dispatch"] {
        assert!(
            unknowns.iter().any(|unknown| unknown["kind"] == expected),
            "missing typed dynamic stop `{expected}`: {flow:#}"
        );
    }
    assert!(
        unknowns.iter().all(|unknown| unknown["kind"] != "route_anchor_ambiguous"),
        "catch-all must not make an exact literal route ambiguous: {flow:#}"
    );
}
