// Responsibility: versioned-ecosystem-support-and-cross-language-pilot
#[test]
fn mixed_monorepo_exposes_release_tiers_and_cross_language_boundaries() {
    let (repo, cache) = fixture_matrix_repo("mixed-monorepo");
    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(doctor["ecosystem_support_version"], 1);
    let support = doctor["ecosystem_support"].as_array().expect("support matrix");
    for (ecosystem, tier) in [
        ("javascript/typescript", "A"),
        ("python", "A"),
        ("rust", "A"),
        ("go", "A"),
        ("swift", "B"),
        ("shell", "C"),
        ("sql", "C"),
        ("yaml/config", "C"),
        ("schema/protocol", "C"),
        ("generated clients", "C"),
    ] {
        let row = ecosystem_row(support, ecosystem);
        assert_eq!(row["tier"], tier, "{ecosystem}: {doctor:#}");
        assert!(row["detected_files"].as_u64().unwrap_or_default() > 0);
        assert_eq!(row["cells"]["inventory"], "verified");
    }
    let tier_a = support.iter().filter(|row| row["tier"] == "A").count();
    assert_eq!(tier_a, 4, "release utility floor must be explicit: {doctor:#}");
    assert!(
        ecosystem_row(support, "swift")["cells"]["runtime"] == "unsupported",
        "Tier B must expose its runtime edge rather than imply completeness: {doctor:#}"
    );
    assert!(
        ecosystem_row(support, "generated clients")["generated_files"]
            .as_u64()
            .unwrap_or_default()
            > 0,
        "generated ownership must be visible: {doctor:#}"
    );

    let contract = run_json(
        repo.path(),
        cache.path(),
        &[
            "contract",
            "contracts/events.openapi.yaml",
            "--all",
            "--format",
            "json",
        ],
    );
    let lineage = contract["lineage"].as_array().expect("contract lineage");
    for kind in ["consumes", "generates", "exports", "verifies_directly"] {
        assert!(
            lineage.iter().any(|edge| edge["type"] == kind),
            "mixed contract lineage must contain {kind}: {contract:#}"
        );
    }
    assert!(
        lineage.iter().any(|edge| {
            edge["type"] == "consumes"
                && edge["from"] == "apps/web/src/events-client.ts"
        }),
        "the generated TypeScript client needs an exact application consumer: {contract:#}"
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "services/audit/audit/main.py", "--format", "json"],
    );
    assert!(
        proof["verification_topology"]["runnable"]
            .as_array()
            .expect("runnable topology")
            .iter()
            .any(|edge| {
                edge["relation"] == "invokes_process"
                    && edge["object"] == "scripts/verify-events.sh"
            }),
        "Python verification must retain its exact shell process boundary: {proof:#}"
    );
}

#[test]
fn unsupported_language_project_keeps_inventory_and_typed_horizon() {
    let repo = TempDir::new().expect("unsupported repo");
    let cache = TempDir::new().expect("unsupported cache");
    git(repo.path(), &["init", "-q"]);
    write(
        &repo.path().join("lib/router.ex"),
        "defmodule Example.Router do\n  def route(path), do: path\nend\n",
    );
    write(
        &repo.path().join("lib/Decoy.java"),
        "class Decoy { String route = \"app.get('/admin', handler)\"; }\n",
    );
    write(
        &repo.path().join("lib/decoy.rb"),
        "ROUTE_EXAMPLE = \"router.post('/charge', handler)\"\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["-c", "user.email=a@example.com", "-c", "user.name=a", "commit", "-qm", "unsupported language"]);

    let root = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert!(
        root["directory"]
            .as_array()
            .expect("root inventory")
            .iter()
            .any(|surface| surface["examples"].as_array().is_some_and(|examples| examples.iter().any(|path| path == "lib/"))),
        "unsupported source still needs a useful bounded inventory: {root:#}"
    );
    let exact = run_json(
        repo.path(),
        cache.path(),
        &["ls", "lib/router.ex", "--format", "json"],
    );
    assert!(
        exact["observations"]["horizons"]
            .as_array()
            .expect("coverage horizons")
            .iter()
            .flat_map(|horizon| {
                horizon["count"]["reasons"]
                    .as_array()
                    .into_iter()
                    .flatten()
            })
            .any(|reason| reason == "unsupported_language"),
        "unknown parser coverage must stay typed: {exact:#}"
    );
    let runtime = run_json(repo.path(), cache.path(), &["runtime", ".", "--format", "json"]);
    assert!(
        runtime["routes"]
            .as_array()
            .expect("runtime routes")
            .is_empty(),
        "unsupported-language decoys must not become runtime facts: {runtime:#}"
    );
    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    let support = doctor["ecosystem_support"].as_array().expect("support matrix");
    let other = ecosystem_row(support, "other source languages");
    assert_eq!(other["tier"], "C");
    assert_eq!(other["detected_files"], 3);
    assert_eq!(other["cells"]["symbols"], "unsupported");
    assert_eq!(other["promise"], "inventory and typed unsupported classification only");
}

#[test]
fn release_manifest_declares_every_support_cell_and_four_flagships() {
    let manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas/manifest.json"))
            .expect("schema manifest"),
    )
    .expect("manifest json");
    assert_eq!(manifest["ecosystem_support_version"], 1);
    let rows = manifest["ecosystem_support"].as_array().expect("support rows");
    assert_eq!(rows.iter().filter(|row| row["tier"] == "A").count(), 4);
    let expected = [
        "inventory",
        "symbols",
        "imports",
        "packages",
        "runtime",
        "contracts",
        "data",
        "verification",
        "dynamic_unknowns",
    ];
    for row in rows {
        let cells = row["cells"].as_object().expect("support cells");
        assert_eq!(cells.len(), expected.len(), "{row:#}");
        for cell in expected {
            assert!(cells.contains_key(cell), "missing {cell}: {row:#}");
        }
        assert!(!row["promise"].as_str().unwrap_or_default().contains("complete"));
    }
}

fn ecosystem_row<'a>(rows: &'a [Value], ecosystem: &str) -> &'a Value {
    rows.iter()
        .find(|row| row["ecosystem"] == ecosystem)
        .unwrap_or_else(|| panic!("missing ecosystem {ecosystem}: {rows:#?}"))
}
