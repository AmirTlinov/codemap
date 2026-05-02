#[test]
fn runtime_lens_extracts_additional_static_api_forms_and_blind_spots() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/api.ts"),
        "const method = 'get';\nconst routePath = '/computed-path';\nconst prefix = '/tenant';\nrouter.route('/users/:id').get(getUser).delete(deleteUser);\nfastify.route({ method: 'PATCH', url: '/fastify/users', handler: patchUser });\nfastify.route({ \"method\": \"PUT\", \"url\": \"/quoted/users\", handler: putUser });\nfastify.route({ 'method': 'POST', 'path': '/single-quoted/users', handler: postUser });\nfastify.route({ method: 'GET', url: routePath, handler: dynamicUser });\nrouter[method]('/computed-method', computedMethod);\nrouter.get(routePath, computedPath);\nrouter.get('/tenant/' + id, tenantHandler);\napp.use('/api', apiRouter);\napp.use(prefix + '/routes', tenantRouter);\nexport function getUser() { return true; }\nexport function deleteUser() { return true; }\nexport function patchUser() { return true; }\nexport function putUser() { return true; }\nexport function postUser() { return true; }\nexport function dynamicUser() { return true; }\nexport function computedMethod() { return true; }\nexport function computedPath() { return true; }\nexport function tenantHandler() { return true; }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "runtime extractor fixture"]);

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "packages/app/src/api.ts", "--format", "json"],
    );
    assert_schema("schemas/runtime.schema.json", &runtime);
    for (method, path, evidence) in [
        ("GET", "/users/:id", "javascript_route_chain_registration"),
        ("DELETE", "/users/:id", "javascript_route_chain_registration"),
        (
            "PATCH",
            "/fastify/users",
            "javascript_route_object_registration",
        ),
        (
            "PUT",
            "/quoted/users",
            "javascript_route_object_registration",
        ),
        (
            "POST",
            "/single-quoted/users",
            "javascript_route_object_registration",
        ),
    ] {
        assert!(
            runtime["routes"]
                .as_array()
                .expect("runtime routes")
                .iter()
                .any(|route| route["method"] == method
                    && route["path"] == path
                    && route["evidence"] == evidence
                    && route["locations"][0]["path"] == "packages/app/src/api.ts"),
            "runtime lens should expose deterministic route form {method} {path}: {runtime:#}"
        );
    }
    for (method, path, handler) in [
        ("GET", "/users/:id", "getUser"),
        ("DELETE", "/users/:id", "deleteUser"),
        ("PATCH", "/fastify/users", "patchUser"),
        ("PUT", "/quoted/users", "putUser"),
        ("POST", "/single-quoted/users", "postUser"),
    ] {
        assert!(
            runtime["routes"]
                .as_array()
                .expect("runtime routes")
                .iter()
                .any(|route| route["method"] == method
                    && route["path"] == path
                    && route["handler_symbol"] == handler),
            "runtime lens should stitch static route {method} {path} to handler `{handler}`: {runtime:#}"
        );
    }
    for path in [
        "/computed-method",
        "/computed-path",
        "/tenant/",
        "/api",
        "/tenant/routes",
    ] {
        assert!(
            runtime["routes"]
                .as_array()
                .expect("runtime routes")
                .iter()
                .all(|route| route["path"] != path),
            "dynamic or mount-only route facts must not become exact routes ({path}): {runtime:#}"
        );
    }
    for kind in [
        "route_object_dynamic",
        "route_dynamic_method",
        "route_dynamic_path",
        "route_string_concat",
        "route_mount_prefix",
        "route_mount_dynamic_prefix",
    ] {
        assert!(
            runtime["unknowns"]
                .as_array()
                .expect("runtime unknowns")
                .iter()
                .any(|unknown| unknown["kind"] == kind
                    && unknown["path"] == "packages/app/src/api.ts"
                    && unknown["line_start"].as_u64().is_some()),
            "runtime lens should expose typed blind spot `{kind}` with location: {runtime:#}"
        );
    }
}

#[test]
fn runtime_lens_reports_unsupported_framework_route_decorators_as_unknowns() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/auth.controller.ts"),
        "import { Controller, Get, Post } from '@nestjs/common';\n\n@Controller('/auth')\nexport class AuthController {\n  @Get(':id')\n  show() { return true; }\n\n  @Post()\n  create() { return true; }\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "unsupported decorator route fixture"]);

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &[
            "runtime",
            "packages/app/src/auth.controller.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/runtime.schema.json", &runtime);
    assert!(
        runtime["routes"]
            .as_array()
            .expect("runtime routes")
            .is_empty(),
        "unsupported decorator framework routes must not become exact runtime routes: {runtime:#}"
    );
    let unknowns = runtime["unknowns"].as_array().expect("unknowns");
    for line in [3, 5, 8] {
        assert!(
            unknowns.iter().any(|unknown| unknown["kind"] == "unsupported_framework_route"
                && unknown["path"] == "packages/app/src/auth.controller.ts"
                && unknown["line_start"] == line),
            "unsupported framework route decorator at line {line} should be a typed blind spot: {runtime:#}"
        );
    }
}

#[test]
fn runtime_lens_does_not_treat_local_decorator_names_as_framework_routes() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/local-decorators.ts"),
        "function Get() { return function noop() {}; }\nfunction Controller() { return function noop() {}; }\n\n@Controller()\nexport class LocalMetadataOnly {\n  @Get()\n  title = 'not a route';\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "local decorator negative fixture"]);

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &[
            "runtime",
            "packages/app/src/local-decorators.ts",
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
        "local ordinary decorators must not become framework route unknowns: {runtime:#}"
    );
}

#[test]
fn runtime_lens_reads_go_gorilla_method_chains_without_guessing_missing_methods() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/server.go"),
        "package main\n\nfunc routes(r Router, method string) {\n    r.HandleFunc(\"/health\", health).Methods(\"GET\")\n    http.HandleFunc(\"/ready\", ready)\n    r.HandleFunc(\"/borrow\", borrow); other.Methods(\"POST\")\n    r.HandleFunc(\"/headers\", headers).Headers(\"X\", other.Methods(\"POST\"))\n    r.HandleFunc(\"/dynamic\", dynamic).Methods(method)\n    r.HandleFunc(\"/dynamic-prefix\", dynamicPrefix).Methods(\"GET\" + method)\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "go runtime extractor fixture"]);

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "packages/app/src/server.go", "--format", "json"],
    );
    assert_schema("schemas/runtime.schema.json", &runtime);
    assert!(
        runtime["routes"]
            .as_array()
            .expect("runtime routes")
            .iter()
            .any(|route| route["method"] == "GET"
                && route["path"] == "/health"
                && route["handler_symbol"] == "health"
                && route["evidence"] == "go_http_route_registration"),
        "Gorilla-style `.Methods(\"GET\")` should make the route method and handler exact: {runtime:#}"
    );
    assert!(
        runtime["routes"]
            .as_array()
            .expect("runtime routes")
            .iter()
            .any(|route| route["method"] == "ANY"
                && route["path"] == "/ready"
                && route["evidence"] == "go_http_route_registration"),
        "plain net/http HandleFunc should remain explicit ANY instead of guessing GET: {runtime:#}"
    );
    assert!(
        runtime["routes"]
            .as_array()
            .expect("runtime routes")
            .iter()
            .any(|route| route["method"] == "ANY"
                && route["path"] == "/borrow"
                && route["evidence"] == "go_http_route_registration"),
        "unrelated `.Methods(\"POST\")` later on the line must not be borrowed by HandleFunc: {runtime:#}"
    );
    assert!(
        runtime["routes"]
            .as_array()
            .expect("runtime routes")
            .iter()
            .any(|route| route["method"] == "ANY"
                && route["path"] == "/headers"
                && route["evidence"] == "go_http_route_registration"),
        "nested `.Methods(\"POST\")` inside another chained call argument must not become exact method: {runtime:#}"
    );
    assert!(
        runtime["routes"]
            .as_array()
            .expect("runtime routes")
            .iter()
            .all(|route| !(route["method"] == "POST"
                && (route["path"] == "/borrow" || route["path"] == "/headers"))),
        "Go method extraction must be scoped to the actual HandleFunc chain: {runtime:#}"
    );
    assert!(
        runtime["routes"]
            .as_array()
            .expect("runtime routes")
            .iter()
            .all(|route| route["path"] != "/dynamic" && route["path"] != "/dynamic-prefix"),
        "dynamic Go Methods(method) should not become exact ANY or static route facts: {runtime:#}"
    );
    assert!(
        runtime["unknowns"]
            .as_array()
            .expect("runtime unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "route_dynamic_method"
                && unknown["path"] == "packages/app/src/server.go"
                && unknown["line_start"].as_u64().is_some()),
        "dynamic Go Methods(method) should be reported as a typed blind spot: {runtime:#}"
    );
}

#[test]
fn runtime_route_extractors_fail_closed_to_local_expression_scope() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/scope.ts"),
        "router.route('/a'); router.get('/b', b);\nrouter.route('/c').get(c); router.route('/d').delete(d);\nrouter.route('/lonely') && apiClient.get('/not-a-route');\nfastify.route({ handler: h, nested: { method: 'GET', url: '/nested' } });\nexport function b() { return true; }\nexport function c() { return true; }\nexport function d() { return true; }\nexport function h() { return true; }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "route scope negative fixture"]);

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "packages/app/src/scope.ts", "--format", "json"],
    );
    assert_schema("schemas/runtime.schema.json", &runtime);
    for (method, path) in [("GET", "/b"), ("GET", "/c"), ("DELETE", "/d")] {
        assert!(
            runtime["routes"]
                .as_array()
                .expect("runtime routes")
                .iter()
                .any(|route| route["method"] == method && route["path"] == path),
            "valid route forms should still be extracted while rejecting false positives: {runtime:#}"
        );
    }
    for (method, path) in [
        ("GET", "/a"),
        ("DELETE", "/c"),
        ("GET", "/lonely"),
        ("GET", "/nested"),
    ] {
        assert!(
            runtime["routes"]
                .as_array()
                .expect("runtime routes")
                .iter()
                .all(|route| !(route["method"] == method && route["path"] == path)),
            "route extractor must not borrow method/path facts outside the local route expression ({method} {path}): {runtime:#}"
        );
    }
    assert!(
        runtime["unknowns"]
            .as_array()
            .expect("runtime unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "route_object_dynamic"
                && unknown["path"] == "packages/app/src/scope.ts"),
        "nested object fields should produce a typed blind spot, not an exact route: {runtime:#}"
    );
}

#[test]
fn runtime_route_extractors_ignore_nested_handler_calls_and_key_suffixes() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/false-positive.ts"),
        "router.route('/a').post(apiClient.get('/not-route'), handler);\nrouter.post('/outer', childRouter.get('/fake-nested'), handler);\nrouter.post('/outer-chain', childRouter.route('/fake-chained').get(handler), handler);\nfastify.route({ badmethod: 'GET', baseurl: '/fake', handler });\nexport function handler() { return true; }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "route parser false positive fixture"]);

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &[
            "runtime",
            "packages/app/src/false-positive.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/runtime.schema.json", &runtime);
    assert!(
        runtime["routes"]
            .as_array()
            .expect("runtime routes")
            .iter()
            .any(|route| route["method"] == "POST" && route["path"] == "/a"),
        "top-level chained route method should still be extracted: {runtime:#}"
    );
    assert!(
        runtime["routes"]
            .as_array()
            .expect("runtime routes")
            .iter()
            .any(|route| route["method"] == "POST" && route["path"] == "/outer"),
        "top-level direct route should still be extracted while rejecting nested child routes: {runtime:#}"
    );
    assert!(
        runtime["routes"]
            .as_array()
            .expect("runtime routes")
            .iter()
            .any(|route| route["method"] == "POST" && route["path"] == "/outer-chain"),
        "top-level direct route should still be extracted while rejecting nested child route chains: {runtime:#}"
    );
    for (method, path) in [
        ("GET", "/a"),
        ("GET", "/fake"),
        ("GET", "/fake-nested"),
        ("GET", "/fake-chained"),
    ] {
        assert!(
            runtime["routes"]
                .as_array()
                .expect("runtime routes")
                .iter()
                .all(|route| !(route["method"] == method && route["path"] == path)),
            "nested calls and key suffixes must not become exact route facts ({method} {path}): {runtime:#}"
        );
    }
    assert!(
        runtime["unknowns"]
            .as_array()
            .expect("runtime unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "route_object_dynamic"
                && unknown["path"] == "packages/app/src/false-positive.ts"),
        "badmethod/baseurl should be a dynamic object blind spot, not exact method/url fields: {runtime:#}"
    );
}

#[test]
fn runtime_route_extractors_ignore_route_text_inside_literals_and_comments() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/literals.ts"),
        "const s = \"router.get('/fake-in-string')\";\nconst t = `router.post('/fake-template')`;\nconst doc = `\nrouter.get('/fake-from-template', handler)\n`;\nconst routePattern = /router.patch('fake-regex')/;\nconst blockMarker = \"/* not a comment */\"; app.post('/real-after-string-marker', handler);\nconst regexMarker = /\\/\\*/; app.put('/real-after-regex-marker', handler);\nconst ru = \"пппппппппппп\"; app.options('/real-after-nonascii-string', handler);\n// русский комментарий app.head('/fake-nonascii-comment')\napp.head('/real-after-nonascii-comment', handler);\n// /* not a block comment\napp.patch('/real-after-line-comment-marker', handler);\n/*\nrouter.put('/fake-block', handler);\n*/\n// router.delete('/fake-comment')\napp.get('/real', handler);\nexport function handler() { return true; }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "route literal false positive fixture"]);

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "packages/app/src/literals.ts", "--format", "json"],
    );
    assert_schema("schemas/runtime.schema.json", &runtime);
    for (method, path) in [
        ("GET", "/real"),
        ("POST", "/real-after-string-marker"),
        ("PUT", "/real-after-regex-marker"),
        ("OPTIONS", "/real-after-nonascii-string"),
        ("HEAD", "/real-after-nonascii-comment"),
        ("PATCH", "/real-after-line-comment-marker"),
    ] {
        assert!(
            runtime["routes"]
                .as_array()
                .expect("runtime routes")
                .iter()
                .any(|route| route["method"] == method
                    && route["path"] == path
                    && route["evidence"] == "javascript_route_registration"),
            "real direct JS route should still be extracted after literal/comment markers ({method} {path}): {runtime:#}"
        );
    }
    for (method, path) in [
        ("GET", "/fake-in-string"),
        ("POST", "/fake-template"),
        ("GET", "/fake-from-template"),
        ("PATCH", "fake-regex"),
        ("PUT", "/fake-block"),
        ("DELETE", "/fake-comment"),
        ("HEAD", "/fake-nonascii-comment"),
    ] {
        assert!(
            runtime["routes"]
                .as_array()
                .expect("runtime routes")
                .iter()
                .all(|route| !(route["method"] == method && route["path"] == path)),
            "route-looking literal/comment text must not become exact route facts ({method} {path}): {runtime:#}"
        );
    }
}

#[test]
fn runtime_route_extractors_require_route_like_python_decorator_receiver() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/api.py"),
        "from fastapi import FastAPI\napp = FastAPI()\n\n\"\"\"\n@app.get('/fake-docstring')\n\"\"\"\n\n@app.get('/health')\ndef health():\n    return {'ok': True}\n\n@cache.get('/fake-cache')\ndef cached():\n    return 'not a route'\n\n@cache(app.get('/fake-nested'))\ndef nested_cached():\n    return 'not a route'\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "python decorator receiver fixture"]);

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "packages/app/src/api.py", "--format", "json"],
    );
    assert_schema("schemas/runtime.schema.json", &runtime);
    assert!(
        runtime["routes"]
            .as_array()
            .expect("runtime routes")
            .iter()
            .any(|route| route["method"] == "GET"
                && route["path"] == "/health"
                && route["evidence"] == "python_route_decorator"),
        "route-like Python decorator should still be extracted: {runtime:#}"
    );
    assert!(
        runtime["routes"]
            .as_array()
            .expect("runtime routes")
            .iter()
            .all(|route| !(route["method"] == "GET" && route["path"] == "/fake-cache")),
        "non-route decorator receivers must not become exact runtime routes: {runtime:#}"
    );
    assert!(
        runtime["routes"]
            .as_array()
            .expect("runtime routes")
            .iter()
            .all(|route| !(route["method"] == "GET" && route["path"] == "/fake-docstring")),
        "Python docstring text must not become exact runtime routes: {runtime:#}"
    );
    assert!(
        runtime["routes"]
            .as_array()
            .expect("runtime routes")
            .iter()
            .all(|route| !(route["method"] == "GET" && route["path"] == "/fake-nested")),
        "nested route-like calls inside unrelated Python decorators must not become exact runtime routes: {runtime:#}"
    );
}
