#[test]
fn public_json_outputs_do_not_reintroduce_router_or_trust_fields() {
    let (repo, cache) = fixture();
    let cases = [
        &["ls", ".", "--format", "json"][..],
        &[
            "cone",
            "packages/replay/src/session.ts",
            "--format",
            "json",
        ],
        &["changed", "--format", "json"],
        &["impact", "--changed", "--format", "json"],
        &[
            "delete",
            "packages/replay/src/session.ts",
            "--format",
            "json",
        ],
        &[
            "proof",
            "packages/replay/src/session.ts",
            "--format",
            "json",
        ],
    ];
    let forbidden_keys = [
        "read_first",
        "source_of_truth",
        "confidence",
        "score",
        "rank",
        "safe_to_delete",
    ];

    for args in cases {
        let report = run_json(repo.path(), cache.path(), args);
        let mut found = Vec::new();
        collect_forbidden_json_keys(&report, "$", &forbidden_keys, &mut found);
        assert!(
            found.is_empty(),
            "public JSON report {:?} reintroduced legacy router/trust fields: {found:?}\n{report:#}",
            args
        );
    }
}

fn collect_forbidden_json_keys(
    value: &Value,
    path: &str,
    forbidden: &[&str],
    found: &mut Vec<String>,
) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                let child_path = format!("{path}.{key}");
                if forbidden.iter().any(|forbidden| forbidden == key) {
                    found.push(child_path.clone());
                }
                collect_forbidden_json_keys(child, &child_path, forbidden, found);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                collect_forbidden_json_keys(child, &format!("{path}[{index}]"), forbidden, found);
            }
        }
        _ => {}
    }
}
