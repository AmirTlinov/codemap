#[test]
fn definition_match_coverage_fails_open_for_unsupported_source_and_container_files() {
    for (path, source, query, excluded_reason) in [
        (
            "src/needle.kt",
            "fun KotlinNeedle(): Int = 1\n",
            "KotlinNeedle",
            "unsupported_language",
        ),
        (
            "src/Needle.java",
            "public final class JavaNeedle {}\n",
            "JavaNeedle",
            "unsupported_language",
        ),
        (
            "scripts/needle.sh",
            "ShellNeedle() { echo found; }\n",
            "ShellNeedle",
            "unsupported_language",
        ),
        (
            "src/Needle.vue",
            "<script setup lang=\"ts\">\nexport function VueNeedle() { return 1; }\n</script>\n",
            "VueNeedle",
            "unsupported_construct",
        ),
        (
            "src/declared.ts",
            "export declare function DeclaredNeedle(): void;\n",
            "DeclaredNeedle",
            "unsupported_construct",
        ),
        (
            "src/generator.ts",
            "export async function* GeneratorNeedle() { yield 1; }\n",
            "GeneratorNeedle",
            "unsupported_construct",
        ),
        (
            "src/abstract.ts",
            "export abstract class AbstractNeedle {}\n",
            "AbstractNeedle",
            "unsupported_construct",
        ),
        (
            "src/namespace.ts",
            "export namespace NamespaceNeedle { export const value = 1; }\n",
            "NamespaceNeedle",
            "unsupported_construct",
        ),
        (
            "src/one-line.ts",
            "import { value } from './other'; export const OneLineNeedle = value;\n",
            "OneLineNeedle",
            "unsupported_construct",
        ),
        (
            "src/using.ts",
            "using ResourceNeedle = acquire();\n",
            "ResourceNeedle",
            "unsupported_construct",
        ),
        (
            "src/multi.ts",
            "export const First = 1, SecondNeedle = 2;\n",
            "SecondNeedle",
            "unsupported_construct",
        ),
        (
            "src/unicode.ts",
            "export function café() { return 1; }\n",
            "café",
            "unsupported_construct",
        ),
        (
            "src/unbalanced-object.ts",
            "const { UnbalancedObjectNeedle\n",
            "UnbalancedObjectNeedle",
            "unsupported_construct",
        ),
        (
            "src/unbalanced-array.ts",
            "const [UnbalancedArrayNeedle\n",
            "UnbalancedArrayNeedle",
            "unsupported_construct",
        ),
        (
            "src/multiline.ts",
            "export const\n  MultilineNeedle = 42;\n",
            "MultilineNeedle",
            "unsupported_construct",
        ),
        (
            "src/eval.js",
            "eval(\"function RuntimeNeedle() { return 1; }\");\n",
            "RuntimeNeedle",
            "unsupported_construct",
        ),
        (
            "src/indirect-eval.js",
            "(0, eval)(\"function IndirectRuntimeNeedle() { return 1; }\");\n",
            "IndirectRuntimeNeedle",
            "unsupported_construct",
        ),
        (
            "src/optional-eval.js",
            "eval?.(\"function OptionalRuntimeNeedle() { return 1; }\");\n",
            "OptionalRuntimeNeedle",
            "unsupported_construct",
        ),
        (
            "src/computed-eval.js",
            "globalThis[\"eval\"] (\"function ComputedRuntimeNeedle() { return 1; }\");\n",
            "ComputedRuntimeNeedle",
            "unsupported_construct",
        ),
        (
            "src/local-export.ts",
            "function internal() { return 1; } export { internal as PublicTarget };\n",
            "PublicTarget",
            "unsupported_construct",
        ),
        (
            "src/remote-export.ts",
            "export { internal as RemotePublicTarget } from './internal';\n",
            "RemotePublicTarget",
            "unsupported_construct",
        ),
        (
            "src/assignment.cjs",
            "exports.CommonPublicTarget = function() {};\n",
            "CommonPublicTarget",
            "unsupported_construct",
        ),
        (
            "src/module-assignment.cjs",
            "module.exports.CommonClassTarget = class {};\n",
            "CommonClassTarget",
            "unsupported_construct",
        ),
        (
            "src/property.cjs",
            "Object.defineProperty(exports, \"CommonGetterTarget\", { get() { return 1; } });\n",
            "CommonGetterTarget",
            "unsupported_construct",
        ),
    ] {
        let repo = TempDir::new().expect("unsupported definition repo");
        let cache = TempDir::new().expect("unsupported definition cache");
        git(repo.path(), &["init", "-q"]);
        git(repo.path(), &["config", "user.email", "a@example.com"]);
        git(repo.path(), &["config", "user.name", "a"]);
        write(
            &repo.path().join("package.json"),
            r#"{"name":"definition-gap","private":true}"#,
        );
        write(&repo.path().join(path), source);
        git(repo.path(), &["add", "."]);
        git(
            repo.path(),
            &["commit", "-qm", "unsupported definition fixture"],
        );

        let json = run_json(
            repo.path(),
            cache.path(),
            &["where", query, "--format", "json"],
        );
        assert_schema("schemas/where.schema.json", &json);
        let ledger = &json["observations"];
        let definitions = horizon(ledger, "definition_matches");
        assert_eq!(
            definitions["count"]["closure"], "open",
            "{path} must keep definition coverage open: {json:#}"
        );
        let certificate_id = definitions["count"]["certificate_id"]
            .as_str()
            .expect("certificate id");
        let certificate = &ledger["certificates"][certificate_id];
        assert_eq!(certificate["eligible_files"], 1, "{path}: {json:#}");
        assert_eq!(certificate["visited_files"], 0, "{path}: {json:#}");
        assert!(
            certificate["unsupported"]
                .as_array()
                .expect("unsupported definitions")
                .iter()
                .any(|unsupported| unsupported["file"] == path),
            "{path} must be named as unsupported: {json:#}"
        );
        assert!(
            certificate["excluded_files_by_reason"][excluded_reason]
                .as_array()
                .expect("typed excluded files")
                .iter()
                .any(|excluded| excluded == path),
            "{path} needs a typed exclusion: {json:#}"
        );
        assert!(
            definitions["unsupported"]
                .as_array()
                .expect("horizon unsupported definitions")
                .iter()
                .any(|unsupported| unsupported["file"] == path),
            "{path} must propagate into the readable horizon model: {json:#}"
        );
        assert_horizon_certificate_resolves(ledger, definitions);
    }
}

#[test]
fn fully_supported_typescript_definition_match_coverage_is_closed() {
    let repo = TempDir::new().expect("supported definition repo");
    let cache = TempDir::new().expect("supported definition cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"definition-closed","private":true}"#,
    );
    write(
        &repo.path().join("src/owner.ts"),
        "export function TypeScriptNeedle() { return 1; }\n",
    );
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &["commit", "-qm", "supported definition fixture"],
    );

    let json = run_json(
        repo.path(),
        cache.path(),
        &["where", "TypeScriptNeedle", "--format", "json"],
    );
    assert_schema("schemas/where.schema.json", &json);
    assert_eq!(json["total_matches"], 1, "{json:#}");
    let ledger = &json["observations"];
    let definitions = horizon(ledger, "definition_matches");
    assert_eq!(definitions["count"]["closure"], "closed", "{json:#}");
    let certificate_id = definitions["count"]["certificate_id"]
        .as_str()
        .expect("certificate id");
    let certificate = &ledger["certificates"][certificate_id];
    assert_eq!(certificate["eligible_files"], 1, "{json:#}");
    assert_eq!(certificate["visited_files"], 1, "{json:#}");
    assert!(
        certificate["unsupported"]
            .as_array()
            .expect("unsupported definitions")
            .is_empty(),
        "fully supported TS must stay closed: {json:#}"
    );
    assert!(
        certificate["excluded_files_by_reason"]
            .as_object()
            .expect("excluded files")
            .is_empty(),
        "fully supported TS must not invent exclusions: {json:#}"
    );
    assert_horizon_certificate_resolves(ledger, definitions);
}

#[test]
fn definition_certificate_identity_includes_the_normalized_kind_filter() {
    let repo = TempDir::new().expect("kind-filter definition repo");
    let cache = TempDir::new().expect("kind-filter definition cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"definition-kind-scope","private":true}"#,
    );
    write(
        &repo.path().join("src/function.ts"),
        "export function shared() { return 1; }\n",
    );
    write(
        &repo.path().join("src/class.ts"),
        "export class shared {}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "kind-filter definitions"]);

    let function = run_json(
        repo.path(),
        cache.path(),
        &["where", "shared", "--kind", "function", "--format", "json"],
    );
    let class = run_json(
        repo.path(),
        cache.path(),
        &["where", "shared", "--kind", "class", "--format", "json"],
    );
    assert_schema("schemas/where.schema.json", &function);
    assert_schema("schemas/where.schema.json", &class);
    assert_eq!(function["definitions"][0]["anchor"]["path"], "src/function.ts#shared");
    assert_eq!(class["definitions"][0]["anchor"]["path"], "src/class.ts#shared");
    let function_horizon = horizon(&function["observations"], "definition_matches");
    let class_horizon = horizon(&class["observations"], "definition_matches");
    assert_ne!(function_horizon["scope"], class_horizon["scope"]);
    assert_ne!(
        function_horizon["count"]["certificate_id"],
        class_horizon["count"]["certificate_id"],
        "different filtered fact sets cannot share one coverage certificate"
    );
}

#[test]
fn source_directory_named_coverage_remains_in_the_definition_universe() {
    let repo = TempDir::new().expect("coverage source repo");
    let cache = TempDir::new().expect("coverage source cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"coverage-domain","private":true}"#,
    );
    write(
        &repo.path().join("src/coverage/owner.ts"),
        "export function CoverageDomainNeedle() { return 1; }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "coverage domain source"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["where", "CoverageDomainNeedle", "--format", "json"],
    );
    assert_eq!(json["total_matches"], 1, "{json:#}");
    assert_eq!(
        json["definitions"][0]["anchor"]["path"],
        "src/coverage/owner.ts#CoverageDomainNeedle",
        "a domain name cannot be treated as generated output globally: {json:#}"
    );
}

#[test]
fn oversized_source_remains_as_a_typed_open_coverage_candidate() {
    let repo = TempDir::new().expect("oversized source repo");
    let cache = TempDir::new().expect("oversized source cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"oversized-source","private":true}"#,
    );
    write(
        &repo.path().join("src/target.ts"),
        "export function target() { return 1; }\n",
    );
    let mut huge =
        "import { target } from './target';\nexport function HugeNeedle() { return target(); }\n"
            .to_string();
    huge.push_str(&" ".repeat(900_100));
    write(&repo.path().join("src/huge.ts"), &huge);
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "oversized source"]);

    let missing_definition = run_json(
        repo.path(),
        cache.path(),
        &["where", "HugeNeedle", "--format", "json"],
    );
    let definitions = horizon(&missing_definition["observations"], "definition_matches");
    assert_eq!(definitions["count"]["observed"], 0, "{missing_definition:#}");
    assert_eq!(definitions["count"]["closure"], "open", "{missing_definition:#}");
    assert!(
        definitions["unsupported"]
            .as_array()
            .expect("unsupported definitions")
            .iter()
            .any(|item| item["file"] == "src/huge.ts"),
        "the skipped source path must remain visible: {missing_definition:#}"
    );

    let consumer = run_json(
        repo.path(),
        cache.path(),
        &["where", "target", "--format", "json"],
    );
    let count = &consumer["definitions"][0]["consumers_total"];
    assert_eq!(count["observed"], 0, "{consumer:#}");
    assert_eq!(count["closure"], "open", "{consumer:#}");
}
