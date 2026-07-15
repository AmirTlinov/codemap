use std::collections::{BTreeMap, BTreeSet};

#[test]
fn public_json_reports_validate_against_manifest_schemas() {
    let (repo, cache) = fixture();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest_text =
        fs::read_to_string(root.join("schemas/manifest.json")).expect("manifest should exist");
    let manifest: Value = serde_json::from_str(&manifest_text).expect("manifest json");
    let manifest_entries = manifest_entries_by_kind(&manifest);

    let cases = [
        PublicJsonReport {
            manifest_kind: "doctor",
            args: &["doctor", "--format", "json"],
            allow_failure: false,
        },
        PublicJsonReport {
            manifest_kind: "status",
            args: &["status", "--format", "json"],
            allow_failure: false,
        },
        PublicJsonReport {
            manifest_kind: "teach",
            args: &["teach", "--format", "json"],
            allow_failure: false,
        },
        PublicJsonReport {
            manifest_kind: "files",
            args: &["files", "--format", "json"],
            allow_failure: false,
        },
        PublicJsonReport {
            manifest_kind: "ls",
            args: &["ls", ".", "--format", "json"],
            allow_failure: false,
        },
        PublicJsonReport {
            manifest_kind: "cone",
            args: &[
                "cone",
                "packages/replay/src/session.ts",
                "--format",
                "json",
            ],
            allow_failure: false,
        },
        PublicJsonReport {
            manifest_kind: "where",
            args: &["where", "seek", "--format", "json"],
            allow_failure: false,
        },
        PublicJsonReport {
            manifest_kind: "graph",
            args: &["graph", "--lens", "causal", "--format", "json"],
            allow_failure: false,
        },
        PublicJsonReport {
            manifest_kind: "runtime",
            args: &["runtime", ".", "--format", "json"],
            allow_failure: false,
        },
        PublicJsonReport {
            manifest_kind: "contract",
            args: &[
                "contract",
                "packages/replay/src/session.ts",
                "--format",
                "json",
            ],
            allow_failure: false,
        },
        PublicJsonReport {
            manifest_kind: "flow",
            args: &[
                "flow",
                "packages/replay/src/session.ts",
                "--format",
                "json",
            ],
            allow_failure: false,
        },
        PublicJsonReport {
            manifest_kind: "boundary-map",
            args: &["boundary-map", ".", "--format", "json"],
            allow_failure: false,
        },
        PublicJsonReport {
            manifest_kind: "siblings",
            args: &["siblings", "packages/replay/src", "--format", "json"],
            allow_failure: false,
        },
        PublicJsonReport {
            manifest_kind: "place",
            args: &[
                "place",
                "packages/replay",
                "--kind",
                "test",
                "--format",
                "json",
            ],
            allow_failure: false,
        },
        PublicJsonReport {
            manifest_kind: "delete",
            args: &[
                "delete",
                "packages/replay/src/session.ts",
                "--format",
                "json",
            ],
            allow_failure: false,
        },
        PublicJsonReport {
            manifest_kind: "changed",
            args: &["changed", "--format", "json"],
            allow_failure: false,
        },
        PublicJsonReport {
            manifest_kind: "diff-map",
            args: &["diff-map", "--changed", "--format", "json"],
            allow_failure: false,
        },
        PublicJsonReport {
            manifest_kind: "impact",
            args: &["impact", "--changed", "--format", "json"],
            allow_failure: false,
        },
        PublicJsonReport {
            manifest_kind: "proof-map",
            args: &["proof-map", ".", "--format", "json"],
            allow_failure: false,
        },
        PublicJsonReport {
            manifest_kind: "proof",
            args: &[
                "proof",
                "packages/replay/src/session.ts",
                "--format",
                "json",
            ],
            allow_failure: false,
        },
        PublicJsonReport {
            manifest_kind: "boundaries",
            args: &["boundaries", "--format", "json"],
            allow_failure: true,
        },
        PublicJsonReport {
            manifest_kind: "anchor-validation",
            args: &["anchors", "validate", "--format", "json"],
            allow_failure: false,
        },
        PublicJsonReport {
            manifest_kind: "teach",
            args: &["teach", "--format", "json"],
            allow_failure: false,
        },
    ];

    let covered = cases
        .iter()
        .map(|case| case.manifest_kind)
        .collect::<BTreeSet<_>>();
    let manifest_structural_outputs = manifest_entries
        .iter()
        .filter(|(_, entry)| entry["contract"] == "structural_output")
        .map(|(kind, _)| *kind)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        covered, manifest_structural_outputs,
        "every structural JSON manifest entry should have a real command validation case"
    );

    for case in cases {
        let entry = manifest_entries
            .get(case.manifest_kind)
            .unwrap_or_else(|| panic!("missing manifest entry for {}", case.manifest_kind));
        let report = run_public_json_report(repo.path(), cache.path(), &case);
        assert_schema(entry["file"].as_str().expect("schema file"), &report);
        assert_eq!(
            report["kind"], entry["json_kind"],
            "{} should report manifest json_kind",
            case.manifest_kind
        );
        assert_eq!(
            report["schema_version"], entry["schema_version"],
            "{} should report manifest schema_version",
            case.manifest_kind
        );
        assert_eq!(
            report["build_identity"]["semver"],
            env!("CARGO_PKG_VERSION"),
            "{} should identify the running codemap build",
            case.manifest_kind
        );
        if matches!(case.manifest_kind, "doctor" | "status") {
            assert_eq!(report["build_identity"]["binary_sha256_state"], "computed");
        } else {
            assert_eq!(
                report["build_identity"]["binary_sha256_state"],
                "not_requested",
                "{} must not hash the binary on an ordinary report",
                case.manifest_kind
            );
            assert_eq!(report["build_identity"]["binary_sha256"], Value::Null);
        }
    }
}

struct PublicJsonReport {
    manifest_kind: &'static str,
    args: &'static [&'static str],
    allow_failure: bool,
}

fn manifest_entries_by_kind(manifest: &Value) -> BTreeMap<&str, &Value> {
    manifest["schemas"]
        .as_array()
        .expect("manifest schemas")
        .iter()
        .map(|entry| {
            (
                entry["kind"].as_str().expect("manifest kind"),
                entry,
            )
        })
        .collect()
}

fn run_public_json_report(repo: &Path, cache: &Path, case: &PublicJsonReport) -> Value {
    let output = codemap()
        .current_dir(repo)
        .env("CODEMAP_CACHE_DIR", cache)
        .args(case.args)
        .output()
        .expect("codemap should run");
    if case.allow_failure {
        assert!(
            !output.stdout.is_empty(),
            "codemap {:?} should still print a JSON report on failure: {}",
            case.args,
            String::from_utf8_lossy(&output.stderr)
        );
    } else {
        assert!(
            output.status.success(),
            "codemap {:?} failed: {}",
            case.args,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "codemap {:?} did not print valid json: {error}\nstdout:\n{}\nstderr:\n{}",
            case.args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}
