// Responsibility: request-guard and response-output provenance regressions
#[test]
fn runtime_flow_and_cone_separate_guard_and_response_roles() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("app/api/panels/route.ts"),
        r#"import { guardRequest } from "../../../lib/guard";
import { createPanel } from "../../../lib/panels";

export async function POST(req: Request) {
  securityPermissionTelemetry(req);
  const payload = sanitizePayload(await req.json());
  return guardRequest(req, async () => {
    const panel = await createPanel(payload);
    const output = publicPanel(panel);
    return respond(output);
  });
}

function sanitizePayload(input: { title: string; ignored?: string }) {
  return { title: input.title };
}

function publicPanel(primary: { id: string; secret?: string }) {
  const alias = primary;
  const publicId = alias.id;
  const unrelated = { secret: "not-primary" };
  if (!primary.secret) return primary;
  return { id: publicId, secret: unrelated.secret };
}

function respond(value: unknown) { return Response.json(value); }
function securityPermissionTelemetry(_req: Request) {}
"#,
    );
    write(
        &repo.path().join("lib/guard.ts"),
        r#"import { requireSession } from "./session";
import { auditSecurityPermission } from "./telemetry";
export async function guardRequest(req: Request, next: () => Promise<unknown>) {
  await requireSession(req);
  auditSecurityPermission(req);
  return next();
}
"#,
    );
    write(
        &repo.path().join("lib/session.ts"),
        "export async function requireSession(_req: Request) {}\n",
    );
    write(
        &repo.path().join("lib/telemetry.ts"),
        "export function auditSecurityPermission(_req: Request) {}\n",
    );
    write(
        &repo.path().join("lib/panels.ts"),
        "export async function createPanel(input: unknown) { return input; }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "request and response roles"]);

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "app/api/panels/route.ts", "--all", "--format", "json"],
    );
    let paths = runtime["paths"].as_array().expect("paths");
    let relation = |kind: &str, target: &str| {
        paths.iter().any(|edge| {
            edge["type"] == kind
                && edge["to"].as_str().is_some_and(|to| to.contains(target))
        })
    };
    for guard in ["guardRequest", "requireSession"] {
        assert!(relation("guarded_by", guard), "missing guard {guard}: {runtime:#}");
    }
    for telemetry in ["securityPermissionTelemetry", "auditSecurityPermission"] {
        assert!(relation("routes_to", telemetry), "missing telemetry {telemetry}: {runtime:#}");
        assert!(!relation("guarded_by", telemetry), "telemetry became a guard: {runtime:#}");
    }
    assert!(relation("transforms", "publicPanel:without(secret)"), "{runtime:#}");
    assert!(
        paths.iter().filter_map(|edge| edge["to"].as_str()).all(|to| {
            !to.starts_with("response_projection:") || !to.contains("sanitizePayload")
        }),
        "input sanitization became response evidence: {runtime:#}"
    );

    for (command, anchor) in [
        ("flow", "POST /api/panels"),
        ("cone", "app/api/panels/route.ts"),
    ] {
        let report = run_json(repo.path(), cache.path(), &[command, anchor, "--all", "--format", "json"]);
        let edges = report
            .get("steps")
            .or_else(|| report.get("outgoing"))
            .and_then(|value| value.as_array())
            .expect("flow steps or cone outgoing");
        let text = serde_json::to_string(edges).expect("json edges");
        assert!(text.contains("requireSession"), "{command} lost awaited guard: {report:#}");
        assert!(text.contains("auditSecurityPermission"), "{command} lost telemetry: {report:#}");
        assert!(text.contains("without(secret)"), "{command} lost projection: {report:#}");
    }
}
