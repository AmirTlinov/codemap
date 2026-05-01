#[test]
fn swift_symbol_reference_proof_requires_imported_target_module() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Package.swift"),
        r#"// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "MultiTarget",
    targets: [
        .target(name: "Foo"),
        .target(name: "Bar"),
        .testTarget(name: "BarTests", dependencies: ["Bar"])
    ]
)
"#,
    );
    write(
        &repo.path().join("Sources/Foo/ViewModel.swift"),
        "public final class FeatureViewModel {}\n",
    );
    write(
        &repo.path().join("Sources/Bar/ViewModel.swift"),
        "public final class FeatureViewModel {}\n",
    );
    write(
        &repo.path().join("Tests/BarTests/ViewModelTests.swift"),
        r#"@testable import Bar
import Testing

@Test
func barFeatureViewModelExists() {
    _ = FeatureViewModel()
}
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "Sources/Foo/ViewModel.swift", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .all(|surface| surface["path"] != "Tests/BarTests/ViewModelTests.swift"),
        "Swift proof must require the test to import the anchor target module, not only share symbol names in one package: {proof:#}"
    );
}


#[test]
fn swift_symbol_reference_xref_scope_includes_package_root_and_target() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    for package in ["A", "B"] {
        write(
            &repo
                .path()
                .join(format!("Packages/{package}/Package.swift")),
            &format!(
                r#"// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "{package}",
    targets: [
        .target(name: "Core")
    ]
)
"#
            ),
        );
    }
    write(
        &repo.path().join("Packages/A/Sources/Core/Model.swift"),
        "public struct SharedModel {}\n",
    );
    write(
        &repo.path().join("Packages/A/Sources/Core/UseModel.swift"),
        "func useSharedModel() { _ = SharedModel() }\n",
    );
    write(
        &repo.path().join("Packages/B/Sources/Core/Other.swift"),
        "func useSharedModel() { _ = SharedModel() }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "Packages/A/Sources/Core/Model.swift",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    let incoming = cone["incoming"].as_array().expect("incoming");
    assert!(
        incoming.iter().any(
            |edge| edge["from"] == "Packages/A/Sources/Core/UseModel.swift"
                && edge["evidence"] == "same_package_symbol_reference"
        ),
        "same package root and target should still produce Swift symbol xref: {cone:#}"
    );
    assert!(
        incoming
            .iter()
            .all(|edge| edge["from"] != "Packages/B/Sources/Core/Other.swift"),
        "Swift symbol xref must not cross nested packages that reuse the same target name: {cone:#}"
    );
}


#[test]
fn swift_symbol_reference_proof_ignores_commented_test_imports() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Package.swift"),
        r#"// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "CommentedImport",
    targets: [
        .target(name: "Foo"),
        .testTarget(name: "FooTests", dependencies: [])
    ]
)
"#,
    );
    write(
        &repo.path().join("Sources/Foo/ViewModel.swift"),
        "public final class FeatureViewModel {}\n",
    );
    write(
        &repo.path().join("Tests/FooTests/ViewModelTests.swift"),
        r#"/*
@testable import Foo
*/
import Testing

@Test
func mentionsFeatureViewModelWithoutImportingFoo() {
    _ = FeatureViewModel.self
}
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let test_file = run_json(
        repo.path(),
        cache.path(),
        &[
            "ls",
            "Tests/FooTests/ViewModelTests.swift",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/ls.schema.json", &test_file);
    let imports = test_file["anchor"]["imports"].as_array().expect("imports");
    assert!(
        imports.iter().all(|import| import != "Foo"),
        "Swift imports inside block comments must not become structural imports: {test_file:#}"
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "Sources/Foo/ViewModel.swift", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .all(|surface| surface["path"] != "Tests/FooTests/ViewModelTests.swift"),
        "commented Swift imports must not unlock high-strength symbol-reference proof: {proof:#}"
    );
}


#[test]
fn swift_symbol_reference_proof_ignores_commented_anchor_symbols() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Package.swift"),
        r#"// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "CommentedSymbol",
    targets: [
        .target(name: "Foo"),
        .testTarget(name: "FooTests", dependencies: ["Foo"])
    ]
)
"#,
    );
    write(
        &repo.path().join("Sources/Foo/Legacy.swift"),
        r#"/*
public final class FeatureViewModel {}
*/
public struct RealThing {}
"#,
    );
    write(
        &repo.path().join("Tests/FooTests/FeatureTests.swift"),
        r#"@testable import Foo
import Testing

@Test
func mentionsRemovedFeatureViewModel() {
    _ = FeatureViewModel.self
}
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let anchor_file = run_json(
        repo.path(),
        cache.path(),
        &["ls", "Sources/Foo/Legacy.swift", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &anchor_file);
    let symbols = anchor_file["anchor"]["symbols"]
        .as_array()
        .expect("symbols");
    assert!(
        symbols
            .iter()
            .all(|symbol| symbol["name"] != "FeatureViewModel"),
        "Swift symbols inside block comments must not become anchor symbols: {anchor_file:#}"
    );
    assert!(
        symbols.iter().any(|symbol| symbol["name"] == "RealThing"),
        "real Swift symbols should still be surfaced after comment stripping: {anchor_file:#}"
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "Sources/Foo/Legacy.swift", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .all(|surface| surface["path"] != "Tests/FooTests/FeatureTests.swift"),
        "commented Swift anchor symbols must not unlock high-strength proof: {proof:#}"
    );
}

