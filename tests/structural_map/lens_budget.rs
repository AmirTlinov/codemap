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
                    expand.starts_with("codemap contract packages/replay/src/many-contracts.ts --all --limit ")
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
    assert_eq!(runtime["workers"].as_array().expect("workers").len(), 3);
    let readable = run_markdown(
        repo.path(),
        cache.path(),
        &["runtime", "packages/app/src/jobs", "--limit", "1"],
    );
    assert!(
        readable.contains("- workers: counted(3); shown=1 hidden=2")
            && readable.contains("codemap runtime packages/app/src/jobs --all --limit 3")
            && !readable.contains("worker/job surfaces hidden by limit")
            && !readable.contains("<larger-number>"),
        "the worker horizon must own bounded visibility without detached hidden accounting: {readable}"
    );
}

#[test]
fn runtime_worker_job_surfaces_require_exact_path_conventions() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/jobs/email.ts"),
        "export function runEmailJob() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/src/email.worker.ts"),
        "export function runEmailWorker() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/src/cron/nightly.ts"),
        "export function nightlyCron() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/src/objective.ts"),
        "export function objective() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/src/jobbery.ts"),
        "export function jobbery() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/src/queue/types.ts"),
        "export interface QueueConfig { enabled: boolean }\n",
    );

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "packages/app/src", "--format", "json"],
    );
    assert_schema("schemas/runtime.schema.json", &runtime);
    let workers = runtime["workers"].as_array().expect("workers");
    for expected in [
        "packages/app/src/jobs/email.ts",
        "packages/app/src/email.worker.ts",
        "packages/app/src/cron/nightly.ts",
    ] {
        assert!(
            workers.iter().any(|surface| surface["path"] == expected
                && surface["evidence"] == "worker_job_path_convention"),
            "runtime should expose exact worker/job convention `{expected}`: {runtime:#}"
        );
    }
    for rejected in [
        "packages/app/src/objective.ts",
        "packages/app/src/jobbery.ts",
        "packages/app/src/queue/types.ts",
    ] {
        assert!(
            workers.iter().all(|surface| surface["path"] != rejected),
            "runtime must not promote substring matches to worker/job surfaces: {runtime:#}"
        );
    }
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
                    expand.starts_with("codemap flow /api/report --all --limit ")
                        && !expand.contains("<larger-number>")
                })),
        "flow lens must show when dependency steps are hidden by --limit: {flow:#}"
    );
}
