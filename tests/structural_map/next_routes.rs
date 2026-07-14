#[test]
fn proof_links_next_route_files_to_e2e_route_visits() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "next-route-proof-fixture",
  "private": true,
  "scripts": {
    "test": "vitest run",
    "test:e2e": "playwright test"
  }
}
"#,
    );
    write(
        &repo.path().join("tsconfig.json"),
        r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": { "@/*": ["./src/*"] }
  }
}
"#,
    );
    write(
        &repo.path().join("src/features/studio/studio-shell.tsx"),
        "export function StudioShell() {\n  return <main data-testid=\"studio-shell\" />;\n}\n",
    );
    write(
        &repo.path().join("app/app/page.tsx"),
        "import { StudioShell } from '@/features/studio/studio-shell';\n\nexport default function StudioAppPage() {\n  return <StudioShell />;\n}\n",
    );
    write(
        &repo.path().join("tests/e2e/studio.spec.ts"),
        "import { test, expect } from '@playwright/test';\n\ntest('/app renders studio', async ({ page }) => {\n  await page.goto('/app');\n  await expect(page.locator('[data-testid=\"studio-shell\"]')).toBeVisible();\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/not-app.spec.ts"),
        "import { test, expect } from '@playwright/test';\n\ntest('does not land on app', async ({ page }) => {\n  await page.goto('/login');\n  await expect(page).not.toHaveURL('/app');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/same-line-not-app.spec.ts"),
        "import { test, expect } from '@playwright/test';\n\ntest('does not land on app in one line', async ({ page }) => {\n  await page.goto('/login'); await expect(page).not.toHaveURL('/app');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/href-only.spec.ts"),
        "import { test, expect } from '@playwright/test';\n\ntest('renders app link without visiting it', async ({ page }) => {\n  await page.goto('/login');\n  await expect(page.getByRole('link')).toHaveAttribute('href', '/app');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/commented-route.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('commented navigation is ignored', async ({ page }) => {\n  // await page.goto('/app');\n  await page.goto('/login');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/not-playwright-page.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('non-page object goto is ignored', async () => {\n  const notPage = { goto(_path: string) {} };\n  notPage.goto('/app');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/goto-url.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('goto-like method is ignored', async ({ page }) => {\n  page.gotoURL('/app');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/dynamic-user.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('dynamic route smoke', async ({ page }) => {\n  await page.goto('/users/123');\n});\n",
    );
    write(
        &repo
            .path()
            .join("tests/e2e/dynamic-user-extra-segment.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('dynamic route extra segment is different route', async ({ page }) => {\n  await page.goto('/users/123/settings');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/dynamic-user-missing.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('dynamic route missing segment is different route', async ({ page }) => {\n  await page.goto('/users');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/docs-catchall.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('docs catchall route smoke', async ({ page }) => {\n  await page.goto('/docs/getting-started/install');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/docs-root.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('docs root route smoke', async ({ page }) => {\n  await page.goto('/docs');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/blog-root.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('blog optional catchall root route smoke', async ({ page }) => {\n  await page.goto('/blog');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/nested-admin.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('nested app route smoke', async ({ page }) => {\n  await page.goto('/admin');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/package-app-dashboard.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('package named app route smoke', async ({ page }) => {\n  await page.goto('/dashboard');\n});\n",
    );
    write(
        &repo.path().join("app/users/[id]/page.tsx"),
        "export default function UserPage() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("app/docs/[...slug]/page.tsx"),
        "export default function DocsPage() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("app/blog/[[...slug]]/page.tsx"),
        "export default function BlogPage() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("apps/web/src/app/admin/page.tsx"),
        "export default function NestedAdminPage() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("packages/app/app/dashboard/page.tsx"),
        "export default function PackageAppDashboardPage() {\n  return null;\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let route_proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "app/app/page.tsx", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &route_proof);
    assert!(
        route_proof["proofs"]
            .as_array()
            .expect("route proofs")
            .iter()
            .any(|proof| proof["path"] == "tests/e2e/studio.spec.ts"
                && proof["evidence"] == "e2e_route"
                && proof["strength"] == "high"
                && proof["locations"][0]["path"] == "tests/e2e/studio.spec.ts"
                && proof["locations"][0]["line_start"] == 4
                && proof["locations"][0]["kind"] == "route_visit"),
        "Next route file should map to exact e2e page.goto route proof: {route_proof:#}"
    );
    for false_proof in [
        "tests/e2e/not-app.spec.ts",
        "tests/e2e/same-line-not-app.spec.ts",
        "tests/e2e/href-only.spec.ts",
        "tests/e2e/commented-route.spec.ts",
        "tests/e2e/not-playwright-page.spec.ts",
        "tests/e2e/goto-url.spec.ts",
    ] {
        assert!(
            route_proof["proofs"]
                .as_array()
                .expect("route proofs")
                .iter()
                .all(|proof| proof["path"] != false_proof),
            "non-navigation route literals must not become e2e_route proof: {route_proof:#}"
        );
    }

    let shell_proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "src/features/studio/studio-shell.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &shell_proof);
    assert!(
        shell_proof["proofs"]
            .as_array()
            .expect("shell proofs")
            .iter()
            .any(|proof| proof["path"] == "tests/e2e/studio.spec.ts"
                && proof["evidence"] == "e2e_route_via_direct_consumer"
                && proof["strength"] == "medium"
                && proof["locations"][0]["path"] == "tests/e2e/studio.spec.ts"
                && proof["locations"][0]["line_start"] == 4
                && proof["locations"][0]["kind"] == "route_visit"),
        "route e2e surface should be available to the shell as mediated evidence through its direct route consumer: {shell_proof:#}"
    );
    assert!(
        shell_proof["unknowns"]
            .as_array()
            .expect("shell proof unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "direct_test_import_not_found"),
        "mediated e2e surface must not hide the missing direct verification surface sensor for the shell: {shell_proof:#}"
    );

    let dynamic_proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "app/users/[id]/page.tsx", "--format", "json"],
    );
    assert!(
        dynamic_proof["proofs"]
            .as_array()
            .expect("dynamic proofs")
            .iter()
            .any(|proof| proof["path"] == "tests/e2e/dynamic-user.spec.ts"
                && proof["evidence"] == "e2e_route"
                && proof["strength"] == "high"),
        "dynamic route proof should map [id] to a concrete page.goto segment: {dynamic_proof:#}"
    );
    for false_proof in [
        "tests/e2e/dynamic-user-extra-segment.spec.ts",
        "tests/e2e/dynamic-user-missing.spec.ts",
    ] {
        assert!(
            dynamic_proof["proofs"]
                .as_array()
                .expect("dynamic proofs")
                .iter()
                .all(|proof| proof["path"] != false_proof),
            "dynamic route proof must not overmatch sibling route shapes: {dynamic_proof:#}"
        );
    }

    let catchall_proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "app/docs/[...slug]/page.tsx", "--format", "json"],
    );
    assert!(
        catchall_proof["proofs"]
            .as_array()
            .expect("catchall proofs")
            .iter()
            .any(|proof| proof["path"] == "tests/e2e/docs-catchall.spec.ts"
                && proof["evidence"] == "e2e_route"),
        "catch-all route proof should map [...slug] to a deeper page.goto route: {catchall_proof:#}"
    );
    assert!(
        catchall_proof["proofs"]
            .as_array()
            .expect("catchall proofs")
            .iter()
            .all(|proof| proof["path"] != "tests/e2e/docs-root.spec.ts"),
        "non-optional catch-all route must not match the route root: {catchall_proof:#}"
    );

    let optional_catchall_proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "app/blog/[[...slug]]/page.tsx", "--format", "json"],
    );
    assert!(
        optional_catchall_proof["proofs"]
            .as_array()
            .expect("optional catchall proofs")
            .iter()
            .any(|proof| proof["path"] == "tests/e2e/blog-root.spec.ts"
                && proof["evidence"] == "e2e_route"),
        "optional catch-all should match the route root: {optional_catchall_proof:#}"
    );

    let nested_app_proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "apps/web/src/app/admin/page.tsx",
            "--format",
            "json",
        ],
    );
    assert!(
        nested_app_proof["proofs"]
            .as_array()
            .expect("nested app proofs")
            .iter()
            .any(|proof| proof["path"] == "tests/e2e/nested-admin.spec.ts"
                && proof["evidence"] == "e2e_route"),
        "Next route proof should work in nested monorepo src/app layouts: {nested_app_proof:#}"
    );

    let package_named_app_proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/app/dashboard/page.tsx",
            "--format",
            "json",
        ],
    );
    assert!(
        package_named_app_proof["proofs"]
            .as_array()
            .expect("package named app proofs")
            .iter()
            .any(
                |proof| proof["path"] == "tests/e2e/package-app-dashboard.spec.ts"
                    && proof["evidence"] == "e2e_route"
            ),
        "Next route proof should use the final /app/ route root when a package is named app: {package_named_app_proof:#}"
    );
    assert_eq!(route_proof.get("read_first"), None);
}


#[test]
fn proof_does_not_link_ambiguous_duplicate_next_routes() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "duplicate-next-route-fixture",
  "private": true,
  "scripts": { "test:e2e": "playwright test" }
}
"#,
    );
    write(
        &repo.path().join("apps/web/src/app/admin/page.tsx"),
        "export default function WebAdminPage() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("apps/web/app/admin/page.tsx"),
        "export default function LegacyWebAdminPage() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("apps/ops/src/app/admin/page.tsx"),
        "export default function OpsAdminPage() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("tests/e2e/admin.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('admin route smoke', async ({ page }) => {\n  await page.goto('/admin');\n});\n",
    );
    write(
        &repo.path().join("apps/web/tests/e2e/admin.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('web admin route smoke', async ({ page }) => {\n  await page.goto('/admin');\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "apps/web/src/app/admin/page.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proof surfaces")
            .iter()
            .all(|surface| surface["evidence"] != "e2e_route"),
        "root e2e route proof must not cross domains when two app roots expose the same route: {proof:#}"
    );
}
