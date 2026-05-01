#[test]
fn directory_cone_stays_at_directory_level_without_file_galaxy() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "directory-cone-fixture",
  "private": true,
  "scripts": { "test:e2e": "playwright test" }
}
"#,
    );
    write(
        &repo.path().join("app/page.tsx"),
        "import { Hero } from './_landing/hero';\nimport { userSchema } from '../src/schema/user.dto';\n\nexport default function Page() {\n  return <Hero title={userSchema.name} />;\n}\n",
    );
    write(
        &repo.path().join("app/_landing/hero.tsx"),
        "import { Button } from '../../src/design';\n\nexport function Hero() {\n  return <Button>Start</Button>;\n}\n",
    );
    write(
        &repo.path().join("app/api/logout/route.ts"),
        "import { logout } from '../../../src/lib/server/auth';\n\nexport async function POST() {\n  return logout();\n}\n",
    );
    write(
        &repo.path().join("src/design/index.ts"),
        "export function Button(props: { children: string }) {\n  return props.children;\n}\n",
    );
    write(
        &repo.path().join("src/lib/server/auth.ts"),
        "export function logout() {\n  return Response.json({ ok: true });\n}\n",
    );
    write(
        &repo.path().join("src/schema/user.dto.ts"),
        "export const userSchema = { name: 'user' };\n",
    );
    write(
        &repo.path().join("tests/e2e/smoke.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('home route smoke', async ({ page }) => {\n  await page.goto('/');\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "app", "--depth", "1", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert_eq!(cone["anchor"]["kind"], "directory");
    assert!(
        cone["outgoing"]
            .as_array()
            .expect("outgoing")
            .iter()
            .any(|edge| edge["from"] == "app/" && edge["to"] == "app/_landing/"),
        "directory cone should show same-level child edges instead of file imports: {cone:#}"
    );
    assert!(
        cone["outgoing"]
            .as_array()
            .expect("outgoing")
            .iter()
            .any(|edge| edge["from"] == "app/api/" && edge["to"] == "src/"),
        "directory cone should preserve external domain edges at this level: {cone:#}"
    );
    assert!(
        cone["proof"]
            .as_array()
            .expect("proof")
            .iter()
            .any(|edge| edge["from"] == "tests/"
                && edge["to"] == "app/"
                && edge["evidence"] == "e2e_route"),
        "directory cone should aggregate e2e proof to the directory level: {cone:#}"
    );
    assert!(
        cone["contracts"]
            .as_array()
            .expect("contracts")
            .iter()
            .any(|edge| edge["from"] == "app/"
                && edge["to"] == "src/"
                && edge["evidence"] == "role:schema_contract"),
        "directory cone should preserve contract/schema edges at the directory level: {cone:#}"
    );
    let mut proof_keys = Vec::new();
    for edge in cone["proof"].as_array().expect("proof") {
        let key = (
            edge["from"].as_str().unwrap_or_default(),
            edge["to"].as_str().unwrap_or_default(),
            edge["type"].as_str().unwrap_or_default(),
        );
        assert!(
            !proof_keys.contains(&key),
            "directory proof should keep one strongest edge per endpoint: {cone:#}"
        );
        proof_keys.push(key);
    }
    for section in ["outgoing", "incoming", "proof", "contracts", "boundary"] {
        assert!(
            cone[section]
                .as_array()
                .expect("edge section")
                .iter()
                .all(|edge| {
                    ["from", "to"].into_iter().all(|key| {
                        let value = edge[key].as_str().unwrap_or_default();
                        !(value.ends_with(".ts")
                            || value.ends_with(".tsx")
                            || value.ends_with(".js")
                            || value.ends_with(".jsx"))
                    })
                }),
            "directory cone should not leak file-level endpoints in {section}: {cone:#}"
        );
    }

    let deeper = run_json(
        repo.path(),
        cache.path(),
        &["cone", "app", "--depth", "2", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &deeper);
    assert!(
        deeper["outgoing"]
            .as_array()
            .expect("deeper outgoing")
            .iter()
            .any(|edge| edge["from"] == "app/api/logout/" && edge["to"] == "src/lib/"),
        "directory cone --depth 2 should reveal the next external layer without file endpoints: {deeper:#}"
    );
    assert!(
        deeper["contracts"]
            .as_array()
            .expect("deeper contracts")
            .iter()
            .any(|edge| edge["from"] == "app/" && edge["to"] == "src/schema/"),
        "directory cone --depth 2 should reveal schema contract layer without file endpoints: {deeper:#}"
    );
}


#[test]
fn tsconfig_jsonc_path_aliases_create_reverse_edges() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "jsonc-alias-fixture",
  "private": true
}
"#,
    );
    write(
        &repo.path().join("tsconfig.json"),
        r#"{
  // Real tsconfig files commonly use JSONC syntax.
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@/*": [
        "./src/*",
      ],
    },
  },
  "include": ["**/*.ts", "**/*.tsx"],
  "exclude": [
    "node_modules",
  ],
}
"#,
    );
    write(
        &repo.path().join("src/features/studio/studio-shell.tsx"),
        "export function StudioShell() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("app/app/page.tsx"),
        "import { StudioShell } from '@/features/studio/studio-shell';\n\nexport default function Page() {\n  return <StudioShell />;\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "src/features/studio/studio-shell.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert_eq!(cone["anchor"]["imported_by_count"], 1);
    assert!(
        cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .any(|edge| edge["from"] == "app/app/page.tsx"
                && edge["to"] == "src/features/studio/studio-shell.tsx"
                && edge["type"] == "imported_by"
                && edge["evidence"] == "reverse_import"),
        "JSONC tsconfig path aliases should produce reverse structural edges: {cone:#}"
    );
    assert_eq!(cone.get("read_first"), None);
}


#[test]
fn malformed_tsconfig_jsonc_does_not_create_alias_edges() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "malformed-jsonc-alias-fixture",
  "private": true
}
"#,
    );
    write(
        &repo.path().join("tsconfig.json"),
        r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@/*": ["./src/*"]
    }
  }
}
/* unterminated
"#,
    );
    write(
        &repo.path().join("src/features/studio/studio-shell.tsx"),
        "export function StudioShell() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("app/app/page.tsx"),
        "import { StudioShell } from '@/features/studio/studio-shell';\n\nexport default function Page() {\n  return <StudioShell />;\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "src/features/studio/studio-shell.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert_eq!(cone["anchor"]["imported_by_count"], 0);
    assert!(
        cone["incoming"].as_array().expect("incoming").is_empty(),
        "malformed tsconfig JSONC must fail closed instead of creating alias edges: {cone:#}"
    );
    assert_eq!(cone.get("read_first"), None);
}

