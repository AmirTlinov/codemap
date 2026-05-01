#[test]
fn swift_package_manifest_surfaces_packages_scripts_and_local_path_edges() {
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
    name: "HostApp",
    dependencies: [
        .package(path: "Packages/Core")
    ],
    targets: [
        .executableTarget(name: "HostApp", dependencies: ["Core"]),
        .testTarget(name: "HostAppTests", dependencies: ["HostApp"])
    ]
)
"#,
    );
    write(
        &repo.path().join("Sources/HostApp/main.swift"),
        r#"import Foundation
import Core

@MainActor
public final class HostViewModel {
    @Published public var title: String = "Host"
    private let cacheKey: String = "host"

    public func refresh() {
        let transientState = title
        _ = transientState
    }
}
"#,
    );
    write(
        &repo.path().join("Sources/HostApp/AppModel.swift"),
        r#"struct HostAppModel {
    let viewModel = HostViewModel()
}
"#,
    );
    write(
        &repo.path().join("Packages/Core/Package.swift"),
        r#"// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "Core",
    targets: [
        .target(name: "Core")
    ]
)
"#,
    );
    write(
        &repo.path().join("Packages/Core/Sources/Core/Core.swift"),
        "public struct Core {}\n",
    );
    write(
        &repo
            .path()
            .join("Tests/HostAppTests/HostViewModelTests.swift"),
        r#"@testable import HostApp
import Testing

@Test
func hostViewModelRefreshes() {
    let model = HostViewModel()
    model.refresh()
    let app = HostAppModel()
    _ = app.viewModel
}
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let status = run_json(repo.path(), cache.path(), &["status", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &status);
    assert_eq!(status["package_manager"], "swift");
    assert!(
        status["scripts"]
            .as_array()
            .expect("scripts")
            .iter()
            .any(|script| script.as_str().unwrap_or_default() == "swift test")
    );

    let ls = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &ls);
    assert!(
        ls["directory"]
            .as_array()
            .expect("directory")
            .iter()
            .any(|surface| surface["kind"] == "package:swift"
                && surface["examples"]
                    .as_array()
                    .expect("examples")
                    .iter()
                    .any(|example| example == "Package.swift")),
        "root map should surface SwiftPM package manifests: {ls:#}"
    );
    assert!(
        ls["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .any(|edge| edge["from"] == "Package.swift"
                && edge["to"] == "Packages/Core/"
                && edge["type"] == "package_internal"
                && edge["evidence"] == "package_manifest:Core"),
        "SwiftPM local path dependencies should become package graph edges: {ls:#}"
    );

    let file = run_json(
        repo.path(),
        cache.path(),
        &["ls", "Sources/HostApp/main.swift", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &file);
    let anchor = &file["anchor"];
    assert!(
        anchor["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .any(|symbol| symbol["name"] == "HostViewModel"
                && symbol["kind"] == "class"
                && symbol["line_start"] == 5
                && symbol["line_end"] == 13),
        "Swift file ls should surface class symbols with ranges: {file:#}"
    );
    assert!(
        anchor["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .any(|symbol| symbol["name"] == "title"
                && symbol["kind"] == "property"
                && symbol["exported"] == true),
        "Swift file ls should surface attributed properties: {file:#}"
    );
    assert!(
        anchor["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .all(|symbol| symbol["name"] != "cacheKey"),
        "Swift file ls should hide low-signal nested private properties by default: {file:#}"
    );
    assert!(
        anchor["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .all(|symbol| symbol["name"] != "transientState"),
        "Swift file ls should hide local constants inside functions by default: {file:#}"
    );
    assert!(
        file["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|group| group["reason"] == "nested symbols hidden by default"),
        "hidden symbol count should explain that deeper member symbols exist: {file:#}"
    );
    let full_file = run_json(
        repo.path(),
        cache.path(),
        &[
            "ls",
            "Sources/HostApp/main.swift",
            "--include-hidden",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/ls.schema.json", &full_file);
    assert!(
        full_file["anchor"]["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .any(|symbol| symbol["name"] == "cacheKey"
                && symbol["kind"] == "constant"
                && symbol["exported"] == false),
        "include-hidden should still expose nested private properties on demand: {full_file:#}"
    );
    assert!(
        full_file["anchor"]["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .any(|symbol| symbol["name"] == "transientState"
                && symbol["kind"] == "constant"
                && symbol["exported"] == false),
        "include-hidden should still expose local constants on demand: {full_file:#}"
    );
    assert!(
        anchor["imports"]
            .as_array()
            .expect("imports")
            .iter()
            .any(|import| import == "Foundation")
            && anchor["imports"]
                .as_array()
                .expect("imports")
                .iter()
                .any(|import| import == "Core"),
        "Swift file ls should surface imported modules: {file:#}"
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "Sources/HostApp/main.swift", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .any(
                |surface| surface["path"] == "Tests/HostAppTests/HostViewModelTests.swift"
                    && surface["evidence"] == "test_symbol_reference"
                    && surface["strength"] == "high"
            ),
        "Swift tests that import the module and reference exported symbols should become structural proof, not fallback: {proof:#}"
    );
    assert!(
        proof["fallback"].as_array().expect("fallback").is_empty(),
        "Swift symbol reference proof should suppress broad fallback: {proof:#}"
    );
    let impact = run_json(
        repo.path(),
        cache.path(),
        &[
            "impact",
            "--files",
            "Sources/HostApp/main.swift",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/impact.schema.json", &impact);
    let impact_proof = impact["clusters"][0]["proof"]
        .as_array()
        .expect("impact proof");
    let host_test_count = impact_proof
        .iter()
        .filter(|edge| edge["from"] == "Tests/HostAppTests/HostViewModelTests.swift")
        .count();
    assert_eq!(
        host_test_count, 1,
        "impact proof candidates should dedupe one test file across changed and consumer seeds: {impact:#}"
    );
    assert!(
        impact_proof.iter().any(|edge| edge["from"]
            == "Tests/HostAppTests/HostViewModelTests.swift"
            && edge["to"] == "Sources/HostApp/main.swift"),
        "impact proof should prefer the direct changed-anchor edge when a test also proves consumers: {impact:#}"
    );
}

