#[test]
fn runtime_lens_ignores_nest_imports_without_route_decorators() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/local-metadata.ts"),
        "import { Injectable } from '@nestjs/common';\n\nfunction Get() { return function noop() {}; }\nfunction Controller() { return function noop() {}; }\n\n@Controller()\n@Injectable()\nexport class LocalMetadataOnly {\n  @Get()\n  title = 'not a route';\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &["commit", "-qm", "local metadata decorator fixture"],
    );

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &[
            "runtime",
            "packages/app/src/local-metadata.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/runtime.schema.json", &runtime);
    assert!(
        runtime["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .all(|unknown| unknown["kind"] != "unsupported_framework_route"),
        "non-route Nest imports and local decorators must not become framework route unknowns: {runtime:#}"
    );
}

#[test]
fn diff_map_staged_uses_staged_framework_context_when_worktree_removes_imports() {
    let (repo, cache) = fixture();
    let path = repo.path().join("packages/app/src/staged-auth.controller.ts");
    write(
        &path,
        "import { Controller, Get } from '@nestjs/common';\n\n@Controller('/auth')\nexport class AuthController {\n  @Get(':id')\n  show() { return true; }\n}\n",
    );
    git(repo.path(), &["add", "packages/app/src/staged-auth.controller.ts"]);
    write(
        &path,
        "export class AuthController {\n  @Get(':id')\n  show() { return true; }\n}\n",
    );

    let staged = run_json(
        repo.path(),
        cache.path(),
        &["diff-map", "--staged", "--format", "json"],
    );
    assert_schema("schemas/diff-map.schema.json", &staged);
    assert!(
        staged["new_unknowns"]
            .as_array()
            .expect("new unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "unsupported_framework_route"
                && unknown["path"] == "packages/app/src/staged-auth.controller.ts"
                && unknown["line_start"] == 3),
        "diff-map --staged must derive unsupported route context from the staged blob, not the working tree: {staged:#}"
    );
}

#[test]
fn diff_map_staged_ignores_worktree_framework_imports_absent_from_index() {
    let (repo, cache) = fixture();
    let path = repo.path().join("packages/app/src/staged-local-decorators.ts");
    write(
        &path,
        "function Get() { return function noop() {}; }\nfunction Controller() { return function noop() {}; }\n\n@Controller()\nexport class LocalMetadataOnly {\n  @Get()\n  title = 'not a route';\n}\n",
    );
    git(
        repo.path(),
        &["add", "packages/app/src/staged-local-decorators.ts"],
    );
    write(
        &path,
        "import { Controller, Get, Injectable } from '@nestjs/common';\n\n@Controller('/fake')\n@Injectable()\nexport class LocalMetadataOnly {\n  @Get()\n  title = 'not a staged route';\n}\n",
    );

    let staged = run_json(
        repo.path(),
        cache.path(),
        &["diff-map", "--staged", "--format", "json"],
    );
    assert_schema("schemas/diff-map.schema.json", &staged);
    assert!(
        staged["new_unknowns"]
            .as_array()
            .expect("new unknowns")
            .iter()
            .all(|unknown| unknown["kind"] != "unsupported_framework_route"),
        "diff-map --staged must not let unstaged framework imports create route unknowns: {staged:#}"
    );
}
