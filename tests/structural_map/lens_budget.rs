#[test]
fn contract_limit_reports_hidden_export_surfaces() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/many-contracts.ts"),
        "export function alpha() { return 1; }\nexport function beta() { return 2; }\nexport function gamma() { return 3; }\n",
    );

    let contract = run_json(
        repo.path(),
        cache.path(),
        &[
            "contract",
            "packages/replay/src/many-contracts.ts",
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/contract.schema.json", &contract);
    assert_eq!(
        contract["exported_contracts"]
            .as_array()
            .expect("exported contracts")
            .len(),
        1
    );
    assert!(
        contract["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|group| group["reason"] == "exported contract surfaces hidden by limit"
                && group["count"] == 2
                && group["expand"].as_str().is_some_and(|expand| {
                    expand.starts_with("codemap contract packages/replay/src/many-contracts.ts --include-hidden --limit ")
                        && !expand.contains("<larger-number>")
                })),
        "contract lens must not silently drop export surfaces behind --limit: {contract:#}"
    );
}

#[test]
fn runtime_limit_reports_hidden_worker_surfaces() {
    let (repo, cache) = fixture();
    for name in ["alpha", "beta", "gamma"] {
        write(
            &repo
                .path()
                .join(format!("packages/app/src/jobs/{name}-worker.ts")),
            "export function run() { return true; }\n",
        );
    }

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &[
            "runtime",
            "packages/app/src/jobs",
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/runtime.schema.json", &runtime);
    assert_eq!(runtime["workers"].as_array().expect("workers").len(), 1);
    assert!(
        runtime["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|group| group["reason"] == "worker/job surfaces hidden by limit"
                && group["count"] == 2
                && group["expand"].as_str().is_some_and(|expand| {
                    expand.starts_with("codemap runtime packages/app/src/jobs --include-hidden --limit ")
                        && !expand.contains("<larger-number>")
                })),
        "runtime lens must not silently drop worker/job surfaces behind --limit: {runtime:#}"
    );
}

#[test]
fn flow_limit_reports_hidden_dependency_steps() {
    let (repo, cache) = fixture();
    for name in ["alpha", "beta", "gamma"] {
        write(
            &repo.path().join(format!("app/api/report/{name}.ts")),
            "export function part() { return true; }\n",
        );
    }
    write(
        &repo.path().join("app/api/report/route.ts"),
        "import { part as alpha } from './alpha';\nimport { part as beta } from './beta';\nimport { part as gamma } from './gamma';\n\nexport function GET() {\n  return Response.json([alpha(), beta(), gamma()]);\n}\n",
    );

    let flow = run_json(
        repo.path(),
        cache.path(),
        &["flow", "/api/report", "--limit", "2", "--format", "json"],
    );
    assert_schema("schemas/flow.schema.json", &flow);
    assert_eq!(flow["steps"].as_array().expect("steps").len(), 2);
    assert!(
        flow["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|group| group["reason"] == "flow steps hidden by limit"
                && group["count"].as_u64().unwrap_or_default() >= 3
                && group["expand"].as_str().is_some_and(|expand| {
                    expand.starts_with("codemap flow /api/report --include-hidden --limit ")
                        && !expand.contains("<larger-number>")
                })),
        "flow lens must show when dependency steps are hidden by --limit: {flow:#}"
    );
}
