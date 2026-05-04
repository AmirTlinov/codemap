#[test]
fn runtime_fact_index_matches_dynamic_next_route_visits() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("app/users/[id]/page.tsx"),
        "export default function UserPage() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("tests/e2e/dynamic-user.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('dynamic route smoke', async ({ page }) => {\n  await page.goto('/users/123');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/dynamic-user-extra-segment.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('dynamic route extra segment is not this route', async ({ page }) => {\n  await page.goto('/users/123/settings');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/dynamic-user-missing.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('dynamic route missing segment is not this route', async ({ page }) => {\n  await page.goto('/users');\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "dynamic runtime route fixture"]);

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &["proof-map", "app/users/[id]/page.tsx", "--format", "json"],
    );
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    assert!(
        proof_map["hard"]
            .as_array()
            .expect("hard")
            .iter()
            .any(|proof| proof["path"] == "tests/e2e/dynamic-user.spec.ts"
                && proof["evidence"] == "e2e_visited_route"
                && proof["reason"]
                    .as_str()
                    .is_some_and(|reason| reason.contains("GET /users/:id"))),
        "proof-map should attach concrete page.goto visits to the dynamic route owner: {proof_map:#}"
    );
    for false_proof in [
        "tests/e2e/dynamic-user-extra-segment.spec.ts",
        "tests/e2e/dynamic-user-missing.spec.ts",
    ] {
        assert!(
            proof_map["hard"]
                .as_array()
                .expect("hard")
                .iter()
                .all(|proof| proof["path"] != false_proof),
            "dynamic runtime route matching must not overmatch sibling path shapes: {proof_map:#}"
        );
    }
}

#[test]
fn flow_resolves_concrete_visit_to_dynamic_next_route_anchor() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("app/users/[id]/page.tsx"),
        "import { seek } from '@fixture/replay';\n\nexport default function UserPage() {\n  return seek(1).frame;\n}\n",
    );
    write(
        &repo.path().join("tests/e2e/dynamic-user.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('dynamic route smoke', async ({ page }) => {\n  await page.goto('/users/123');\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "dynamic runtime flow fixture"]);

    let flow = run_json(repo.path(), cache.path(), &["flow", "/users/123", "--format", "json"]);
    assert_schema("schemas/flow.schema.json", &flow);
    assert!(
        flow["steps"]
            .as_array()
            .expect("steps")
            .iter()
            .any(|step| step["kind"] == "route_anchor"
                && step["anchor"] == "GET /users/:id"
                && step["locations"][0]["path"] == "app/users/[id]/page.tsx"),
        "flow should resolve a concrete URL to the deterministic dynamic route owner: {flow:#}"
    );
    assert!(
        flow["proof"]
            .as_array()
            .expect("proof")
            .iter()
            .any(|edge| edge["from"] == "tests/e2e/dynamic-user.spec.ts"
                && edge["evidence"] == "e2e_visited_route"),
        "flow should carry the e2e page.goto proof for the dynamic route owner: {flow:#}"
    );
}

#[test]
fn dynamic_route_visit_fails_closed_when_static_route_owner_also_matches() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("app/users/[id]/page.tsx"),
        "export default function UserPage() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("app/users/settings/page.tsx"),
        "export default function SettingsPage() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("tests/e2e/user-settings.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('settings route smoke', async ({ page }) => {\n  await page.goto('/users/settings');\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "ambiguous dynamic runtime route fixture"]);

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &["proof-map", "app/users/[id]/page.tsx", "--format", "json"],
    );
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    assert!(
        proof_map["hard"]
            .as_array()
            .expect("hard")
            .iter()
            .all(|proof| proof["path"] != "tests/e2e/user-settings.spec.ts"),
        "dynamic route proof must fail closed when a concrete visit also matches a static owner: {proof_map:#}"
    );
    assert!(
        proof_map["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "ambiguous_route_visit_owner"
                && unknown["path"] == "app/users/[id]/page.tsx"),
        "ambiguous dynamic/static route ownership should be a typed unknown: {proof_map:#}"
    );
}

#[test]
fn workspace_package_dynamic_routes_use_same_owner_policy_in_proof_and_proof_map() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/admin/package.json"),
        r#"{
  "name": "@fixture/admin",
  "private": true,
  "scripts": { "test:e2e": "playwright test" }
}
"#,
    );
    write(
        &repo.path().join("packages/app/app/users/[id]/page.tsx"),
        "export default function AppUserPage() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("packages/admin/app/users/[id]/page.tsx"),
        "export default function AdminUserPage() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("packages/app/tests/e2e/user.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('app user route smoke', async ({ page }) => {\n  await page.goto('/users/123');\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &["commit", "-qm", "workspace dynamic route owner fixture"],
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "packages/app/app/users/[id]/page.tsx", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .any(|proof| proof["path"] == "packages/app/tests/e2e/user.spec.ts"
                && proof["evidence"] == "e2e_route"),
        "legacy proof should use package-scoped dynamic route ownership, not global silence: {proof:#}"
    );

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof-map",
            "packages/app/app/users/[id]/page.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    assert!(
        proof_map["hard"]
            .as_array()
            .expect("hard")
            .iter()
            .any(|proof| proof["path"] == "packages/app/tests/e2e/user.spec.ts"
                && proof["evidence"] == "e2e_visited_route"
                && proof["reason"]
                    .as_str()
                    .is_some_and(|reason| reason.contains("GET /users/:id"))
                && !proof["reason"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("GET /app/users/:id")),
        "proof-map should use the same package-scoped owner and final app route root as proof: {proof_map:#}"
    );
}
